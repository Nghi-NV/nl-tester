import * as vscode from 'vscode';
import { LUMI_COMMANDS } from './schema/commands';

export class LumiCompletionProvider implements vscode.CompletionItemProvider {

  provideCompletionItems(
    document: vscode.TextDocument,
    position: vscode.Position,
    token: vscode.CancellationToken,
    context: vscode.CompletionContext
  ): vscode.ProviderResult<vscode.CompletionItem[] | vscode.CompletionList> {

    const lineText = document.lineAt(position.line).text;
    const linePrefix = lineText.substring(0, position.character);

    // Check if we're at the start of a command (after "- ")
    if (linePrefix.match(/^\s*-\s*$/)) {
      return this.getCommandCompletions();
    }

    // Check if we're typing a command name
    if (linePrefix.match(/^\s*-\s*\w*$/)) {
      return this.getCommandCompletions();
    }

    // Check if we're inside a command and need parameter completions
    const commandMatch = linePrefix.match(/^\s*-\s*(\w+):\s*$/);
    if (commandMatch) {
      const commandName = commandMatch[1];
      return this.getParameterCompletions(commandName);
    }

    return undefined;
  }

  private getCommandCompletions(): vscode.CompletionItem[] {
    return LUMI_COMMANDS.map(cmd => {
      const item = new vscode.CompletionItem(cmd.name, vscode.CompletionItemKind.Function);
      item.detail = cmd.category;
      item.documentation = new vscode.MarkdownString(cmd.description);

      if (cmd.snippet) {
        item.insertText = new vscode.SnippetString(cmd.snippet);
      } else if (cmd.hasParams) {
        item.insertText = new vscode.SnippetString(`${cmd.name}:\n    $0`);
      } else {
        item.insertText = cmd.name;
      }

      // Add command icon
      item.kind = cmd.hasParams ? vscode.CompletionItemKind.Method : vscode.CompletionItemKind.Keyword;

      return item;
    });
  }

  private getParameterCompletions(commandName: string): vscode.CompletionItem[] {
    const command = LUMI_COMMANDS.find(c => c.name === commandName || c.aliases?.includes(commandName));
    if (!command || !command.params) {
      return [];
    }

    return command.params.map(param => {
      const item = new vscode.CompletionItem(param.name, vscode.CompletionItemKind.Property);
      item.detail = param.type;
      item.documentation = new vscode.MarkdownString(param.description);

      if (param.type === 'string') {
        item.insertText = new vscode.SnippetString(`${param.name}: "$0"`);
      } else if (param.type === 'boolean') {
        item.insertText = new vscode.SnippetString(`${param.name}: \${1|true,false|}`);
      } else if (param.type === 'number') {
        item.insertText = new vscode.SnippetString(`${param.name}: $0`);
      } else {
        item.insertText = `${param.name}: `;
      }

      return item;
    });
  }
}
