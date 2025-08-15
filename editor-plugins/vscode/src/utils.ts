// editor-plugins/vscode/src/utils.ts
// Utilitaires communs pour l’extension Vitte.
// - Terminal unique "Vitte" pour exécuter vitc / vitte-fmt / etc.
// - Détection (best-effort) du dossier projet pour `cd` avant commande
// - Quoting d’arguments portable
//
// SPDX-License-Identifier: MIT

import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

const TERM_NAME = 'Vitte';

/**
 * Lance une commande dans le terminal intégré "Vitte".
 * - Si possible, se place d’abord dans le dossier projet (vitte.toml/.git/…).
 * - Quote proprement les arguments.
 */
export function runCmdInTerminal(cmd: string, args: string[] = [], uri?: vscode.Uri) {
  const term =
    vscode.window.terminals.find(t => t.name === TERM_NAME) ||
    vscode.window.createTerminal(TERM_NAME);

  const cwd = findProjectRoot(uri) || pickWorkspaceFolderFor(uri);
  term.show(true);

  if (cwd) {
    term.sendText(cdCommand(cwd));
  }

  const line = [cmd, ...args].map(quoteArg).join(' ');
  term.sendText(line);
}

/** Construit une commande `cd` compatible des shells courants. */
function cdCommand(dir: string): string {
  if (process.platform === 'win32') {
    // `cd /d` fonctionne dans cmd et PowerShell (change aussi de lecteur).
    return `cd /d "${dir}"`;
  }
  return `cd "${dir}"`;
}

/** Quote minimaliste mais robuste pour terminal intégré. */
export function quoteArg(a: string): string {
  if (a === '') return '""';
  // Si contient espace/quote/backslash/etc → on quote et on échappe " \ $ `
  if (/[\s"'\\$`]/.test(a)) {
    return `"${a.replace(/(["\\$`])/g, '\\$1')}"`;
  }
  return a;
}

/**
 * Renvoie le dossier de l’espace de travail pertinent (si présent),
 * sinon le premier workspace ouvert.
 */
export function pickWorkspaceFolderFor(uri?: vscode.Uri): string | undefined {
  const folder = uri
    ? vscode.workspace.getWorkspaceFolder(uri)
    : vscode.window.activeTextEditor
      ? vscode.workspace.getWorkspaceFolder(vscode.window.activeTextEditor.document.uri)
      : vscode.workspace.workspaceFolders?.[0];
  return folder?.uri.fsPath;
}

/**
 * Tente d’identifier la racine du projet en remontant depuis le fichier actif
 * (ou l’URI fourni) et en cherchant des marqueurs connus.
 */
export function findProjectRoot(uri?: vscode.Uri): string | undefined {
  const activePath =
    uri?.fsPath ?? vscode.window.activeTextEditor?.document.uri.fsPath;
  if (!activePath) return undefined;

  let dir: string;
  try {
    const st = fs.statSync(activePath);
    dir = st.isDirectory() ? activePath : path.dirname(activePath);
  } catch {
    dir = path.dirname(activePath);
  }

  const markers = [
    'vitte.toml',
    'vitte.json',
    'Cargo.toml',
    'package.json',
    '.git',
  ];

  // Remonte au maximum ~50 niveaux (sécurité)
  for (let i = 0; i < 50; i++) {
    try {
      for (const m of markers) {
        if (fs.existsSync(path.join(dir, m))) {
          return dir;
        }
      }
    } catch {
      // ignore erreurs d’accès ponctuelles
    }
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return undefined;
}

/** Affiche une erreur utilisateur (confort). */
export function showError(err: unknown, userMsg?: string) {
  const msg =
    (userMsg ? userMsg + ' — ' : '') +
    (err instanceof Error ? err.message : String(err));
  vscode.window.showErrorMessage(msg);
}

/** Affiche une info utilisateur (confort). */
export function showInfo(msg: string) {
  vscode.window.showInformationMessage(msg);
}
