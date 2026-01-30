import * as vscode from 'vscode';
import { LUMI_COMMANDS } from './schema/commands';

// Header fields that appear before ---
interface HeaderField {
  name: string;
  description: string;
  type: 'string' | 'object' | 'array' | 'number' | 'boolean';
  snippet?: string;
}

const HEADER_FIELDS: HeaderField[] = [
  { name: 'appId', description: 'Package name (Android) or Bundle ID (iOS)', type: 'string', snippet: 'appId: "$1"' },
  { name: 'name', description: 'Test file name (optional)', type: 'string', snippet: 'name: "$1"' },
  { name: 'tags', description: 'Test tags for filtering', type: 'array', snippet: 'tags:\n  - $1' },
  { name: 'env', description: 'Environment variables', type: 'object', snippet: 'env:\n  $1: "$2"' },
  { name: 'env (file)', description: 'Load environment variables from file', type: 'object', snippet: 'env:\n  file: ${1:.env}' },
  { name: 'vars', description: 'Environment variables (alias for env)', type: 'object', snippet: 'vars:\n  $1: "$2"' },
  { name: 'speed', description: 'Execution speed (turbo, fast, normal, safe)', type: 'string', snippet: 'speed: "${1|turbo,fast,normal,safe|}"' },
  { name: 'browser', description: 'Web browser (Chrome, Firefox, Webkit)', type: 'string', snippet: 'browser: "${1|Chrome,Firefox,Webkit|}"' },
  { name: 'closeWhenFinish', description: 'Close app when test finishes', type: 'boolean', snippet: 'closeWhenFinish: ${1|true,false|}' },
  { name: 'defaultTimeout', description: 'Default timeout in ms', type: 'number', snippet: 'defaultTimeout: ${1:30000}' },
  { name: 'timeout', description: 'Default timeout in ms (alias)', type: 'number', snippet: 'timeout: ${1:30000}' },
  { name: 'onFlowStart', description: 'Commands to run at flow start', type: 'object', snippet: 'onFlowStart:\n  commands:\n    - $1' },
  { name: 'onFlowComplete', description: 'Commands to run at flow end', type: 'object', snippet: 'onFlowComplete:\n  commands:\n    - $1' },
  { name: 'onFlowFail', description: 'Commands to run on flow failure', type: 'object', snippet: 'onFlowFail:\n  commands:\n    - $1' },
  { name: 'retryOnFail', description: 'Retry flow on failure', type: 'boolean', snippet: 'retryOnFail: ${1|true,false|}' },
  { name: 'locale', description: 'Device locale setting', type: 'string', snippet: 'locale: "${1:en_US}"' },
  { name: 'device', description: 'Target device ID', type: 'string', snippet: 'device: "$1"' },
  { name: 'platform', description: 'Target platform (android/ios/web)', type: 'string', snippet: 'platform: "${1|android,ios,web|}"' },
];

export class LumiCompletionProvider implements vscode.CompletionItemProvider {

  provideCompletionItems(
    document: vscode.TextDocument,
    position: vscode.Position,
    _token: vscode.CancellationToken,
    _context: vscode.CompletionContext
  ): vscode.ProviderResult<vscode.CompletionItem[] | vscode.CompletionList> {

    const lineText = document.lineAt(position.line).text;
    const linePrefix = lineText.substring(0, position.character);
    const documentText = document.getText();

    // Find if we're before or after the --- separator
    const separatorIndex = documentText.indexOf('---');
    const currentOffset = document.offsetAt(position);
    const isInHeader = separatorIndex === -1 || currentOffset < separatorIndex;

    // If in header section (before ---), suggest header fields
    if (isInHeader) {
      // Check if we're at the start of a line (for new field)
      if (linePrefix.match(/^\s*$/) || linePrefix.match(/^\s*\w*$/)) {
        return this.getHeaderCompletions();
      }
      return undefined;
    }

    // Check if we're at the start of a command (after "- ")
    if (linePrefix.match(/^\s*-\s*$/)) {
      return this.getCommandCompletions();
    }

    // Check if we're typing a command name (after "- " with partial text)
    if (linePrefix.match(/^\s*-\s+\w*$/)) {
      return this.getCommandCompletions();
    }

    // Check if we're inside a command block and need parameter completions
    // Match: "  - commandName:" at start of previous lines, now on a new indented line
    const commandMatch = linePrefix.match(/^\s*-\s*(\w+):\s*$/);
    if (commandMatch) {
      const commandName = commandMatch[1];
      return this.getParameterCompletions(commandName);
    }

    // Check if we're on an indented line after a command (for nested parameters)
    const indentMatch = linePrefix.match(/^\s+$/);
    if (indentMatch) {
      // Look backwards to find the parent command
      const parentCommand = this.findParentCommand(document, position.line);
      if (parentCommand) {
        return this.getParameterCompletions(parentCommand);
      }
    }

    // Check if we're typing a parameter name (after indent, partial word)
    const paramStartMatch = linePrefix.match(/^\s+(\w*)$/);
    if (paramStartMatch) {
      const parentCommand = this.findParentCommand(document, position.line);
      if (parentCommand) {
        return this.getParameterCompletions(parentCommand);
      }
    }

    return undefined;
  }

  private findParentCommand(document: vscode.TextDocument, currentLine: number): string | null {
    // Look backwards to find the command this parameter belongs to
    for (let i = currentLine - 1; i >= 0; i--) {
      const line = document.lineAt(i).text;

      // Found a command line: "- commandName:" or "- commandName: value"
      const cmdMatch = line.match(/^\s*-\s*(\w+):/);
      if (cmdMatch) {
        return cmdMatch[1];
      }

      // If we hit another top-level item or separator, stop
      if (line.match(/^---/) || line.match(/^\s*-\s*\w+\s*$/)) {
        break;
      }
    }
    return null;
  }

  private getHeaderCompletions(): vscode.CompletionItem[] {
    return HEADER_FIELDS.map(field => {
      const item = new vscode.CompletionItem(field.name, vscode.CompletionItemKind.Field);
      item.detail = `(${field.type}) Header field`;
      item.documentation = new vscode.MarkdownString(field.description);

      if (field.snippet) {
        item.insertText = new vscode.SnippetString(field.snippet);
      } else {
        item.insertText = `${field.name}: `;
      }

      // Sort header fields at top
      item.sortText = `0_${field.name}`;

      return item;
    });
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
      item.detail = param.type + (param.required ? ' (required)' : '');
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
