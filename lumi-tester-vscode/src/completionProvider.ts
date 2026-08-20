import * as vscode from 'vscode';
import { LUMI_COMMANDS, CommandParam, SELECTOR_PARAMS } from './schema/commands';
import { resolveJigProfileFromDocument } from './jigProfileResolver';

// Header fields that appear before ---
interface HeaderField {
  name: string;
  description: string;
  type: 'string' | 'object' | 'array' | 'number' | 'boolean';
  snippet?: string;
}

const HEADER_FIELDS: HeaderField[] = [
  { name: 'platform', description: 'Target platform (macos, android, ios, windows, web)', type: 'string', snippet: 'platform: ${1|macos,android,ios,windows,web|}' },
  { name: 'appId', description: 'Package name (Android), Bundle ID (iOS), or .app Path (macOS)', type: 'string', snippet: 'appId: "$1"' },
  { name: 'windowSize', description: 'Default window size for desktop/web (e.g. 1280x800 or struct)', type: 'object', snippet: 'windowSize:\n  width: ${1:1280}\n  height: ${2:800}' },
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
  { name: 'jig', description: 'Hardware Jig serial port configuration (e.g. COM5, struct, or profile file)', type: 'object', snippet: 'jig: "${1:COM5}"' },
  { name: 'jig (profile)', description: 'Hardware Jig reusable profile file (e.g. profiles/jig_switch_sample.yaml)', type: 'string', snippet: 'jig: "${1:profiles/jig_switch_sample.yaml}"' },
  { name: 'skip', description: 'Skip this flow when running in bulk / whole directory', type: 'boolean', snippet: 'skip: ${1|true,false|}' },
  { name: 'manual', description: 'Mark flow as manual-only (skipped when running whole directory)', type: 'boolean', snippet: 'manual: ${1|true,false|}' },
];

