// tests/bc_tests.rs
use std::fs;
use std::io::Read;
use std::path::PathBuf;

#[cfg(feature = "checksum")]
use blake3;

const MAGIC: &[u8; 4] = b"VBC\x01";
const FLAG_CHECKSUM: u8 = 0b0000_0001;
const FLAG_COMPRESS: u8 = 0b0000_0010;

fn read_header(path: &PathBuf) -> (u8, u64) {
    let mut f = fs::File::open(path).unwrap();
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic).unwrap();
    assert_eq!(&magic, MAGIC);

    let mut flag = [0u8; 1];
    f.read_exact(&mut flag).unwrap();

    let mut lenbuf = [0u8; 8];
    f.read_exact(&mut lenbuf).unwrap();
    (flag[0], u64::from_le_bytes(lenbuf))
}

#[test]
fn writes_vbc_file_with_magic() {
    // Arrange
    let tmpdir = tempfile::tempdir().unwrap();
    let src = tmpdir.path().join("a.vit");
    fs::write(&src, "print(42)\n").unwrap();

    // Fake minimal project layout calling the real compile function
    // Ici on suppose commands::bc::compile est public.
    let out = tmpdir.path().join("a.vbc");

    // Act
    vitte_lang::commands::bc::compile(&src, Some(&out)).unwrap();

    // Assert
    let (flags, _olen) = read_header(&out);
    // flags dépendent des features
    #[cfg(all(not(feature = "checksum"), not(feature = "compress")))]
    assert_eq!(flags & (FLAG_CHECKSUM | FLAG_COMPRESS), 0);

    #[cfg(feature = "checksum")]
    assert_eq!(flags & FLAG_CHECKSUM, FLAG_CHECKSUM);

    #[cfg(feature = "compress")]
    assert_eq!(flags & FLAG_COMPRESS, FLAG_COMPRESS);

    let bytes = fs::read(&out).unwrap();
    assert!(bytes.len() > 16); // header + payload
}

#[test]
fn writes_to_stdout_dash() {
    let tmpdir = tempfile::tempdir().unwrap();
    let src = tmpdir.path().join("b.vit");
    fs::write(&src, "foo()\n").unwrap();

    // Capture stdout via pipe : ici on simule en écrivant dans un fichier via "-" en redirigeant la sortie du process
    // Si compile est appelé in-process, on ne peut pas capter stdout facilement sans util.
    // On teste surtout que ça ne panique pas.
    vitte_lang::commands::bc::compile(&src, Some(&PathBuf::from("-"))).unwrap();
}

#[cfg(feature = "checksum")]
#[test]
fn checksum_trailing_bytes_present() {
    let tmpdir = tempfile::tempdir().unwrap();
    let src = tmpdir.path().join("c.vit");
    fs::write(&src, "bar()\n").unwrap();
    let out = tmpdir.path().join("c.vbc");

    vitte_lang::commands::bc::compile(&src, Some(&out)).unwrap();
    let bytes = fs::read(&out).unwrap();
    // 32 derniers octets = BLAKE3
    assert!(bytes.len() > 32);
    let checksum = &bytes[bytes.len()-32..];
    assert_eq!(checksum.len(), 32);
}
