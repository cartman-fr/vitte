// editor-plugins/vscode/src/format.ts
// Formateur Vitte — appelle le binaire `vitte-fmt`.
// Stratégie:
//  1) Essai avec `--stdin` (préféré, formatte depuis STDIN → STDOUT)
//  2) Fallback avec arg `-` (beaucoup d’outils l’acceptent)
//  3) Fallback pipe simple (sans arg) si les deux au-dessus échouent
//
// Notes:
//  - On ne modifie pas les EOL: on renvoie ce que le formateur retourne.
//  - Si la sortie est vide mais code=0, on retourne le texte d’entrée (changed=false).
//  - Messages d’erreur consolidés pour une UX claire dans l’extension.
//
// SPDX-License-Identifier: MIT

import { spawn } from 'child_process';

type FmtSuccess = { ok: true; text: string; changed: boolean; engine: 'stdin' | 'dash' | 'pipe' };
type FmtFailure = { ok: false; error: string };

/**
 * Exécute le formateur avec plusieurs stratégies de secours.
 * Signature stable (utilisée par l’extension): (bin, input) -> { ok, text|error }
 */
export async function runFmt(bin: string, input: string): Promise<FmtSuccess | FmtFailure> {
  // 1) --stdin
  const a = await fmtOnce(bin, ['--stdin'], input);
  if (a.kind === 'ok') return { ok: true, text: pickText(a.text, input), changed: a.text !== '', engine: 'stdin' };

  // 2) tiret seul (stdin implicite)
  const b = await fmtOnce(bin, ['-'], input);
  if (b.kind === 'ok') return { ok: true, text: pickText(b.text, input), changed: b.text !== '', engine: 'dash' };

  // 3) pipe sans arguments
  const c = await fmtOnce(bin, [], input);
  if (c.kind === 'ok') return { ok: true, text: pickText(c.text, input), changed: c.text !== '', engine: 'pipe' };

  // Échec: fusionne les erreurs (la plus parlante en premier)
  const err = [a.err, b.err, c.err].filter(Boolean).join('\n').trim();
  return { ok: false, error: err || `vitte-fmt failed (bin="${bin}")` };
}

/* ------------------------------------------------------- */
/* Helpers                                                 */
/* ------------------------------------------------------- */

function pickText(out: string, fallback: string): string {
  // Certains formatteurs renvoient rien si pas de changement.
  // Dans ce cas, on renvoie le texte original pour éviter d’effacer le buffer.
  return out.length ? out : fallback;
}

async function fmtOnce(
  bin: string,
  args: string[],
  input: string,
): Promise<{ kind: 'ok'; text: string } | { kind: 'err'; err: string }> {
  try {
    const { stdout, stderr, code, error, signal } = await spawnCollect(bin, args, input, 20_000);
    if (error) {
      // binaire introuvable, permission, etc.
      return { kind: 'err', err: friendlyProcError(bin, error) };
    }
    if (code === 0) {
      // succès: retourne stdout (peut être vide s’il n’y a pas de diff)
      return { kind: 'ok', text: stdout };
    }
    // échec: compose un message
    const msg = [
      `vitte-fmt exit ${code}${signal ? ` (signal ${signal})` : ''} with args: ${shellishArgs(args)}`,
      stderr || stdout
    ].filter(Boolean).join('\n');
    return { kind: 'err', err: msg };
  } catch (e: any) {
    return { kind: 'err', err: friendlyProcError(bin, e) };
  }
}

function shellishArgs(args: string[]): string {
  return args.map(q).join(' ');
}

function q(a: string): string {
  return /[\s"'\\]/.test(a) ? `"${a.replace(/(["\\$`])/g, '\\$1')}"` : a;
}

/**
 * Lance un process avec input sur stdin et récupère stdout/stderr.
 * Timeout pour éviter les blocages (kill au bout de `timeoutMs`).
 */
function spawnCollect(
  bin: string,
  args: string[],
  input: string,
  timeoutMs = 15000
): Promise<{ stdout: string; stderr: string; code: number | null; error?: Error; signal?: NodeJS.Signals }> {
  return new Promise((resolve) => {
    const proc = spawn(bin, args, { stdio: ['pipe', 'pipe', 'pipe'], shell: process.platform === 'win32' });
    let stdout = '';
    let stderr = '';
    let done = false;
    let to: NodeJS.Timeout | undefined;

    const settle = (payload: { stdout: string; stderr: string; code: number | null; error?: Error; signal?: NodeJS.Signals }) => {
      if (done) return;
      done = true;
      if (to) clearTimeout(to);
      resolve(payload);
    };

    // Timeout
    to = setTimeout(() => {
      try { proc.kill('SIGKILL'); } catch {}
      settle({ stdout, stderr: stderr || 'vitte-fmt: timeout exceeded', code: null, signal: 'SIGKILL' });
    }, timeoutMs);

    proc.stdout.on('data', (d) => (stdout += d.toString()));
    proc.stderr.on('data', (d) => (stderr += d.toString()));
    proc.on('error', (error) => settle({ stdout, stderr, code: null, error }));
    proc.on('close', (code, signal) => settle({ stdout, stderr, code, signal: signal || undefined }));

    // Push input then close
    try {
      proc.stdin.end(input);
    } catch (e: any) {
      // EPIPE — on laisse le process se fermer et on récupère l’erreur via 'close'
    }
  });
}

function friendlyProcError(bin: string, e: Error): string {
  const msg = (e && e.message) ? e.message : String(e);
  // Messages communs à rendre plus explicites
  if (/ENOENT/i.test(msg)) {
    return `vitte-fmt introuvable: "${bin}". Ajoute-le à ton PATH ou configure "vitte.fmtPath".`;
  }
  if (/EACCES|EPERM/i.test(msg)) {
    return `vitte-fmt non exécutable: "${bin}". Vérifie les permissions.`;
  }
  return `vitte-fmt erreur: ${msg}`;
}