const SELECTOR_PROPERTIES = [
  { name: 'id', type: 'string', description: 'Find element by resource ID', snippet: 'id: "$1"' },
  { name: 'text', type: 'string', description: 'Find element by exact text', snippet: 'text: "$1"' },
  { name: 'regex', type: 'string', description: 'Find element by regex pattern', snippet: 'regex: "$1"' },
  { name: 'type', type: 'string', description: 'Find element by type (Slider, Button, View, Input...)', snippet: 'type: "${1|Slider,Button,View,Input|}"' },
  { name: 'offset', type: 'string', description: 'Relative percentage offset within element (e.g. "0%,50%" or "20%,50%")', snippet: 'offset: "${1:20%},${2:50%}"' },
  { name: 'align', type: 'string', description: 'Alignment preset within element (left, right, top, bottom, center)', snippet: 'align: ${1|left,right,top,bottom,center|}' },
  { name: 'xpath', type: 'string', description: 'Find element by XPath expression', snippet: 'xpath: "$1"' },
  { name: 'point', type: 'string', description: 'Direct coordinate (x,y or x%,y%)', snippet: 'point: "${1:50%},${2:50%}"' },
  { name: 'index', type: 'number', description: 'Element index (0-based)', snippet: 'index: ${1:1}' },
  { name: 'desc', type: 'string', description: 'Find element by accessibility content description', snippet: 'desc: "$1"' },
  { name: 'css', type: 'string', description: 'Find element by CSS selector (Web only)', snippet: 'css: "$1"' },
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
      if (linePrefix.match(/^\s*appId:\s*"?\w*$/)) {
        return this.getAppIdCompletions();
      }
      if (linePrefix.match(/^\s*$/) || linePrefix.match(/^\s*\w*$/)) {
        return this.getHeaderCompletions();
      }
      return undefined;
    }

    // 0. Check if user is typing a hardware command value (short syntax)
    const relayCmdMatch = linePrefix.match(/^\s*-\s*(hwPowerOn|hwPowerOff|hwPowerCycle|hwReadRelay):\s*"?([^"\r\n]*)$/);
    if (relayCmdMatch) {
      return this.getRelayCompletions(document);
    }

    const buttonCmdMatch = linePrefix.match(/^\s*-\s*(hwClick|hwPress|hwRelease|hwRepeatClick|hwStartRepeatClick|hwStopRepeatClick|hwReadServo|hwConfigureServo):\s*"?([^"\r\n]*)$/);
    if (buttonCmdMatch) {
      return this.getButtonCompletions(document);
    }

    const sensorCmdMatch = linePrefix.match(/^\s*-\s*(hwSeeLed|hwSeeLedBlink|hwSeeLedOff|hwReadColor|hwReadSensorLight|hwSensorLight|hwCalibrateColor|hwCalibrateBrightness|hwSetBrightnessThresholds|hwWaitForBrightness|hwWaitForCct|hwAddCctPoint):\s*"?([^"\r\n]*)$/);
    if (sensorCmdMatch) {
      return this.getSensorCompletions(document);
    }

    // 1. Check if user is typing an indented property/parameter value
    const currentIndent = lineText.match(/^(\s*)/)?.[1].length ?? 0;
    if (currentIndent > 0) {
      const path = this.resolveYamlPath(document, position.line, currentIndent);
      if (path.length > 0) {
        // Check if typing a specific parameter value for hardware commands
        const propMatch = linePrefix.match(/^\s*([A-Za-z0-9_\-]+):\s*"?([^"\r\n]*)$/);
        if (propMatch) {
          const propName = propMatch[1];
          const rootCmd = path[0];
          if (rootCmd === 'hwPowerOn' || rootCmd === 'hwPowerOff' || rootCmd === 'hwPowerCycle' || rootCmd === 'hwReadRelay') {
            if (propName === 'channel' || propName === 'channels' || propName === 'relay' || propName === 'relays' || propName === 'name') {
              return this.getRelayCompletions(document);
            }
          }
          if (rootCmd === 'hwClick' || rootCmd === 'hwPress' || rootCmd === 'hwRelease' || rootCmd === 'hwRepeatClick' || rootCmd === 'hwStartRepeatClick' || rootCmd === 'hwStopRepeatClick' || rootCmd === 'hwReadServo' || rootCmd === 'hwConfigureServo' || rootCmd === 'hwRotate') {
            if (propName === 'button' || propName === 'channel' || propName === 'btn') {
              return this.getButtonCompletions(document);
            }
          }
          if (rootCmd === 'hwSeeLed' || rootCmd === 'hwSeeLedBlink' || rootCmd === 'hwSeeLedOff' || rootCmd === 'hwReadColor' || rootCmd === 'hwReadSensorLight' || rootCmd === 'hwSensorLight' || rootCmd === 'hwCalibrateColor' || rootCmd === 'hwCalibrateBrightness' || rootCmd === 'hwSetBrightnessThresholds' || rootCmd === 'hwWaitForBrightness' || rootCmd === 'hwWaitForCct' || rootCmd === 'hwAddCctPoint') {
            if (propName === 'button' || propName === 'channel' || propName === 'sensor' || propName === 'btn') {
              return this.getSensorCompletions(document);
            }
          }
        }

        const completions = this.getNestedParamCompletions(path);
        if (completions.length > 0) {
          return completions;
        }
      }
    }

    // 2. Command completions: matches "- ", "-", "-d", "-drag"
    if (linePrefix.match(/^\s*-\s*\w*$/)) {
      return this.getCommandCompletions(false);
    }

    // 3. Command completions on new unindented line (user typed "dr" or "drag" without dash)
    if (linePrefix.match(/^\s*\w*$/)) {
      return this.getCommandCompletions(true);
    }

    return undefined;
  }

  /**
   * Traverse upwards through YAML document to construct the ancestor key path
   * e.g. ['drag', 'from'] or ['tap', 'relative', 'rightOf']
   */
  private resolveYamlPath(document: vscode.TextDocument, currentLine: number, currentIndent: number): string[] {
    const path: string[] = [];
    let targetIndent = currentIndent;

    for (let i = currentLine - 1; i >= 0; i--) {
      const line = document.lineAt(i).text;
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith('#')) {
        continue;
      }
      if (line.match(/^---/)) {
        break;
      }

      const indent = line.match(/^(\s*)/)?.[1].length ?? 0;
      if (indent < targetIndent) {
        // Check for command header: "- commandName:"
        const cmdMatch = line.match(/^\s*-\s*(\w+):/);
        if (cmdMatch) {
          path.unshift(cmdMatch[1]);
          break; // Top-level command reached
        }

        // Check for property key: "key:"
        const propMatch = line.match(/^\s*(\w+):/);
        if (propMatch) {
          path.unshift(propMatch[1]);
          targetIndent = indent;
        }
      }
    }

    return path;
  }

  /**
   * Get completions for a nested path by walking CommandParam.params hierarchy
   */
  private getNestedParamCompletions(path: string[]): vscode.CompletionItem[] {
    if (path.length === 0) {
      return [];
    }

    const [rootCmdName, ...subKeys] = path;
    const command = LUMI_COMMANDS.find(c => c.name === rootCmdName || c.aliases?.includes(rootCmdName));
    if (!command || !command.params) {
      return [];
    }

    let currentParams: CommandParam[] = command.params;

    for (const key of subKeys) {
      const matchedParam = currentParams.find(p => p.name === key);
      if (matchedParam && matchedParam.params && matchedParam.params.length > 0) {
        currentParams = matchedParam.params;
      } else if (key === 'from' || key === 'to' || key === 'target' || key === 'anchor' || key === 'rightOf' || key === 'leftOf' || key === 'above' || key === 'below') {
        currentParams = SELECTOR_PARAMS;
      } else {
        return [];
      }
    }

    return currentParams.map(param => {
      const item = new vscode.CompletionItem(param.name, vscode.CompletionItemKind.Property);
      item.detail = `(${param.type}) ${param.required ? '[required] ' : ''}Property`;
      item.documentation = new vscode.MarkdownString(param.description);

      if (param.snippet) {
        item.insertText = new vscode.SnippetString(param.snippet);
      } else if (param.type === 'string') {
        item.insertText = new vscode.SnippetString(`${param.name}: "$0"`);
      } else if (param.type === 'boolean') {
        item.insertText = new vscode.SnippetString(`${param.name}: \${1|true,false|}`);
      } else if (param.type === 'number') {
        item.insertText = new vscode.SnippetString(`${param.name}: $0`);
      } else if (param.type === 'object') {
        item.insertText = new vscode.SnippetString(`${param.name}:\n  $0`);
      } else {
        item.insertText = `${param.name}: `;
      }

      return item;
    });
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

      item.sortText = `0_${field.name}`;
      return item;
    });
  }

  private async getAppIdCompletions(): Promise<vscode.CompletionItem[]> {
    const { getMacosApplications } = await import('./appDiscovery');
    const apps = await getMacosApplications();
    return apps.map((app, idx) => {
      const item = new vscode.CompletionItem(app.name, vscode.CompletionItemKind.Module);
      item.detail = app.path;
      item.documentation = new vscode.MarkdownString(
        `**${app.name}**\n\n- Path: \`${app.path}\`${app.bundleId ? `\n- Bundle ID: \`${app.bundleId}\`` : ''}${app.isRunning ? '\n- Status: **Running**' : ''}`
      );
      item.insertText = `"${app.path}"`;
      item.sortText = `${app.isRunning ? '0' : '1'}_${String(idx).padStart(4, '0')}`;
      return item;
    });
  }

  private getCommandCompletions(includeDash: boolean = false): vscode.CompletionItem[] {
    return LUMI_COMMANDS.map(cmd => {
      const item = new vscode.CompletionItem(cmd.name, vscode.CompletionItemKind.Function);
      item.detail = cmd.category;
      item.documentation = new vscode.MarkdownString(cmd.description);

      const prefix = includeDash ? '- ' : '';
      if (cmd.snippet) {
        item.insertText = new vscode.SnippetString(`${prefix}${cmd.snippet}`);
      } else if (cmd.hasParams) {
        item.insertText = new vscode.SnippetString(`${prefix}${cmd.name}:\n    $0`);
      } else {
        item.insertText = `${prefix}${cmd.name}`;
      }

      item.kind = cmd.hasParams ? vscode.CompletionItemKind.Method : vscode.CompletionItemKind.Keyword;
      return item;
    });
  }

  private getRelayCompletions(document: vscode.TextDocument): vscode.CompletionItem[] {
    const profile = resolveJigProfileFromDocument(document);
    const items: vscode.CompletionItem[] = [];

    if (profile && profile.relays.size > 0) {
      for (const [name, cfg] of profile.relays.entries()) {
        const item = new vscode.CompletionItem(`"${name}"`, vscode.CompletionItemKind.EnumMember);
        const chStr = cfg.channels.join(', ');
        item.detail = `⚡ Relay: [${chStr}] (${name})`;
        item.documentation = new vscode.MarkdownString(
          `**Relay Alias: \`${name}\`**\n\n- Channels: \`[${chStr}]\`${profile.sourceFile ? `\n- Defined in: \`${profile.sourceFile}\`` : ''}`
        );
        item.insertText = `"${name}"`;
        item.sortText = `0_${name}`;
        items.push(item);
      }
    }

    // Also suggest common numeric relay channels
    for (let ch = 1; ch <= 8; ch++) {
      const item = new vscode.CompletionItem(`${ch}`, vscode.CompletionItemKind.Value);
      item.detail = `Relay Channel ${ch}`;
      item.insertText = `${ch}`;
      item.sortText = `1_${ch}`;
      items.push(item);
    }

    return items;
  }

  private getButtonCompletions(document: vscode.TextDocument): vscode.CompletionItem[] {
    const profile = resolveJigProfileFromDocument(document);
    const items: vscode.CompletionItem[] = [];

    if (profile && profile.buttons.size > 0) {
      for (const [name, cfg] of profile.buttons.entries()) {
        const item = new vscode.CompletionItem(`"${name}"`, vscode.CompletionItemKind.EnumMember);
        const parts: string[] = [];
        if (cfg.servo !== undefined) {
          parts.push(`Servo Ch ${cfg.servo}`);
        }
        if (cfg.sensor !== undefined) {
          parts.push(`Sensor Ch ${cfg.sensor}`);
        }
        const detailStr = parts.length > 0 ? parts.join(', ') : `Ch ${cfg.channel ?? '?'}`;
        item.detail = `🎛️ Button: ${name} (${detailStr})`;
        item.documentation = new vscode.MarkdownString(
          `**Button Alias: \`${name}\`**\n\n- Servo Channel: \`${cfg.servo ?? cfg.channel ?? 'N/A'}\`\n- Color Sensor Channel: \`${cfg.sensor ?? cfg.channel ?? 'N/A'}\`${profile.sourceFile ? `\n- Defined in: \`${profile.sourceFile}\`` : ''}`
        );
        item.insertText = `"${name}"`;
        item.sortText = `0_${name}`;
        items.push(item);
      }
    }

    // Also suggest numeric servo channels
    for (let ch = 1; ch <= 8; ch++) {
      const item = new vscode.CompletionItem(`${ch}`, vscode.CompletionItemKind.Value);
      item.detail = `Servo Channel ${ch}`;
      item.insertText = `${ch}`;
      item.sortText = `1_${ch}`;
      items.push(item);
    }

    return items;
  }

  private getSensorCompletions(document: vscode.TextDocument): vscode.CompletionItem[] {
    const profile = resolveJigProfileFromDocument(document);
    const items: vscode.CompletionItem[] = [];

    if (profile && profile.buttons.size > 0) {
      for (const [name, cfg] of profile.buttons.entries()) {
        const item = new vscode.CompletionItem(`"${name}"`, vscode.CompletionItemKind.EnumMember);
        const sensorCh = cfg.sensor ?? cfg.channel ?? cfg.servo;
        item.detail = `🎨 Sensor: ${name} (Ch ${sensorCh ?? '?'})`;
        item.documentation = new vscode.MarkdownString(
          `**Color Sensor Alias: \`${name}\`**\n\n- Sensor Channel: \`${sensorCh ?? 'N/A'}\`${profile.sourceFile ? `\n- Defined in: \`${profile.sourceFile}\`` : ''}`
        );
        item.insertText = `"${name}"`;
        item.sortText = `0_${name}`;
        items.push(item);
      }
    }

    // Also suggest numeric sensor channels
    for (let ch = 1; ch <= 8; ch++) {
      const item = new vscode.CompletionItem(`${ch}`, vscode.CompletionItemKind.Value);
      item.detail = `Sensor Channel ${ch}`;
      item.insertText = `${ch}`;
      item.sortText = `1_${ch}`;
      items.push(item);
    }

    return items;
  }
}
