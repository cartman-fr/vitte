#!/usr/bin/env python3
# compress_bytecode.py — (dé)compresse un fichier VITBC (Vitte ByteCode)
# -----------------------------------------------------------------------------
# - Supporte input VITBC v2 (MAGIC + header + sections [+ CRC + VEND]).
# - Supporte input VITBC v1 (sans trailer/CRC) : convertit en v2 en sortie.
# - Compression: zstd des *sections* uniquement (conforme à vitte-vm/loader.rs).
# - Recalcule le CRC32 IEEE sur le body (version..sections) et écrit le trailer.
#
# Usage :
#   python scripts/compress_bytecode.py in.vitbc out.vitbc            # compresse (→ v2)
#   python scripts/compress_bytecode.py --level 9 in.vitbc out.vitbc  # niveau zstd
#   python scripts/compress_bytecode.py --decompress in.vitbc out.vit # décompresse → v2 non compressé
#   python scripts/compress_bytecode.py --verify file.vitbc           # vérifie CRC/structure
#   python scripts/compress_bytecode.py --show file.vitbc             # affiche header
#
# Exit non-zero au moindre souci.

import argparse
import struct
import sys
import zlib
from typing import Tuple

try:
    import zstandard as zstd
except Exception as e:
    print("✖ Le module 'zstandard' est requis. Installe-le via: pip install zstandard", file=sys.stderr)
    sys.exit(2)

MAGIC = b"VITBC\x00"      # 6
TRAILER_MAGIC = b"VEND\x00\x00"  # 6
FILE_VERSION_V2 = 2
FLAG_COMPRESSED_ZSTD = 0x0000_0001

# ----------------------------- Helpers binaires -----------------------------

def u32(x: int) -> bytes:
    return struct.pack("<I", x)

def i64(x: int) -> bytes:
    return struct.pack("<q", x)

def read_u32(buf: bytes, off: int) -> Tuple[int, int]:
    return struct.unpack_from("<I", buf, off)[0], off + 4

def read_i64(buf: bytes, off: int) -> Tuple[int, int]:
    return struct.unpack_from("<q", buf, off)[0], off + 8

# ----------------------------- Parse VITBC ----------------------------------

class VitbcInfo:
    def __init__(self):
        self.version = None
        self.flags = None
        self.entry_pc = None  # int or None
        self.counts = (0,0,0,0,0)  # n_ints,n_floats,n_strings,n_data,n_code
        self.sections = b""   # bytes (uncompressed)
        self.compressed_in = False
        self.has_trailer = False

def parse_vitbc(data: bytes) -> VitbcInfo:
    if not data.startswith(MAGIC):
        raise ValueError("MAGIC invalide (pas un fichier VITBC)")
    body = data[len(MAGIC):]
    info = VitbcInfo()

    # Détecte trailer (v2)
    has_trailer = len(body) >= 10 and body[-6:] == TRAILER_MAGIC
    info.has_trailer = has_trailer

    if has_trailer:
        # v2 : sépare body et trailer, vérifie CRC
        crc_got = struct.unpack("<I", body[-10:-6])[0]
        body_no_trailer = body[:-10]
        crc_exp = zlib.crc32(body_no_trailer) & 0xFFFFFFFF
        if crc_got != crc_exp:
            raise ValueError(f"CRC invalide (expected=0x{crc_exp:08X}, got=0x{crc_got:08X})")

        # parse header
        off = 0
        version, off = read_u32(body_no_trailer, off)
        if version != FILE_VERSION_V2:
            raise ValueError(f"Version non supportée (got {version}, expected {FILE_VERSION_V2})")
        flags, off = read_u32(body_no_trailer, off)
        entry_raw, off = read_i64(body_no_trailer, off)
        n_ints, off = read_u32(body_no_trailer, off)
        n_floats, off = read_u32(body_no_trailer, off)
        n_strings, off = read_u32(body_no_trailer, off)
        n_data, off = read_u32(body_no_trailer, off)
        n_code, off = read_u32(body_no_trailer, off)

        sections_bytes = body_no_trailer[off:]
        info.version = version
        info.flags = flags
        info.entry_pc = None if entry_raw < 0 else int(entry_raw)
        info.counts = (n_ints, n_floats, n_strings, n_data, n_code)
        info.compressed_in = bool(flags & FLAG_COMPRESSED_ZSTD)

        # sections (dé)compression
        if info.compressed_in:
            info.sections = zstd.ZstdDecompressor().decompress(sections_bytes)
        else:
            info.sections = sections_bytes
        return info

    # v1 (sans trailer/CRC) : on parse de la même façon mais on traite le reste comme sections
    off = 0
    version, off = read_u32(body, off)
    if version not in (1, FILE_VERSION_V2):
        raise ValueError(f"Version inconnue (got {version})")
    flags, off = read_u32(body, off)
    entry_raw, off = read_i64(body, off)
    n_ints, off = read_u32(body, off)
    n_floats, off = read_u32(body, off)
    n_strings, off = read_u32(body, off)
    n_data, off = read_u32(body, off)
    n_code, off = read_u32(body, off)

    sections_bytes = body[off:]
    info.version = version
    info.flags = flags
    info.entry_pc = None if entry_raw < 0 else int(entry_raw)
    info.counts = (n_ints, n_floats, n_strings, n_data, n_code)
    info.compressed_in = bool(flags & FLAG_COMPRESSED_ZSTD)
    # En v1 on considère qu'il n'y avait pas de compression (ancien format)
    if info.compressed_in:
        # Très improbable en v1, mais on prévoit au cas où :
        info.sections = zstd.ZstdDecompressor().decompress(sections_bytes)
    else:
        info.sections = sections_bytes
    return info

