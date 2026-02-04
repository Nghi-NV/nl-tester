import { exec } from 'child_process';
import * as path from 'path';
import * as vscode from 'vscode';
import { LumiCodeLensProvider } from './codeLensProvider';
import { LumiCompletionProvider } from './completionProvider';
import { LumiDecorationProvider } from './decorationProvider';
import { DeviceManager } from './deviceManager';
import { InspectorPanel } from './inspectorPanel';
import { MockLocationPanel } from './mockLocationPanel';
import { LumiTestRunner } from './testRunner';

let terminal: vscode.Terminal | undefined;
let testRunner: LumiTestRunner | undefined;
let decorationProvider: LumiDecorationProvider | undefined;
let deviceManager: DeviceManager | undefined;
let gpsStatusBarItem: vscode.StatusBarItem | undefined;
let extensionContext: vscode.ExtensionContext | undefined;

export function activate(context: vscode.ExtensionContext) {
  console.log('Lumi Tester extension is now active!');
  extensionContext = context;

  // Initialize device manager
  deviceManager = DeviceManager.getInstance();
  context.subscriptions.push({
    dispose: () => deviceManager?.dispose()
  });

  // Create GPS Control Status Bar Item
  gpsStatusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  gpsStatusBarItem.text = '$(compass) GPS Control';
  gpsStatusBarItem.tooltip = 'Open GPS Speed Control Panel';
  gpsStatusBarItem.command = 'lumi-tester.openGpsControl';
  context.subscriptions.push(gpsStatusBarItem);

  // Show status bar when editing YAML files
  const updateStatusBarVisibility = () => {
    const editor = vscode.window.activeTextEditor;
    if (editor && editor.document.languageId === 'yaml') {
      gpsStatusBarItem?.show();
    } else {
      gpsStatusBarItem?.hide();
    }
  };

  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor(updateStatusBarVisibility)
  );
  updateStatusBarVisibility();

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

  // Mock location event handlers
  testRunner.onMockLocationStarted((data) => {
    const editor = vscode.window.activeTextEditor;
    const filePath = editor?.document.uri.fsPath || '';
    const lumiPath = findLumiTesterPath(filePath);

    if (lumiPath) {
      MockLocationPanel.show(context, lumiPath, 60);
      vscode.window.showInformationMessage(`🛰️ GPS Mock started with ${data.pointCount} points`);
    }
  });

  // Note: Auto-hide disabled - panel persists until manually closed
  // testRunner.onMockLocationStopped(() => {
  //   MockLocationPanel.hide();
  // });

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
        const device = deviceManager?.getSelectedDevice() || undefined;
        await InspectorPanel.show(context, lumiPath, device);
        console.log('Lumi: InspectorPanel.show() completed');
      } catch (error) {
        console.error('Lumi: Error showing inspector panel:', error);
        vscode.window.showErrorMessage(`Failed to open inspector: ${error}`);
      }
    })
  );

  // GPS Speed Control command
  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.openGpsControl', async () => {
      const editor = vscode.window.activeTextEditor;
      const filePath = editor?.document.uri.fsPath || '';
      const lumiPath = findLumiTesterPath(filePath);

      if (!lumiPath) {
        vscode.window.showErrorMessage('Could not find lumi-tester. Please set lumi-tester.lumiTesterPath in settings.');
        return;
      }

      MockLocationPanel.show(context, lumiPath, 60);
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

  // 3. Check global PATH
  const checkGlobal = new Promise<string>((resolve, reject) => {
    const command = process.platform === 'win32' ? 'where lumi-tester' : 'which lumi-tester';
    exec(command, (err: Error | null, stdout: string) => {
      if (err || !stdout) {
        reject(err);
      } else {
        resolve(stdout.split('\n')[0].trim());
      }
    });
  });

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

  // Check if YAML contains mockLocation/gps command and auto-show GPS Control panel
  try {
    const fs = require('fs');
    const content = fs.readFileSync(filePath, 'utf8');
    if (/(?:mockLocation|gps):/i.test(content) && extensionContext) {
      // Parse speed from YAML content
      const speedMatch = content.match(/speed:\s*([\d.]+)/i);
      const initialSpeed = speedMatch ? parseFloat(speedMatch[1]) : 60;

      MockLocationPanel.show(extensionContext, lumiPath, initialSpeed);
    }
  } catch (e) {
    // Ignore read errors
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
  if (gpsStatusBarItem) {
    gpsStatusBarItem.dispose();
  }
}

