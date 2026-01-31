import * as child_process from 'child_process';
import * as vscode from 'vscode';

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
        :root {
            --primary: #007acc;
            --bg: var(--vscode-editor-background);
            --fg: var(--vscode-editor-foreground);
            --border: var(--vscode-panel-border);
            --input-bg: var(--vscode-input-background);
            --input-fg: var(--vscode-input-foreground);
            --input-border: var(--vscode-input-border);
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, 'Open Sans', 'Helvetica Neue', sans-serif;
            background-color: var(--bg);
            color: var(--fg);
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100vh;
            margin: 0;
            padding: 20px;
        }
        .setup-card {
            background-color: var(--vscode-sideBar-background);
            border: 1px solid var(--border);
            border-radius: 8px;
            padding: 32px;
            width: 100%;
            max-width: 400px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.2);
        }
        h1 {
            margin: 0 0 24px 0;
            font-size: 20px;
            font-weight: 600;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 10px;
        }
        .form-group {
            margin-bottom: 20px;
            text-align: left;
        }
        label {
            display: block;
            margin-bottom: 8px;
            font-size: 12px;
            font-weight: 600;
            color: var(--vscode-descriptionForeground);
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }
        select, input {
            width: 100%;
            padding: 10px;
            border: 1px solid var(--input-border);
            background-color: var(--input-bg);
            color: var(--input-fg);
            border-radius: 6px;
            font-size: 13px;
            outline: none;
            box-sizing: border-box;
        }
        select:focus, input:focus {
            border-color: var(--primary);
        }
        button {
            background-color: var(--primary);
            color: white;
            border: none;
            padding: 12px;
            font-size: 14px;
            font-weight: 500;
            cursor: pointer;
            border-radius: 6px;
            width: 100%;
            margin-top: 10px;
            transition: opacity 0.2s;
        }
        button:hover {
            opacity: 0.9;
        }
        .features {
            margin-top: 30px;
            border-top: 1px solid var(--border);
            padding-top: 20px;
        }
        .feature-item {
            display: flex;
            align-items: center;
            gap: 10px;
            margin-bottom: 12px;
            font-size: 13px;
            color: var(--vscode-descriptionForeground);
        }
        .feature-icon {
            font-size: 16px;
            width: 24px;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="setup-card">
        <h1>🔍 Lumi Inspector</h1>
        
        <div class="form-group">
            <label for="platform">Platform</label>
            <select id="platform">
                <option value="android">Android</option>
                <option value="ios">iOS</option>
            </select>
        </div>
        
        <div class="form-group">
            <label for="device">Device ID</label>
            <input type="text" id="device" placeholder="Auto-detect first device">
        </div>
        
        <button onclick="startInspector()">Start Inspector</button>
        
        <div class="features">
            <div class="feature-item">
                <span class="feature-icon">🖥️</span> Live screen mirroring
            </div>
            <div class="feature-item">
                <span class="feature-icon">⚡</span> Smart selectors & commands
            </div>
            <div class="feature-item">
                <span class="feature-icon">⌨️</span> VSCode integration
            </div>
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
            background-color: #0d1117; /* Match Inspector Dark Mode */
        }
        .toolbar {
            background-color: #161b22;
            padding: 8px 16px;
            display: flex;
            align-items: center;
            gap: 12px;
            border-bottom: 1px solid #30363d;
            height: 40px;
            box-sizing: border-box;
        }
        .toolbar button {
            background-color: #21262d;
            color: #c9d1d9;
            border: 1px solid #30363d;
            padding: 4px 12px;
            cursor: pointer;
            border-radius: 6px;
            font-size: 12px;
            font-weight: 500;
            transition: all 0.2s;
            display: flex;
            align-items: center;
            gap: 6px;
        }
        .toolbar button:hover {
            background-color: #30363d;
            border-color: #8b949e;
            color: #fff;
        }
        .status {
            font-size: 12px;
            color: #8b949e;
            margin-left: auto;
            font-family: -apple-system, BlinkMacSystemFont, monospace;
        }
        .status.connected {
            color: #3fb950;
        }
        iframe {
            width: 100%;
            height: calc(100% - 40px);
            border: none;
            background-color: #0d1117;
        }
    </style>
</head>
<body>
    <div class="toolbar">
        <button onclick="refresh()">
            <span>↻</span> Refresh
        </button>
        <button onclick="stopInspector()" style="color: #f85149; border-color: rgba(248, 81, 73, 0.4);">
            <span>⏹</span> Stop
        </button>
        <span class="status connected">● Connected: ${this._port}</span>
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
                    selector: event.data.value
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
