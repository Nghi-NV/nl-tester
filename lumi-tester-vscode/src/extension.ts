import { execFileSync } from 'child_process';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vscode from 'vscode';
import { LumiCodeLensProvider } from './codeLensProvider';
import { buildRunInvocation } from './commandInvocation';
import { LumiCompletionProvider } from './completionProvider';
import { LumiDecorationProvider } from './decorationProvider';
import { parseAdbDevices } from './deviceDiscovery';
import { DeviceManager } from './deviceManager';
import { InspectorPanel } from './inspectorPanel';
import { MockLocationPanel } from './mockLocationPanel';
import {
  LumiRuntime,
  resolveAdbExecutable,
  resolveLumiRuntime,
  RuntimeResolverOptions
} from './runtimeResolver';

let taskExecution: vscode.TaskExecution | undefined;
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
    vscode.commands.registerCommand('lumi-tester.runFromCommand', async (uri: vscode.Uri, fromCommandIndex: number) => {
      decorationProvider?.clearDecorations();
      await runFromCommand(uri, fromCommandIndex);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.stopTest', () => {
      if (taskExecution) {
        taskExecution.terminate();
        taskExecution = undefined;
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
      const uri = editor?.document.uri;
      console.log('Lumi: Active file path:', uri?.fsPath || '');

      if (!uri) {
        vscode.window.showErrorMessage('Open a YAML file before starting Lumi Inspector.');
        return;
      }
      const runtime = resolveRuntimeOrShow(uri);
      if (!runtime) return;
      console.log('Lumi: Found lumi-tester runtime:', runtime);

      try {
        const device = deviceManager?.getSelectedDevice() || undefined;
        await InspectorPanel.show(context, runtime, device);
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
      const uri = editor?.document.uri;
      if (!uri) {
        vscode.window.showErrorMessage('Open a YAML file before opening GPS Control.');
        return;
      }
      const runtime = resolveRuntimeOrShow(uri);
      if (!runtime) return;

      MockLocationPanel.show(context, runtime.cwd ?? runtime.executable, 60);
    })
  );

  const diagnostics = vscode.window.createOutputChannel('Lumi Setup');
  context.subscriptions.push(diagnostics);
  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.diagnoseSetup', () => {
      diagnostics.clear();
      diagnostics.show(true);
      const uri = vscode.window.activeTextEditor?.document.uri
        ?? vscode.workspace.workspaceFolders?.[0]?.uri;
      if (!uri) {
        diagnostics.appendLine('No workspace or active file.');
        return;
      }

      try {
        const options = resolverOptions(uri);
        const runtime = resolveLumiRuntime(options);
        diagnostics.appendLine(`CLI kind: ${runtime.kind}`);
        diagnostics.appendLine(`CLI path: ${runtime.executable}`);
        const version = execFileSync(
          runtime.executable,
          [...runtime.argsPrefix, '--version'],
          {
            encoding: 'utf8',
            cwd: runtime.cwd,
            windowsHide: true,
            timeout: 10000
          }
        );
        diagnostics.appendLine(`CLI version: ${version.trim()}`);

        const adb = resolveAdbExecutable(options);
        diagnostics.appendLine(`ADB path: ${adb ?? 'not found'}`);
        if (adb) {
          const adbOutput = execFileSync(
            adb,
            ['devices', '-l'],
            { encoding: 'utf8', windowsHide: true, timeout: 10000 }
          );
          diagnostics.appendLine(`Android devices: ${parseAdbDevices(adbOutput).length}`);
        }
      } catch (error) {
        diagnostics.appendLine(`ERROR: ${error}`);
      }
    })
  );

  // Hardware Jig Detect & Ping commands
  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.detectJigPorts', async () => {
      const editor = vscode.window.activeTextEditor;
      const workspaceUri = editor?.document.uri || vscode.workspace.workspaceFolders?.[0]?.uri;
      if (!workspaceUri) {
        vscode.window.showErrorMessage('Open a workspace or YAML file first.');
        return;
      }
      const runtime = resolveRuntimeOrShow(workspaceUri);
      if (!runtime) return;

      try {
        const raw = execFileSync(
          runtime.executable,
          [...runtime.argsPrefix, 'jig', 'ports', '--json'],
          {
            cwd: runtime.cwd,
            encoding: 'utf8',
            windowsHide: true,
            timeout: 10000
          }
        );
        const ports = JSON.parse(raw) as Array<{
          portName: string;
          portType: string;
          manufacturer?: string | null;
          product?: string | null;
          vid?: number | null;
          pid?: number | null;
        }>;

        if (!ports || ports.length === 0) {
          vscode.window.showWarningMessage('No Serial / COM ports detected. Check your USB Jig connection.');
          return;
        }

        const items = ports.map(p => {
          const details = [p.product, p.manufacturer].filter(Boolean).join(' - ');
          return {
            label: `$(plug) ${p.portName}`,
            description: `[${p.portType}] ${details}`,
            port: p.portName
          };
        });

        const selected = await vscode.window.showQuickPick(items, {
          placeHolder: 'Select a COM Port to Ping or use in test header',
          title: `Detected ${ports.length} Serial / COM Ports`
        });

        if (selected) {
          const action = await vscode.window.showQuickPick([
            { label: `$(radio-tower) Ping ${selected.port}`, action: 'ping' },
            { label: `$(copy) Copy "${selected.port}" to Clipboard`, action: 'copy' },
            { label: `$(edit) Insert 'jig: "${selected.port}"' into Active File Header`, action: 'insert' }
          ], { placeHolder: `Action for ${selected.port}` });

          if (action?.action === 'ping') {
            await pingJigPort(workspaceUri, runtime, selected.port);
          } else if (action?.action === 'copy') {
            await vscode.env.clipboard.writeText(selected.port);
            vscode.window.showInformationMessage(`Copied ${selected.port} to clipboard.`);
          } else if (action?.action === 'insert' && editor) {
            editor.edit(editBuilder => {
              editBuilder.insert(new vscode.Position(0, 0), `jig: "${selected.port}"\n`);
            });
            vscode.window.showInformationMessage(`Inserted 'jig: "${selected.port}"' into header.`);
          }
        }
      } catch (e: any) {
        vscode.window.showErrorMessage(`Failed to detect COM ports: ${e.message || e}`);
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.pingJig', async () => {
      const editor = vscode.window.activeTextEditor;
      const workspaceUri = editor?.document.uri || vscode.workspace.workspaceFolders?.[0]?.uri;
      if (!workspaceUri) {
        vscode.window.showErrorMessage('Open a workspace or YAML file first.');
        return;
      }
      const runtime = resolveRuntimeOrShow(workspaceUri);
      if (!runtime) return;

      const portInput = await vscode.window.showInputBox({
        prompt: 'Enter Serial Port name (e.g. COM5, /dev/ttyUSB0) or Jig Profile path',
        placeHolder: 'COM5'
      });
      if (portInput) {
        await pingJigPort(workspaceUri, runtime, portInput.trim());
      }
    })
  );

  // Check for updates
  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.checkForUpdates', async () => {
      const editor = vscode.window.activeTextEditor;
      const workspaceUri = editor?.document.uri || vscode.workspace.workspaceFolders?.[0]?.uri;
      if (!workspaceUri) {
        vscode.window.showErrorMessage('Open a workspace or YAML file first.');
        return;
      }
      const runtime = resolveRuntimeOrShow(workspaceUri);
      if (!runtime) return;

      await checkLumiUpdates(workspaceUri, runtime, false);
    })
  );

  // Update CLI & Extension
  context.subscriptions.push(
    vscode.commands.registerCommand('lumi-tester.update', async () => {
      const editor = vscode.window.activeTextEditor;
      const workspaceUri = editor?.document.uri || vscode.workspace.workspaceFolders?.[0]?.uri;
      if (!workspaceUri) {
        vscode.window.showErrorMessage('Open a workspace or YAML file first.');
        return;
      }
      const runtime = resolveRuntimeOrShow(workspaceUri);
      if (!runtime) return;

      await performLumiUpdate(workspaceUri, runtime);
    })
  );

  console.log('Lumi Tester extension activated successfully');
}

