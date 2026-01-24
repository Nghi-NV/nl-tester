import * as vscode from 'vscode';

export class LumiCodeLensProvider implements vscode.CodeLensProvider {
  private _onDidChangeCodeLenses: vscode.EventEmitter<void> = new vscode.EventEmitter<void>();
  public readonly onDidChangeCodeLenses: vscode.Event<void> = this._onDidChangeCodeLenses.event;

  provideCodeLenses(
    document: vscode.TextDocument,
    token: vscode.CancellationToken
  ): vscode.ProviderResult<vscode.CodeLens[]> {
    const codeLenses: vscode.CodeLens[] = [];

    // Find the --- separator line
    let separatorLine = -1;
    for (let i = 0; i < document.lineCount; i++) {
      const line = document.lineAt(i);
      if (line.text.trim() === '---') {
        separatorLine = i;
        break;
      }
    }

    // If no separator found, treat entire file as commands starting from line 0
    const commandStartLine = separatorLine >= 0 ? separatorLine + 1 : 0;

    // Add "Run All" button at the separator line (or first line if no separator)
    const runAllLine = separatorLine >= 0 ? separatorLine : 0;
    codeLenses.push(new vscode.CodeLens(
      document.lineAt(runAllLine).range,
      {
        title: '▶ Run All',
        command: 'lumi-tester.runFile',
        arguments: [document.uri]
      }
    ));

    // Find all commands AFTER the separator and add "Run" button for each
    let commandIndex = 0;
    for (let i = commandStartLine; i < document.lineCount; i++) {
      const line = document.lineAt(i);
      const text = line.text;

      // Match command lines: "- commandName" or "- commandName:"
      // Must start with optional whitespace, then "-", then word characters
      const commandMatch = text.match(/^(\s*)-\s*(\w+)/);
      if (commandMatch) {
        const range = new vscode.Range(i, 0, i, text.length);

        codeLenses.push(new vscode.CodeLens(
          range,
          {
            title: `▷ Run [${commandIndex}]`,
            command: 'lumi-tester.runCommand',
            arguments: [document.uri, commandIndex]
          }
        ));

        commandIndex++;
      }
    }

    return codeLenses;
  }

  public refresh(): void {
    this._onDidChangeCodeLenses.fire();
  }
}