# ----------------------------- Rebuild VITBC v2 -----------------------------

def write_v2(info: VitbcInfo, out_path: str, compress: bool, level: int) -> None:
    # (Ré)encode en V2 avec trailer & CRC, compression zstd optionnelle sur SECTIONS
    flags = info.flags or 0
    if compress:
        flags |= FLAG_COMPRESSED_ZSTD
        cctx = zstd.ZstdCompressor(level=level)
        sect_out = cctx.compress(info.sections)
    else:
        flags &= ~FLAG_COMPRESSED_ZSTD
        sect_out = info.sections

    entry_raw = -1 if info.entry_pc is None else int(info.entry_pc)

    # Header v2
    body = bytearray()
    body += u32(FILE_VERSION_V2)
    body += u32(flags)
    body += i64(entry_raw)
    n_ints, n_floats, n_strings, n_data, n_code = info.counts
    body += u32(n_ints)
    body += u32(n_floats)
    body += u32(n_strings)
    body += u32(n_data)
    body += u32(n_code)
    # Sections (peu importe leur contenu : on ne les parse pas ici)
    body += sect_out
    # CRC (sur body)
    crc = zlib.crc32(body) & 0xFFFFFFFF
    body += u32(crc)
    body += TRAILER_MAGIC

    with open(out_path, "wb") as f:
        f.write(MAGIC)
        f.write(body)

# ----------------------------- CLI -----------------------------------------

def main():
    ap = argparse.ArgumentParser(description="(Dé)compresse un fichier VITBC (Vitte ByteCode) conforme au loader v2.")
    ap.add_argument("inp", help="Fichier d'entrée (.vitbc)")
    ap.add_argument("out", nargs="?", help="Fichier de sortie (.vitbc) — requis sauf --verify/--show")
    g = ap.add_mutually_exclusive_group()
    g.add_argument("--decompress", action="store_true", help="Sortie non compressée (flag zstd désactivé)")
    g.add_argument("--compress", action="store_true", help="Force compression (défaut si out fourni)")
    ap.add_argument("--level", type=int, default=10, help="Niveau zstd (par défaut: 10)")
    ap.add_argument("--verify", action="store_true", help="Vérifie uniquement l'intégrité (CRC v2) et la structure")
    ap.add_argument("--show", action="store_true", help="Affiche le header et quitte")
    args = ap.parse_args()

    with open(args.inp, "rb") as f:
        data = f.read()

    try:
        info = parse_vitbc(data)
    except Exception as e:
        print(f"✖ Fichier invalide: {e}", file=sys.stderr)
        sys.exit(1)

    if args.verify:
        if info.has_trailer:
            print("✅ VITBC v2 OK — CRC valide.")
        else:
            print("✅ VITBC v1 (sans trailer) — pas de CRC à vérifier.")
        return

    if args.show:
        n_ints, n_floats, n_strings, n_data, n_code = info.counts
        print("VITBC header :")
        print(f"  version        : {info.version} ({'v2' if info.has_trailer else 'v1'})")
        print(f"  compressed(zstd): {bool(info.flags & FLAG_COMPRESSED_ZSTD)}")
        print(f"  entry_pc       : {info.entry_pc}")
        print(f"  counts         : ints={n_ints}, floats={n_floats}, strings={n_strings}, data={n_data}, code={n_code}")
        print(f"  sections bytes : {len(info.sections)} (décompressées)")
        return

    if not args.out:
        print("✖ Spécifie un fichier de sortie (ou utilise --verify/--show).", file=sys.stderr)
        sys.exit(2)

    # Opération par défaut : compresser si --decompress n'est pas donné
    want_compress = args.compress or (not args.decompress)

    try:
        write_v2(info, args.out, compress=want_compress, level=args.level)
    except Exception as e:
        print(f"✖ Échec écriture: {e}", file=sys.stderr)
        sys.exit(1)

    mode = "compressé" if want_compress else "décompressé"
    print(f"✅ Écrit '{args.out}' ({mode}, v2, CRC ok)")

if __name__ == "__main__":
    main()