async function checkLumiUpdates(uri: vscode.Uri, runtime: LumiRuntime, silentIfUpToDate = false): Promise<void> {
  await vscode.window.withProgress({
    location: vscode.ProgressLocation.Notification,
    title: 'Checking for Lumi Tester updates...',
    cancellable: false
  }, async () => {
    try {
      const raw = execFileSync(
        runtime.executable,
        [...runtime.argsPrefix, 'update', '--check', '--json'],
        {
          cwd: runtime.cwd,
          encoding: 'utf8',
          windowsHide: true,
          timeout: 15000
        }
      );
      const res = JSON.parse(raw);
      if (res.cli_update_available || res.extension_update_available) {
        const items = [];
        if (res.cli_update_available) items.push(`CLI: ${res.cli_current} → ${res.cli_latest}`);
        if (res.extension_update_available) items.push(`Extension: ${res.extension_current || 'current'} → ${res.extension_latest}`);

        const action = await vscode.window.showInformationMessage(
          `🚀 Lumi Tester Update Available! (${items.join(', ')})`,
          'Update Now',
          'Release Notes'
        );
        if (action === 'Update Now') {
          await performLumiUpdate(uri, runtime);
        } else if (action === 'Release Notes') {
          vscode.env.openExternal(vscode.Uri.parse('https://github.com/Nghi-NV/nl-tester/releases'));
        }
      } else if (!silentIfUpToDate) {
        vscode.window.showInformationMessage(`✅ Lumi Tester is up to date (${res.cli_current})!`);
      }
    } catch (e: any) {
      if (!silentIfUpToDate) {
        vscode.window.showErrorMessage(`Failed to check updates: ${e.message || e}`);
      }
    }
  });
}

