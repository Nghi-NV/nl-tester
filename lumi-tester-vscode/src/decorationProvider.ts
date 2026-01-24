import * as vscode from 'vscode';

export interface CommandStatus {
  index: number;
  status: 'pending' | 'running' | 'passed' | 'failed';
  message?: string;
  duration?: number;
}

export interface TestStatus {
  filePath: string;
  commandStatuses: CommandStatus[];
}

export class LumiDecorationProvider {
  private pendingDecorationType: vscode.TextEditorDecorationType;
  private runningDecorationType: vscode.TextEditorDecorationType;
  private passedDecorationType: vscode.TextEditorDecorationType;
  private failedDecorationType: vscode.TextEditorDecorationType;

  constructor() {
    this.pendingDecorationType = vscode.window.createTextEditorDecorationType({
      gutterIconPath: this.getIconPath('pending'),
      gutterIconSize: 'contain',
      before: {
        contentText: '⚪',
        margin: '0 4px 0 0'
      }
    });

    this.runningDecorationType = vscode.window.createTextEditorDecorationType({
      gutterIconPath: this.getIconPath('running'),
      gutterIconSize: 'contain',
      before: {
        contentText: '⏳',
        margin: '0 4px 0 0'
      }
    });

    this.passedDecorationType = vscode.window.createTextEditorDecorationType({
      gutterIconPath: this.getIconPath('passed'),
      gutterIconSize: 'contain',
      before: {
        contentText: '✅',
        margin: '0 4px 0 0'
      }
    });

    this.failedDecorationType = vscode.window.createTextEditorDecorationType({
      gutterIconPath: this.getIconPath('failed'),
      gutterIconSize: 'contain',
      before: {
        contentText: '❌',
        margin: '0 4px 0 0'
      }
    });
  }

  private getIconPath(status: string): vscode.Uri {
    // Return placeholder - icons would be in resources folder
    return vscode.Uri.file('');
  }

  public updateDecorations(status: TestStatus): void {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.uri.fsPath !== status.filePath) {
      return;
    }

    const pendingRanges: vscode.DecorationOptions[] = [];
    const runningRanges: vscode.DecorationOptions[] = [];
    const passedRanges: vscode.DecorationOptions[] = [];
    const failedRanges: vscode.DecorationOptions[] = [];

    // Find the --- separator line (same logic as codeLensProvider)
    let separatorLine = -1;
    for (let i = 0; i < editor.document.lineCount; i++) {
      const line = editor.document.lineAt(i);
      if (line.text.trim() === '---') {
        separatorLine = i;
        break;
      }
    }
    const commandStartLine = separatorLine >= 0 ? separatorLine + 1 : 0;

    // Find command lines AFTER the separator and apply decorations
    let commandIndex = 0;
    for (let i = commandStartLine; i < editor.document.lineCount; i++) {
      const line = editor.document.lineAt(i);
      const text = line.text;

      // Match command lines (same logic as codeLensProvider)
      if (text.match(/^\s*-\s*\w+/)) {
        const cmdStatus = status.commandStatuses.find(s => s.index === commandIndex);
        if (cmdStatus) {
          const range = new vscode.Range(i, 0, i, 0);
          const decoration: vscode.DecorationOptions = {
            range,
            hoverMessage: cmdStatus.message || cmdStatus.status
          };

          switch (cmdStatus.status) {
            case 'pending':
              pendingRanges.push(decoration);
              break;
            case 'running':
              runningRanges.push(decoration);
              break;
            case 'passed':
              passedRanges.push(decoration);
              break;
            case 'failed':
              failedRanges.push(decoration);
              break;
          }
        }
        commandIndex++;
      }
    }

    editor.setDecorations(this.pendingDecorationType, pendingRanges);
    editor.setDecorations(this.runningDecorationType, runningRanges);
    editor.setDecorations(this.passedDecorationType, passedRanges);
    editor.setDecorations(this.failedDecorationType, failedRanges);
  }


  public clearDecorations(): void {
    const editor = vscode.window.activeTextEditor;
    if (editor) {
      editor.setDecorations(this.pendingDecorationType, []);
      editor.setDecorations(this.runningDecorationType, []);
      editor.setDecorations(this.passedDecorationType, []);
      editor.setDecorations(this.failedDecorationType, []);
    }
  }

  dispose(): void {
    this.pendingDecorationType.dispose();
    this.runningDecorationType.dispose();
    this.passedDecorationType.dispose();
    this.failedDecorationType.dispose();
  }
}
