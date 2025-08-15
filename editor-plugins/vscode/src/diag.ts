// editor-plugins/vscode/src/diag.ts
// Diagnostics Vitte — collecte via `vitc check`, parse JSON si dispo, fallback regex texte.
// - Regroupe par fichier, applique les ranges corrects, nettoie les diagnostics obsolètes
// - Résout chemins relatifs/absolus, gère Windows/Unix
// - Tolère différents formats de sortie (JSON/texte style gcc/clang)
//
// SPDX-License-Identifier: MIT

import * as vscode from 'vscode';
import { spawn } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

// -------------------------------
// Types internes
// -------------------------------

type Sev = 'error' | 'warning' | 'note' | 'info';

interface VitcRangePos {
  line: number;   // 1-based dans la sortie cli
  column: number; // 1-based
}

interface VitcRange {
  start: VitcRangePos;
  end?: VitcRangePos; // optionnel si l'outil ne donne que le point de départ
}

interface VitcDiag {
  file: string;
  message: string;
  severity: Sev;
  code?: string | number;
  range?: VitcRange;
}

// -------------------------------
// API exposée
// -------------------------------

/**
 * Lance `vitc check` (en JSON si possible) puis reporte les diagnostics.
 * @param collection DiagnosticCollection fourni par l’extension.
 * @param vitcBin Chemin du binaire `vitc`.
 * @param userArgs Arguments configurés (ex: ["check"]).
 * @param doc Document courant (sert de fallback pour le nettoyage).
 */
export async function runCheckAndReport(
  collection: vscode.DiagnosticCollection,
  vitcBin: string,
  userArgs: string[],
  doc: vscode.TextDocument
) {
  const cwd = vscode.workspace.getWorkspaceFolder(doc.uri)?.uri.fsPath
    || vscode.workspace.rootPath
    || path.dirname(doc.uri.fsPath);

  // Essaye d’abord en JSON, sinon fallback en texte.
  const { args: jsonArgs, addedFormat } = buildArgsForJson(userArgs);
  const { stdout: jStdout, stderr: jStderr, error: jErr } =
    await spawnCollect(vitcBin, jsonArgs, cwd);

  let byFile = new Map<string, vscode.Diagnostic[]>();

  if (!jErr && isLikelyJson(jStdout)) {
    const parsed = parseVitcJson(jStdout, cwd);
    byFile = groupToVsDiagnostics(parsed);
  } else {
    // Fallback : on relance *sans* --format=json si besoin (si on avait modifié les args),
    // sinon on réutilise la sortie déjà récupérée si elle n’était pas JSON mais texte utile.
    let tStdout = jStdout;
    let tStderr = jStderr;

    if (addedFormat && !jErr) {
      const { stdout, stderr } = await spawnCollect(vitcBin, userArgs, cwd);
      tStdout = stdout;
      tStderr = stderr;
    }
    const parsed = parseTextDiagnostics([tStdout, tStderr].filter(Boolean).join('\n'), cwd);
    byFile = groupToVsDiagnostics(parsed);
  }

  // Maj collection (clear + set par fichier)
  collection.clear();

  // S’il n’y a aucun diag, on nettoie le doc courant pour bien vider la Vue "Problems".
  if (byFile.size === 0) {
    collection.set(doc.uri, []);
    return;
  }

  for (const [filePath, diags] of byFile.entries()) {
    collection.set(vscode.Uri.file(filePath), diags);
  }
}

// -------------------------------
// Helpers — exécution & parsing
// -------------------------------

/** Exécute un process et collecte stdout/ stderr. */
function spawnCollect(bin: string, args: string[], cwd?: string): Promise<{
  stdout: string; stderr: string; code: number | null; error?: Error;
}> {
  return new Promise((resolve) => {
    const proc = spawn(bin, args, { cwd, shell: process.platform === 'win32' });
    let stdout = '';
    let stderr = '';
    let settled = false;

    proc.stdout.on('data', (d) => (stdout += d.toString()));
    proc.stderr.on('data', (d) => (stderr += d.toString()));
    proc.on('error', (error) => {
      if (!settled) {
        settled = true;
        resolve({ stdout, stderr, code: null, error });
      }
    });
    proc.on('close', (code) => {
      if (!settled) {
        settled = true;
        resolve({ stdout, stderr, code });
      }
    });
  });
}