async function performLumiUpdate(uri: vscode.Uri, runtime: LumiRuntime): Promise<void> {
  await vscode.window.withProgress({
    location: vscode.ProgressLocation.Notification,
    title: 'Updating Lumi Tester CLI & Extension...',
    cancellable: false
  }, async () => {
    try {
      const terminal = vscode.window.createTerminal({
        name: 'Lumi Tester Update',
        cwd: runtime.cwd
      });
      terminal.show();
      const prefix = runtime.argsPrefix.length > 0 ? ` ${runtime.argsPrefix.join(' ')}` : '';
      terminal.sendText(`${runtime.executable}${prefix} update --all`);
    } catch (e: any) {
      vscode.window.showErrorMessage(`Failed to run update: ${e.message || e}`);
    }
  });
}

async function pingJigPort(uri: vscode.Uri, runtime: LumiRuntime, port: string): Promise<void> {
  await vscode.window.withProgress({
    location: vscode.ProgressLocation.Notification,
    title: `Pinging Hardware Jig on ${port}...`,
    cancellable: false
  }, async () => {
    try {
      const raw = execFileSync(
        runtime.executable,
        [...runtime.argsPrefix, 'jig', 'ping', port, '--json'],
        {
          cwd: runtime.cwd,
          encoding: 'utf8',
          windowsHide: true,
          timeout: 10000
        }
      );
      const res = JSON.parse(raw);
      if (res.connected) {
        const details = [
          `Latency: ${res.latencyMs}ms`,
          res.nodeId !== undefined && res.nodeId !== null ? `Node ID: ${res.nodeId}` : null,
          res.firmwareVersion ? `FW: ${res.firmwareVersion}` : null,
          res.systemStatus ? `Status: ${res.systemStatus}` : null
        ].filter(Boolean).join(', ');
        vscode.window.showInformationMessage(`🔌 Connected to Jig on ${port}! (${details})`);
      } else {
        vscode.window.showErrorMessage(`❌ Failed to connect to Jig on ${port}: ${res.error || 'Unknown error'}`);
      }
    } catch (e: any) {
      vscode.window.showErrorMessage(`❌ Failed to ping Jig on ${port}: ${e.message || e}`);
    }
  });
}

function lookupCommand(name: string): string | undefined {
  const executable = process.platform === 'win32' ? 'where.exe' : 'which';
  try {
    return execFileSync(executable, [name], { encoding: 'utf8', windowsHide: true })
      .split(/\r?\n/)
      .map(value => value.trim())
      .find(Boolean);
  } catch {
    return undefined;
  }
}

