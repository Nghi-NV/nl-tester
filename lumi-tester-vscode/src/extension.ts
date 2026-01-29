import * as vscode from 'vscode';
import * as path from 'path';
import { LumiCompletionProvider } from './completionProvider';
import { LumiCodeLensProvider } from './codeLensProvider';
import { LumiDecorationProvider } from './decorationProvider';
import { LumiTestRunner } from './testRunner';
import { DeviceManager } from './deviceManager';
import { InspectorPanel } from './inspectorPanel';

let terminal: vscode.Terminal | undefined;
let testRunner: LumiTestRunner | undefined;
let decorationProvider: LumiDecorationProvider | undefined;
let deviceManager: DeviceManager | undefined;

export function activate(context: vscode.ExtensionContext) {
  console.log('Lumi Tester extension is now active!');

  // Initialize device manager
  deviceManager = DeviceManager.getInstance();
  context.subscriptions.push({
    dispose: () => deviceManager?.dispose()
  });

  // Register completion provider for YAML files
  const completionProvider = new LumiCompletionProvider();
  context.subscriptions.push(
    vscode.languages.registerCompletionItemProvider(
      { language: 'yaml', scheme: 'file' },
      completionProvider,
      '-', ' ', ':'
    )
  );

  // Register CodeLens provider for play buttons
  const codeLensProvider = new LumiCodeLensProvider();
  context.subscriptions.push(
    vscode.languages.registerCodeLensProvider(
      { language: 'yaml', scheme: 'file' },
      codeLensProvider
    )
  );

  // Initialize decoration provider for status display
  decorationProvider = new LumiDecorationProvider();
  context.subscriptions.push({
    dispose: () => decorationProvider?.dispose()
  });

  // Initialize test runner
  testRunner = new LumiTestRunner();
  testRunner.onStatusChange((status) => {
    decorationProvider?.updateDecorations(status);
  });

  // Register commands
  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.runFile', async () => {
      const editor = vscode.window.activeTextEditor;
      if (editor && editor.document.languageId === 'yaml') {
        decorationProvider?.clearDecorations();
        await runTestFile(editor.document.uri);
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.runCommand', async (uri: vscode.Uri, commandIndex: number) => {
      decorationProvider?.clearDecorations();
      await runSingleCommand(uri, commandIndex);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.stopTest', () => {
      if (testRunner) {
        testRunner.stop();
      }
      if (terminal) {
        terminal.sendText('\x03'); // Send Ctrl+C
      }
    })
  );

  // Device selection commands
  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.selectDevice', async () => {
      await deviceManager?.showDevicePicker();
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.refreshDevices', async () => {
      const devices = await deviceManager?.refreshDevices(true);
      vscode.window.showInformationMessage(`Found ${devices?.length || 0} devices`);
    })
  );

  // Inspector command
  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.openInspector', async () => {
      console.log('Lumi: openInspector command triggered');

      const editor = vscode.window.activeTextEditor;
      const filePath = editor?.document.uri.fsPath || '';
      console.log('Lumi: Active file path:', filePath);

      const lumiPath = findLumiTesterPath(filePath);
      console.log('Lumi: Found lumi-tester path:', lumiPath);

      if (!lumiPath) {
        vscode.window.showErrorMessage('Could not find lumi-tester. Please set lumi-tester.lumiTesterPath in settings.');
        return;
      }

      try {
        await InspectorPanel.show(context, lumiPath);
        console.log('Lumi: InspectorPanel.show() completed');
      } catch (error) {
        console.error('Lumi: Error showing inspector panel:', error);
        vscode.window.showErrorMessage(`Failed to open inspector: ${error}`);
      }
    })
  );

  console.log('Lumi Tester extension activated successfully');
}

function getOrCreateTerminal(): vscode.Terminal {
  if (!terminal || terminal.exitStatus !== undefined) {
    terminal = vscode.window.createTerminal({
      name: 'Lumi Tester',
      iconPath: new vscode.ThemeIcon('beaker')
    });
  }
  return terminal;
}

function findLumiTesterPath(testFilePath: string): string | null {
  // Check configuration first
  const config = vscode.workspace.getConfiguration('lumi-tester');
  const configPath = config.get<string>('lumiTesterPath');
  if (configPath && configPath.length > 0) {
    return configPath;
  }

  // Try to find lumi-tester in parent directories
  let dir = path.dirname(testFilePath);
  const fs = require('fs');

  for (let i = 0; i < 10; i++) {
    const cargoPath = path.join(dir, 'Cargo.toml');
    try {
      if (fs.existsSync(cargoPath)) {
        const content = fs.readFileSync(cargoPath, 'utf8');
        if (content.includes('lumi-tester')) {
          return dir;
        }
      }
    } catch {
      // Ignore
    }

    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }

  // Fallback: assume relative to workspace
  const workspaceFolder = vscode.workspace.workspaceFolders?.[0];
  if (workspaceFolder) {
    const possiblePaths = [
      path.join(workspaceFolder.uri.fsPath, 'lumi-tester'),
      workspaceFolder.uri.fsPath
    ];
    for (const p of possiblePaths) {
      try {
        if (fs.existsSync(path.join(p, 'Cargo.toml'))) {
          return p;
        }
      } catch {
        // Ignore
      }
    }
  }

  return null;
}

function buildDeviceArgs(): string {
  const device = deviceManager?.getSelectedDevice();
  if (!device) {
    return '';
  }
  return `--platform ${device.platform} --device "${device.id}"`;
}

async function runTestFile(uri: vscode.Uri): Promise<void> {
  const filePath = uri.fsPath;
  const lumiPath = findLumiTesterPath(filePath);

  if (!lumiPath) {
    vscode.window.showErrorMessage('Could not find lumi-tester. Please set lumi-tester.lumiTesterPath in settings.');
    return;
  }

  // Ensure device is selected (auto-select if only 1, prompt if multiple)
  await deviceManager?.ensureDeviceSelected();

  const term = getOrCreateTerminal();
  term.show(true);

  const deviceArgs = buildDeviceArgs();
  const command = deviceArgs
    ? `cd "${lumiPath}" && cargo run -- run "${filePath}" ${deviceArgs}`
    : `cd "${lumiPath}" && cargo run -- run "${filePath}"`;

  term.sendText(command);
}

async function runSingleCommand(uri: vscode.Uri, commandIndex: number): Promise<void> {
  const filePath = uri.fsPath;
  const lumiPath = findLumiTesterPath(filePath);

  if (!lumiPath) {
    vscode.window.showErrorMessage('Could not find lumi-tester. Please set lumi-tester.lumiTesterPath in settings.');
    return;
  }

  // Ensure device is selected (auto-select if only 1, prompt if multiple)
  await deviceManager?.ensureDeviceSelected();

  const term = getOrCreateTerminal();
  term.show(true);

  const deviceArgs = buildDeviceArgs();
  const command = deviceArgs
    ? `cd "${lumiPath}" && cargo run -- run "${filePath}" --command-index ${commandIndex} ${deviceArgs}`
    : `cd "${lumiPath}" && cargo run -- run "${filePath}" --command-index ${commandIndex}`;

  term.sendText(command);
}

export function deactivate() {
  if (terminal) {
    terminal.dispose();
  }
  if (decorationProvider) {
    decorationProvider.dispose();
  }
  if (deviceManager) {
    deviceManager.dispose();
  }
}