/** Injecte `--format=json` si absent, sans casser les args utilisateur. */
function buildArgsForJson(args: string[]): { args: string[]; addedFormat: boolean } {
  const hasFormat = args.some(a => a === '--format' || a.startsWith('--format='));
  if (hasFormat) return { args, addedFormat: false };
  // Forme compacte pour limiter les surprises des parsers d’arguments.
  return { args: [...args, '--format=json'], addedFormat: true };
}

function isLikelyJson(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed) return false;
  // Accept either an array or an object with diagnostics array
  return (trimmed.startsWith('{') || trimmed.startsWith('['));
}

/** Parse format JSON “idéal” (souple) du compilateur. */
function parseVitcJson(stdout: string, cwd: string): VitcDiag[] {
  try {
    const data = JSON.parse(stdout);

    // Accept:
    // 1) { diagnostics: [...] }
    // 2) [ ... ] directly
    const list = Array.isArray(data) ? data : (Array.isArray(data.diagnostics) ? data.diagnostics : []);
    const out: VitcDiag[] = [];

    for (const d of list) {
      const file = typeof d.file === 'string' ? resolvePathSafe(cwd, d.file) : undefined;
      if (!file) continue;

      const msg = String(d.message ?? d.msg ?? '');
      const severity: Sev = normalizeSeverity(d.severity);
      const code = (d.code != null) ? d.code : undefined;

      let range: VitcRange | undefined;
      if (d.range && d.range.start) {
        range = {
          start: {
            line: toIntSafe(d.range.start.line, 1),
            column: toIntSafe(d.range.start.column, 1),
          },
          end: d.range.end ? {
            line: toIntSafe(d.range.end.line, d.range.start.line ?? 1),
            column: toIntSafe(d.range.end.column, d.range.start.column ?? 1),
          } : undefined
        };
      } else if (d.line || d.column) {
        range = {
          start: { line: toIntSafe(d.line, 1), column: toIntSafe(d.column, 1) }
        };
      }

      out.push({ file, message: msg, severity, code, range });
    }

    return out;
  } catch {
    // Pas du JSON exploitable → on renvoie vide (le fallback texte prendra la main)
    return [];
  }
}

/** Parse texte style gcc/clang: `path:line:col: error: message` (+ variantes). */
function parseTextDiagnostics(text: string, cwd: string): VitcDiag[] {
  const out: VitcDiag[] = [];
  const lines = text.split(/\r?\n/);

  // Patterns tolérants
  const RE_1 = /^(.+?):(\d+):(\d+):\s*(error|warning|note|info)\s*:?\s*(.+)$/i;
  const RE_2 = /^(.+?):(\d+):\s*(error|warning|note|info)\s*:?\s*(.+)$/i; // sans colonne
  // Exemple multi-lignes code-frame : on ignore tant qu’on n’a pas une nouvelle ligne “fichier:”
  // On tente aussi de capturer des “at file:line:col” (moins prioritaire)
  const RE_AT = /at\s+(.+?):(\d+):(\d+)/i;

  for (let i = 0; i < lines.length; i++) {
    const ln = lines[i];

    let m = RE_1.exec(ln);
    if (m) {
      const file = resolvePathSafe(cwd, m[1]);
      const line = toIntSafe(m[2], 1);
      const col = toIntSafe(m[3], 1);
      const sev = normalizeSeverity(m[4]);
      const msg = m[5].trim();

      out.push({
        file,
        message: msg,
        severity: sev,
        range: { start: { line, column: col } }
      });
      continue;
    }

    m = RE_2.exec(ln);
    if (m) {
      const file = resolvePathSafe(cwd, m[1]);
      const line = toIntSafe(m[2], 1);
      const sev = normalizeSeverity(m[3]);
      const msg = m[4].trim();

      out.push({
        file,
        message: msg,
        severity: sev,
        range: { start: { line, column: 1 } }
      });
      continue;
    }

    // “at path:line:col …” en fin de message (rare)
    const at = RE_AT.exec(ln);
    if (at) {
      const file = resolvePathSafe(cwd, at[1]);
      const line = toIntSafe(at[2], 1);
      const col = toIntSafe(at[3], 1);
      out.push({
        file,
        message: ln.replace(RE_AT, '').trim(),
        severity: 'note',
        range: { start: { line, column: col } }
      });
      continue;
    }

    // Pas de match → on ignore (bruit, code frames, etc.)
  }

  return out;
}