function resolverOptions(uri: vscode.Uri): RuntimeResolverOptions {
  const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri)?.uri.fsPath
    ?? vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  return {
    platform: process.platform,
    homeDir: os.homedir(),
    workspaceFolder,
    configuredPath: vscode.workspace
      .getConfiguration('lumi-tester', uri)
      .get<string>('lumiTesterPath'),
    pathLookup: lookupCommand,
    exists: fs.existsSync,
    isFile: value => fs.existsSync(value) && fs.statSync(value).isFile(),
    isLumiSourceDirectory: value => {
      const manifest = path.join(value, 'Cargo.toml');
      return fs.existsSync(manifest)
        && fs.readFileSync(manifest, 'utf8').includes('lumi-tester');
    }
  };
}

function resolveRuntime(uri: vscode.Uri): LumiRuntime {
  return resolveLumiRuntime(resolverOptions(uri));
}

function resolveRuntimeOrShow(uri: vscode.Uri): LumiRuntime | undefined {
  try {
    return resolveRuntime(uri);
  } catch (error) {
    vscode.window.showErrorMessage(`Could not resolve Lumi Tester CLI: ${error}`);
    return undefined;
  }
}

async function runTestFile(uri: vscode.Uri): Promise<void> {
  const filePath = uri.fsPath;
  const runtime = resolveRuntimeOrShow(uri);
  if (!runtime) return;

  // Check if YAML contains mockLocation/gps command and auto-show GPS Control panel
  try {
    const fs = require('fs');
    const content = fs.readFileSync(filePath, 'utf8');
    if (/(?:mockLocation|gps):/i.test(content) && extensionContext) {
      // Parse speed from YAML content
      const speedMatch = content.match(/speed:\s*([\d.]+)/i);
      const initialSpeed = speedMatch ? parseFloat(speedMatch[1]) : 60;

      MockLocationPanel.show(extensionContext, runtime.cwd ?? runtime.executable, initialSpeed);
    }
  } catch (e) {
    // Ignore read errors
  }

  // Ensure device is selected (auto-select if only 1, prompt if multiple)
  await deviceManager?.ensureDeviceSelected();

  await executeRunTask(uri, runtime);
}

async function runSingleCommand(uri: vscode.Uri, commandIndex: number): Promise<void> {
  const runtime = resolveRuntimeOrShow(uri);
  if (!runtime) return;

  // Ensure device is selected (auto-select if only 1, prompt if multiple)
  await deviceManager?.ensureDeviceSelected();

  await executeRunTask(uri, runtime, commandIndex, undefined);
}

async function runFromCommand(uri: vscode.Uri, fromCommandIndex: number): Promise<void> {
  const runtime = resolveRuntimeOrShow(uri);
  if (!runtime) return;

  // Ensure device is selected (auto-select if only 1, prompt if multiple)
  await deviceManager?.ensureDeviceSelected();

  await executeRunTask(uri, runtime, undefined, fromCommandIndex);
}

async function executeRunTask(
  uri: vscode.Uri,
  runtime: LumiRuntime,
  commandIndex?: number,
  fromCommandIndex?: number
): Promise<void> {
  try {
    const invocation = buildRunInvocation({
      runtime,
      testFilePath: uri.fsPath,
      commandIndex,
      fromCommandIndex,
      device: deviceManager?.getSelectedDevice() ?? undefined
    });
    const execution = new vscode.ProcessExecution(
      invocation.executable,
      invocation.args,
      invocation.cwd ? { cwd: invocation.cwd } : undefined
    );
    const scope = vscode.workspace.getWorkspaceFolder(uri) ?? vscode.TaskScope.Workspace;
    let taskName = 'Run Test File';
    if (commandIndex !== undefined) {
      taskName = `Run Command ${commandIndex}`;
    } else if (fromCommandIndex !== undefined) {
      taskName = `Run From Command ${fromCommandIndex}`;
    }
    const task = new vscode.Task(
      { type: 'lumi-tester' },
      scope,
      taskName,
      'Lumi Tester',
      execution
    );
    task.presentationOptions = {
      reveal: vscode.TaskRevealKind.Always,
      panel: vscode.TaskPanelKind.Dedicated,
      clear: true
    };
    taskExecution?.terminate();
    taskExecution = await vscode.tasks.executeTask(task);
  } catch (error) {
    vscode.window.showErrorMessage(`Failed to run Lumi Tester: ${error}`);
  }
}

export function deactivate() {
  if (taskExecution) {
    taskExecution.terminate();
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
