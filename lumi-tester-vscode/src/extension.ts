import { execFileSync } from 'child_process';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vscode from 'vscode';
import { LumiCodeLensProvider } from './codeLensProvider';
import { buildRunInvocation } from './commandInvocation';
import { LumiCompletionProvider } from './completionProvider';
import { LumiDecorationProvider } from './decorationProvider';
import { DeviceManager } from './deviceManager';
import { InspectorPanel } from './inspectorPanel';
import { MockLocationPanel } from './mockLocationPanel';
import {
  LumiRuntime,
  resolveLumiRuntime,
  RuntimeResolverOptions
} from './runtimeResolver';
import { LumiTestRunner } from './testRunner';

let taskExecution: vscode.TaskExecution | undefined;
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
    const uri = editor?.document.uri;

    if (uri) {
      const runtime = resolveRuntimeOrShow(uri);
      if (runtime) {
        MockLocationPanel.show(context, runtime.cwd ?? runtime.executable, 60);
      }
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
        await InspectorPanel.show(context, runtime.cwd ?? runtime.executable, device);
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

  console.log('Lumi Tester extension activated successfully');
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

  await executeRunTask(uri, runtime, commandIndex);
}

async function executeRunTask(uri: vscode.Uri, runtime: LumiRuntime, commandIndex?: number): Promise<void> {
  try {
    const invocation = buildRunInvocation({
      runtime,
      testFilePath: uri.fsPath,
      commandIndex,
      device: deviceManager?.getSelectedDevice() ?? undefined
    });
    const execution = new vscode.ProcessExecution(
      invocation.executable,
      invocation.args,
      invocation.cwd ? { cwd: invocation.cwd } : undefined
    );
    const scope = vscode.workspace.getWorkspaceFolder(uri) ?? vscode.TaskScope.Workspace;
    const task = new vscode.Task(
      { type: 'lumi-tester' },
      scope,
      commandIndex === undefined ? 'Run Test File' : `Run Command ${commandIndex}`,
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