/** Construit la map fichier → Diagnostic[] pour VS Code. */
function groupToVsDiagnostics(list: VitcDiag[]): Map<string, vscode.Diagnostic[]> {
  const map = new Map<string, vscode.Diagnostic[]>();

  for (const d of list) {
    // Guard file path
    if (!d.file) continue;

    const uriFile = normalizeFsCase(d.file);
    const arr = map.get(uriFile) || [];

    const vsRange = toVsRange(d.range);
    const severity = toVsSeverity(d.severity);
    const diag = new vscode.Diagnostic(vsRange, d.message, severity);
    diag.source = 'vitc';

    if (d.code !== undefined) {
      diag.code = String(d.code);
    }

    arr.push(diag);
    map.set(uriFile, arr);
  }

  return map;
}

// -------------------------------
/* Conversions utilitaires */
// -------------------------------

function toVsSeverity(sev: Sev): vscode.DiagnosticSeverity {
  switch (sev) {
    case 'error': return vscode.DiagnosticSeverity.Error;
    case 'warning': return vscode.DiagnosticSeverity.Warning;
    case 'note': return vscode.DiagnosticSeverity.Hint;
    case 'info': return vscode.DiagnosticSeverity.Information;
    default: return vscode.DiagnosticSeverity.Information;
  }
}

function normalizeSeverity(x: any): Sev {
  const s = String(x ?? '').toLowerCase();
  if (s === 'error' || s === 'warning' || s === 'note' || s === 'info') return s as Sev;
  // Fallback: map inconnu à 'info'
  return 'info';
}

/** Convertit la range CLI (1-based) en Range VS Code (0-based). */
function toVsRange(r?: VitcRange): vscode.Range {
  if (!r || !r.start) {
    // Range vide: début de fichier
    return new vscode.Range(0, 0, 0, 1);
  }
  const sl = Math.max(0, (r.start.line ?? 1) - 1);
  const sc = Math.max(0, (r.start.column ?? 1) - 1);

  if (r.end && r.end.line && r.end.column) {
    const el = Math.max(0, r.end.line - 1);
    const ec = Math.max(0, r.end.column - 1);
    return new vscode.Range(sl, sc, el, ec);
  }

  // Sinon : point
  return new vscode.Range(sl, sc, sl, sc + 1);
}

function toIntSafe(v: any, def: number): number {
  const n = Number.parseInt(String(v), 10);
  return Number.isFinite(n) ? n : def;
}

/** Résout un chemin relatif par rapport au cwd, normalise, et vérifie la plausibilité. */
function resolvePathSafe(cwd: string, pth: string): string {
  let candidate = pth;

  // Enlève "file://" éventuel
  if (candidate.startsWith('file://')) {
    try {
      candidate = decodeURIComponent(candidate.replace(/^file:\/\//, ''));
      // Sous Windows, file://C:/...
      if (process.platform === 'win32' && /^[A-Za-z]:/.test(candidate) === false) {
        // déjà bon
      }
    } catch { /* ignore */ }
  }

  if (!path.isAbsolute(candidate)) {
    candidate = path.resolve(cwd, candidate);
  }

  // Normalise séparateurs et casse (Windows)
  candidate = normalizeFsCase(candidate);

  // Pas besoin d’exiger l’existence sur disque pour afficher un diag,
  // mais ça aide à éviter des collisions d’URI incorrects.
  // On laisse passer même si inexistant (ex: fichier temporaire).
  return candidate;
}

/** Normalise la casse des chemins sous Windows (best-effort). */
function normalizeFsCase(pth: string): string {
  if (process.platform !== 'win32') return path.normalize(pth);
  // Best-effort: retourne tel quel si inaccessible, sinon canonicalise lettre de lecteur.
  try {
    const stat = fs.statSync(pth);
    if (stat.isFile() || stat.isDirectory()) {
      // Uppercase drive letter
      return path.normalize(pth.replace(/^([a-z]):/, (_, d) => `${d.toUpperCase()}:`));
    }
  } catch { /* ignore */ }
  return path.normalize(pth.replace(/^([a-z]):/, (_, d) => `${d.toUpperCase()}:`));
}
