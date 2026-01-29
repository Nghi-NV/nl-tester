import * as vscode from 'vscode';
import * as path from 'path';
import * as child_process from 'child_process';
import { DeviceManager } from './deviceManager';

export class InspectorPanel {
  public static currentPanel: InspectorPanel | undefined;
  private static readonly viewType = 'lumiInspector';

  private readonly _panel: vscode.WebviewPanel;
  private _inspectorProcess: child_process.ChildProcess | undefined;
  private _port: number = 9333;
  private _disposables: vscode.Disposable[] = [];

  public static async show(context: vscode.ExtensionContext, lumiTesterPath: string) {
    const column = vscode.ViewColumn.Beside;

    // If we already have a panel, show it
    if (InspectorPanel.currentPanel) {
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

    InspectorPanel.currentPanel = new InspectorPanel(panel, context, lumiTesterPath);
  }

  private constructor(
    panel: vscode.WebviewPanel,
    private context: vscode.ExtensionContext,
    private lumiTesterPath: string
  ) {
    this._panel = panel;

    // Set the webview's initial html content
    this._update();

    // Listen for when the panel is disposed
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

    // Handle messages from the webview
    this._panel.webview.onDidReceiveMessage(
      async (message) => {
        switch (message.command) {
          case 'startInspector':
            await this._startInspector(message.platform, message.device);
            break;
          case 'stopInspector':
            this._stopInspector();
            break;
          case 'insertSelector':
            this._insertSelectorToEditor(message.selector);
            break;
        }
      },
      null,
      this._disposables
    );
  }

  private async _startInspector(platform: string, device?: string) {
    // Stop existing process if any
    this._stopInspector();

    // Find an available port
    this._port = 9333 + Math.floor(Math.random() * 100);

    // Build command
    const args = ['run', '--', 'inspect', '--platform', platform, '--port', this._port.toString()];
    if (device) {
      args.push('--device', device);
    }

    try {
      this._inspectorProcess = child_process.spawn('cargo', args, {
        cwd: this.lumiTesterPath,
        shell: true
      });

      this._inspectorProcess.stdout?.on('data', (data) => {
        console.log(`Inspector: ${data}`);
      });

      this._inspectorProcess.stderr?.on('data', (data) => {
        console.error(`Inspector error: ${data}`);
      });

      // Wait a bit for server to start
      await new Promise(resolve => setTimeout(resolve, 2000));

      // Update webview with iframe pointing to inspector
      this._panel.webview.html = this._getInspectorHtml();

    } catch (error) {
      vscode.window.showErrorMessage(`Failed to start inspector: ${error}`);
    }
  }

  private _stopInspector() {
    if (this._inspectorProcess) {
      this._inspectorProcess.kill();
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

  private _update() {
    this._panel.webview.html = this._getSetupHtml();
  }

  private _getSetupHtml(): string {
    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Lumi Inspector</title>
    <style>
        body {
            font-family: var(--vscode-font-family);
            background-color: var(--vscode-editor-background);
            color: var(--vscode-editor-foreground);
            padding: 20px;
            margin: 0;
        }
        .setup-container {
            max-width: 400px;
            margin: 0 auto;
            text-align: center;
        }
        h1 {
            color: var(--vscode-textLink-foreground);
            margin-bottom: 30px;
        }
        .form-group {
            margin-bottom: 20px;
            text-align: left;
        }
        label {
            display: block;
            margin-bottom: 8px;
            font-weight: bold;
        }
        select, input {
            width: 100%;
            padding: 10px;
            border: 1px solid var(--vscode-input-border);
            background-color: var(--vscode-input-background);
            color: var(--vscode-input-foreground);
            border-radius: 4px;
            font-size: 14px;
        }
        button {
            background-color: var(--vscode-button-background);
            color: var(--vscode-button-foreground);
            border: none;
            padding: 12px 24px;
            font-size: 14px;
            cursor: pointer;
            border-radius: 4px;
            width: 100%;
            margin-top: 10px;
        }
        button:hover {
            background-color: var(--vscode-button-hoverBackground);
        }
        .info {
            margin-top: 30px;
            padding: 15px;
            background-color: var(--vscode-textBlockQuote-background);
            border-radius: 4px;
            text-align: left;
            font-size: 13px;
        }
        .info h3 {
            margin-top: 0;
        }
        .info ul {
            padding-left: 20px;
            margin-bottom: 0;
        }
    </style>
</head>
<body>
    <div class="setup-container">
        <h1>🔍 Lumi Inspector</h1>
        
        <div class="form-group">
            <label for="platform">Platform</label>
            <select id="platform">
                <option value="android">Android</option>
                <option value="ios">iOS</option>
            </select>
        </div>
        
        <div class="form-group">
            <label for="device">Device ID (optional)</label>
            <input type="text" id="device" placeholder="Auto-detect first device">
        </div>
        
        <button onclick="startInspector()">▶️ Start Inspector</button>
        
        <div class="info">
            <h3>Features:</h3>
            <ul>
                <li>🖥️ Live screen mirroring</li>
                <li>🖱️ Click elements to get selectors</li>
                <li>📝 Right-click to add commands</li>
                <li>🔍 Smart selector suggestions</li>
            </ul>
        </div>
    </div>
    
    <script>
        const vscode = acquireVsCodeApi();
        
        function startInspector() {
            const platform = document.getElementById('platform').value;
            const device = document.getElementById('device').value || undefined;
            
            vscode.postMessage({
                command: 'startInspector',
                platform: platform,
                device: device
            });
        }
    </script>
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
        }
        .toolbar {
            background-color: var(--vscode-editor-background);
            padding: 8px 12px;
            display: flex;
            align-items: center;
            gap: 10px;
            border-bottom: 1px solid var(--vscode-panel-border);
        }
        .toolbar button {
            background-color: var(--vscode-button-background);
            color: var(--vscode-button-foreground);
            border: none;
            padding: 6px 12px;
            cursor: pointer;
            border-radius: 3px;
            font-size: 12px;
        }
        .toolbar button:hover {
            background-color: var(--vscode-button-hoverBackground);
        }
        .toolbar .status {
            color: var(--vscode-descriptionForeground);
            font-size: 12px;
            margin-left: auto;
        }
        .toolbar .status.connected {
            color: #4EC9B0;
        }
        iframe {
            width: 100%;
            height: calc(100% - 45px);
            border: none;
        }
    </style>
</head>
<body>
    <div class="toolbar">
        <button onclick="refresh()">🔄 Refresh</button>
        <button onclick="stopInspector()">⏹️ Stop</button>
        <span class="status connected">● Connected to localhost:${this._port}</span>
    </div>
    <iframe src="http://localhost:${this._port}" id="inspectorFrame"></iframe>
    
    <script>
        const vscode = acquireVsCodeApi();
        
        function refresh() {
            document.getElementById('inspectorFrame').src = 'http://localhost:${this._port}';
        }
        
        function stopInspector() {
            vscode.postMessage({ command: 'stopInspector' });
        }
        
        // Listen for messages from iframe (for selector insertion)
        window.addEventListener('message', (event) => {
            if (event.data && event.data.type === 'insertSelector') {
                vscode.postMessage({
                    command: 'insertSelector',
                    selector: event.data.selector
                });
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

    while (this._disposables.length) {
      const disposable = this._disposables.pop();
      if (disposable) {
        disposable.dispose();
      }
    }
  }
}
