import * as vscode from 'vscode';
import { resolveJigProfileFromDocument } from './jigProfileResolver';

export class LumiJigHoverProvider implements vscode.HoverProvider {
  provideHover(
    document: vscode.TextDocument,
    position: vscode.Position,
    _token: vscode.CancellationToken
  ): vscode.ProviderResult<vscode.Hover> {
    const range = document.getWordRangeAtPosition(position, /[A-Za-z0-9_\-]+/);
    if (!range) {
      return undefined;
    }

    const word = document.getText(range);
    const profile = resolveJigProfileFromDocument(document);
    if (!profile) {
      return undefined;
    }

    // Check if hovered word is a Relay name
    if (profile.relays.has(word)) {
      const relay = profile.relays.get(word)!;
      const chList = relay.channels.join(', ');
      const md = new vscode.MarkdownString();
      md.appendMarkdown(`### ⚡ Relay: \`${word}\`\n\n`);
      md.appendMarkdown(`- **Relay Channels:** \`[${chList}]\`\n`);
      if (profile.sourceFile) {
        md.appendMarkdown(`- **Defined in:** \`${profile.sourceFile}\`\n`);
      }
      return new vscode.Hover(md, range);
    }

    // Check if hovered word is a Button / Servo / Sensor name
    if (profile.buttons.has(word)) {
      const btn = profile.buttons.get(word)!;
      const md = new vscode.MarkdownString();
      md.appendMarkdown(`### 🎛️ Button: \`${word}\`\n\n`);
      if (btn.servo !== undefined) {
        md.appendMarkdown(`- **Servo Channel:** \`${btn.servo}\`\n`);
      }
      if (btn.sensor !== undefined) {
        md.appendMarkdown(`- **Color Sensor Channel:** \`${btn.sensor}\`\n`);
      }
      if (btn.channel !== undefined && btn.servo === undefined && btn.sensor === undefined) {
        md.appendMarkdown(`- **Channel:** \`${btn.channel}\`\n`);
      }
      if (profile.sourceFile) {
        md.appendMarkdown(`- **Defined in:** \`${profile.sourceFile}\`\n`);
      }
      return new vscode.Hover(md, range);
    }

    return undefined;
  }
}
