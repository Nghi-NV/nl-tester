import * as child_process from 'child_process';
import * as net from 'net';
import * as vscode from 'vscode';
import { Device } from './deviceManager';

export class InspectorPanel {
  public static currentPanel: InspectorPanel | undefined;
  private static readonly viewType = 'lumiInspector';

  private readonly _panel: vscode.WebviewPanel;
  private _inspectorProcess: child_process.ChildProcess | undefined;
  private _port: number = 9333;
  private _disposables: vscode.Disposable[] = [];
  private _selectedDevice: Device | undefined;
  private _outputChannel: vscode.OutputChannel;

  public static async show(context: vscode.ExtensionContext, lumiTesterPath: string, device?: Device) {
    const column = vscode.ViewColumn.Beside;

    // If we already have a panel, show it
    if (InspectorPanel.currentPanel) {
      if (device) {
        InspectorPanel.currentPanel.setDevice(device);
      }
      InspectorPanel.currentPanel._panel.reveal(column);
      return;
    }

    // Create a new panel
    const panel = vscode.window.createWebviewPanel(
      InspectorPanel.viewType,
      '🔍 Lumi Inspector',
      column,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
        localResourceRoots: []
      }
    );

    InspectorPanel.currentPanel = new InspectorPanel(panel, context, lumiTesterPath, device);
  }

  private constructor(
    panel: vscode.WebviewPanel,
    private context: vscode.ExtensionContext,
    private lumiTesterPath: string,
    device?: Device
  ) {
    this._panel = panel;
    this._selectedDevice = device;
    this._outputChannel = vscode.window.createOutputChannel('Lumi Inspector');

    // Listen for when the panel is disposed
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

    // Handle messages from the webview
    this._panel.webview.onDidReceiveMessage(
      async (message) => {
        switch (message.command) {
          case 'insertSelector':
            this._insertSelectorToEditor(message.selector);
            break;
          case 'copySelector':
            await vscode.env.clipboard.writeText(message.selector);
            break;
        }
      },
      null,
      this._disposables
    );

    // Start inspector immediately
    this.startInspectorProcess();
  }

  public setDevice(device: Device) {
    this._selectedDevice = device;
    // Restart with new device
    this.startInspectorProcess();
  }

  private async startInspectorProcess() {
    this._stopInspector();

    // Find an available port
    this._port = await this._findAvailablePort(9333);

    const platform = this._selectedDevice?.platform || 'android';
    const deviceId = this._selectedDevice?.id;

    // Build command
    const args = ['run', '--', 'inspect', '--platform', platform, '--port', this._port.toString()];
    if (deviceId) {
      args.push('--device', deviceId);
    }

    this._outputChannel.appendLine(`Starting inspector on port ${this._port}...`);
    this._outputChannel.appendLine(`Command: cargo ${args.join(' ')}`);
    this._outputChannel.appendLine(`CWD: ${this.lumiTesterPath}`);

    try {
      this._panel.webview.html = this._getLoadingHtml("Starting Inspector Server...");

      this._inspectorProcess = child_process.spawn('cargo', args, {
        cwd: this.lumiTesterPath,
        shell: true,
        env: { ...process.env, RUST_BACKTRACE: '1' }
      });

      let hasExited = false;
      let exitCode: number | null = null;

      this._inspectorProcess.stdout?.on('data', (data) => {
        const msg = data.toString();
        this._outputChannel.append(msg);
        console.log(`Inspector: ${msg}`);
      });

      this._inspectorProcess.stderr?.on('data', (data) => {
        const msg = data.toString();
        this._outputChannel.append(msg);
        console.error(`Inspector error: ${msg}`);
      });

      this._inspectorProcess.on('exit', (code) => {
        hasExited = true;
        exitCode = code;
        this._outputChannel.appendLine(`Inspector process exited with code ${code}`);
      });

      this._inspectorProcess.on('error', (err) => {
        hasExited = true;
        this._outputChannel.appendLine(`Inspector process failed to spawn: ${err.message}`);
      });

      // Wait for port to be ready (timeout 60s for compilation)
      const portReady = await this._waitForPort(this._port, 60000, () => hasExited);

      if (hasExited) {
        vscode.window.showErrorMessage(`Inspector failed to start. Process exited with code ${exitCode}. Check "Lumi Inspector" output.`);
        this._outputChannel.show();
        return;
      }

      if (portReady) {
        this._outputChannel.appendLine('Inspector server is ready. Loading UI...');
        this._panel.webview.html = this._getInspectorHtml();
      } else {
        vscode.window.showErrorMessage('Inspector failed to start (port check timed out after 60s). Check "Lumi Inspector" output.');
        this._outputChannel.show();
      }

    } catch (error) {
      vscode.window.showErrorMessage(`Failed to start inspector: ${error}`);
    }
  }

  private _stopInspector() {
    if (this._inspectorProcess) {
      this._outputChannel.appendLine('Stopping inspector process...');
      this._inspectorProcess.kill();
      // Note: In some environments 'kill' might not kill the entire tree (cargo -> binary).
      // But we change ports anyway.
      this._inspectorProcess = undefined;
    }
  }

  private _insertSelectorToEditor(selector: string) {
    const editor = vscode.window.activeTextEditor;
    if (editor && editor.document.languageId === 'yaml') {
      const position = editor.selection.active;
      editor.edit(editBuilder => {
        editBuilder.insert(position, selector);
      });
    }
  }

  private _getLoadingHtml(message: string): string {
    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <style>
        body { background-color: var(--vscode-editor-background); color: var(--vscode-editor-foreground); font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; }
        .loader { border: 2px solid var(--vscode-editor-foreground); border-top: 2px solid transparent; border-radius: 50%; width: 20px; height: 20px; animation: spin 1s linear infinite; margin-right: 10px; }
        @keyframes spin { 0% { transform: rotate(0deg); } 100% { transform: rotate(360deg); } }
    </style>
</head>
<body>
    <div class="loader"></div>
    <div>${message}</div>
</body>
</html>`;
  }

  private _getInspectorHtml(): string {
    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Lumi Inspector</title>
    <style>
        body, html {
            margin: 0;
            padding: 0;
            width: 100%;
            height: 100%;
            overflow: hidden;
            background-color: var(--vscode-editor-background);
        }
        iframe {
            width: 100%;
            height: 100%;
            border: none;
            background-color: var(--vscode-editor-background);
        }
    </style>
</head>
<body>
    <iframe src="http://localhost:${this._port}" id="inspectorFrame"></iframe>
    <script>
        const vscode = acquireVsCodeApi();
        // Listen for messages from iframe
        window.addEventListener('message', (event) => {
            if (event.data) {
                if (event.data.type === 'insertSelector') {
                    vscode.postMessage({
                        command: 'insertSelector',
                        selector: event.data.value
                    });
                }
                if (event.data.type === 'copySelector') {
                    vscode.postMessage({
                        command: 'copySelector',
                        selector: event.data.value
                    });
                }
            }
        });
    </script>
</body>
</html>`;
  }

  public dispose() {
    InspectorPanel.currentPanel = undefined;
    this._stopInspector();
    this._panel.dispose();
    this._outputChannel.dispose();

    while (this._disposables.length) {
      const disposable = this._disposables.pop();
      if (disposable) {
        disposable.dispose();
      }
    }
  }

  // --- Helpers ---

  private _findAvailablePort(startPort: number): Promise<number> {
    return new Promise((resolve) => {
      const server = net.createServer();
      server.listen(startPort, () => {
        server.close(() => resolve(startPort));
      });
      server.on('error', () => {
        // Try next port
        resolve(this._findAvailablePort(startPort + 1));
      });
    });
  }

  private _waitForPort(port: number, timeoutMs = 60000, checkExit?: () => boolean): Promise<boolean> {
    const start = Date.now();
    return new Promise((resolve) => {
      const check = () => {
        if (checkExit && checkExit()) {
          resolve(false);
          return;
        }

        const socket = new net.Socket();
        socket.setTimeout(200);
        socket.on('connect', () => {
          socket.destroy();
          resolve(true);
        });
        socket.on('timeout', () => {
          socket.destroy();
          tryNext();
        });
        socket.on('error', () => {
          tryNext();
        });
        socket.connect(port, '127.0.0.1');
      };

      const tryNext = () => {
        if (Date.now() - start > timeoutMs) {
          resolve(false);
        } else {
          setTimeout(check, 500);
        }
      };

      check();
    });
  }
}
