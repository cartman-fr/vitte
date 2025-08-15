// editor-plugins/vscode/src/extension.ts
// Extension VS Code pour Vitte : formatage, diagnostics, et commandes utilitaires.
// - Format provider branché sur `vitte-fmt`
// - Diagnostics via `vitc check` (JSON si dispo, fallback texte)
// - Commandes: build / run / test / fmt / check
//
// SPDX-License-Identifier: MIT

import * as vscode from 'vscode';
import { runFmt } from './format';
import { runCheckAndReport } from './diag';
import { runCmdInTerminal } from './utils';

export function activate(context: vscode.ExtensionContext) {
  const diag = vscode.languages.createDiagnosticCollection('vitte');
  context.subscriptions.push(diag);

  // ---------- Formatting provider ----------
  context.subscriptions.push(
    vscode.languages.registerDocumentFormattingEditProvider('vitte', {
      provideDocumentFormattingEdits: async (doc) => {
        const cfg = vscode.workspace.getConfiguration('vitte', doc.uri);
        const fmtPath = cfg.get<string>('fmtPath', 'vitte-fmt');
        const res = await runFmt(fmtPath, doc.getText());
        if (res.ok) {
          const full = new vscode.Range(0, 0, doc.lineCount, 0);
          return [vscode.TextEdit.replace(full, res.text)];
        } else {
          vscode.window.showWarningMessage(`vitte-fmt: ${res.error}`);
          return [];
        }
      }
    })
  );

  // ---------- Format-on-save fallback ----------
  // Si l’utilisateur met vitte.formatOnSave=true, on applique un format à l’enregistrement
  // même si editor.formatOnSave est désactivé. (Si editor.formatOnSave est ON, VS Code
  // appellera déjà notre provider, donc on s’abstient pour éviter les doublons.)
  context.subscriptions.push(
    vscode.workspace.onWillSaveTextDocument(async (e) => {
      const doc = e.document;
      if (doc.languageId !== 'vitte') return;

      const vitteCfg = vscode.workspace.getConfiguration('vitte', doc.uri);
      const wantVitteFormatOnSave = vitteCfg.get<boolean>('formatOnSave', true);

      const editorCfg = vscode.workspace.getConfiguration('editor', doc.uri);
      const editorFormatOnSave = editorCfg.get<boolean>('formatOnSave', false);

      if (!wantVitteFormatOnSave || editorFormatOnSave) return;

      const fmtPath = vitteCfg.get<string>('fmtPath', 'vitte-fmt');
      const res = await runFmt(fmtPath, doc.getText());
      if (!res.ok) {
        vscode.window.showWarningMessage(`vitte-fmt: ${res.error}`);
        return;
      }
      // Applique l’édition avant l’écriture sur disque
      const edit = new vscode.WorkspaceEdit();
      const fullRange = new vscode.Range(0, 0, doc.lineCount, 0);
      edit.replace(doc.uri, fullRange, res.text);
      await vscode.workspace.applyEdit(edit);
    })
  );

  // ---------- Diagnostics ----------
  // Au save : `vitc check`
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument(async (doc) => {
      if (doc.languageId !== 'vitte') return;
      const cfg = vscode.workspace.getConfiguration('vitte', doc.uri);
      if (!cfg.get<boolean>('enableDiagnostics', true)) return;
      const vitc = cfg.get<string>('vitcPath', 'vitc');
      const args = cfg.get<string[]>('checkArgs', ['check']);
      await runCheckAndReport(diag, vitc, args, doc);
    })
  );

  // À l’ouverture d’un fichier Vitte, on peut lancer un check léger (optionnel)
  context.subscriptions.push(
    vscode.workspace.onDidOpenTextDocument(async (doc) => {
      if (doc.languageId !== 'vitte') return;
      const cfg = vscode.workspace.getConfiguration('vitte', doc.uri);
      if (!cfg.get<boolean>('enableDiagnostics', true)) return;
      const vitc = cfg.get<string>('vitcPath', 'vitc');
      const args = cfg.get<string[]>('checkArgs', ['check']);
      await runCheckAndReport(diag, vitc, args, doc);
    })
  );

  // ---------- Commands ----------
  context.subscriptions.push(
    vscode.commands.registerCommand('vitte.check', async () => {
      const doc = vscode.window.activeTextEditor?.document;
      if (!doc || doc.languageId !== 'vitte') return;
      const cfg = vscode.workspace.getConfiguration('vitte', doc.uri);
      const vitc = cfg.get<string>('vitcPath', 'vitc');
      const args = cfg.get<string[]>('checkArgs', ['check']);
      await runCheckAndReport(diag, vitc, args, doc);
    }),

    vscode.commands.registerCommand('vitte.build', () => {
      const editor = vscode.window.activeTextEditor;
      const uri = editor?.document?.uri;
      const cfg = vscode.workspace.getConfiguration('vitte', uri);
      runCmdInTerminal(cfg.get<string>('vitcPath', 'vitc'), cfg.get<string[]>('buildArgs', ['build']));
    }),

    vscode.commands.registerCommand('vitte.run', () => {
      const editor = vscode.window.activeTextEditor;
      const uri = editor?.document?.uri;
      const cfg = vscode.workspace.getConfiguration('vitte', uri);
      runCmdInTerminal(cfg.get<string>('vitcPath', 'vitc'), cfg.get<string[]>('runArgs', ['run']));
    }),

    vscode.commands.registerCommand('vitte.test', () => {
      const editor = vscode.window.activeTextEditor;
      const uri = editor?.document?.uri;
      const cfg = vscode.workspace.getConfiguration('vitte', uri);
      runCmdInTerminal(cfg.get<string>('vitcPath', 'vitc'), cfg.get<string[]>('testArgs', ['test']));
    }),

    vscode.commands.registerCommand('vitte.fmt', async () => {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== 'vitte') return;
      await vscode.commands.executeCommand('editor.action.formatDocument');
    })
  );

  // ---------- Status bar ----------
  const status = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
  status.text = 'Vitte';
  status.tooltip = 'Vitte Language Tools — Check (Ctrl+Alt+C)';
  status.command = 'vitte.check';
  status.show();
  context.subscriptions.push(status);
}

export function deactivate() {
  // VS Code nettoie les subscriptions via context.subscriptions,
  // donc rien de spécial ici.
}
