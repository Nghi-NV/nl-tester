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
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg: #0d1117;
            --panel: #161b22;
            --card: #21262d;
            --accent: #58a6ff;
            --green: #3fb950;
            --text: #c9d1d9;
            --muted: #8b949e;
            --border: #30363d;
            --input-bg: #0d1117;
            --shadow: 0 8px 24px rgba(0, 0, 0, 0.5);
        }
        * { box-sizing: border-box; }
        body {
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, sans-serif;
            background-color: var(--bg);
            color: var(--text);
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            height: 100vh;
            margin: 0;
            padding: 20px;
        }
        .logo {
            font-size: 32px;
            margin-bottom: 24px;
            display: flex;
            align-items: center;
            gap: 12px;
            color: #fff;
            font-weight: 600;
        }
        .setup-card {
            background-color: var(--panel);
            border: 1px solid var(--border);
            border-radius: 12px;
            padding: 32px;
            width: 100%;
            max-width: 380px;
            box-shadow: var(--shadow);
        }
        .form-group {
            margin-bottom: 20px;
        }
        label {
            display: block;
            margin-bottom: 8px;
            font-size: 12px;
            font-weight: 600;
            color: var(--muted);
            text-transform: uppercase;
            letter-spacing: 0.5px;
        }
        select, input {
            width: 100%;
            padding: 10px 12px;
            border: 1px solid var(--border);
            background-color: var(--input-bg);
            color: #fff;
            border-radius: 6px;
            font-size: 13px;
            font-family: inherit;
            outline: none;
            transition: border-color 0.2s;
        }
        select:focus, input:focus {
            border-color: var(--accent);
        }
        button {
            background-color: var(--accent);
            color: #050505;
            border: none;
            padding: 12px;
            font-size: 14px;
            font-weight: 600;
            cursor: pointer;
            border-radius: 6px;
            width: 100%;
            margin-top: 12px;
            transition: all 0.2s;
        }
        button:hover {
            opacity: 0.9;
            transform: translateY(-1px);
        }
        .features {
            margin-top: 24px;
            padding-top: 20px;
            border-top: 1px solid var(--border);
        }
        .feature-item {
            display: flex;
            align-items: center;
            gap: 12px;
            margin-bottom: 12px;
            font-size: 13px;
            color: var(--muted);
        }
        .feature-icon {
            color: var(--accent);
            width: 20px;
            text-align: center;
        }
    </style>
</head>
<body>
    <div class="logo">
        🔍 Lumi Inspector
    </div>
    <div class="setup-card">
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
                <span class="feature-icon">⚡</span> Smart selector detection
            </div>
            <div class="feature-item">
                <span class="feature-icon">🪄</span> Auto-generate commands
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
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600&display=swap" rel="stylesheet">
    <style>
        :root {
            --bg: #0d1117;
            --panel: #161b22;
            --border: #30363d;
            --text: #c9d1d9;
            --accent: #58a6ff;
            --red: #f85149;
        }
        body, html {
            margin: 0;
            padding: 0;
            width: 100%;
            height: 100%;
            overflow: hidden;
            background-color: var(--bg);
            font-family: 'Inter', sans-serif;
        }
        .toolbar {
            background-color: var(--panel);
            padding: 8px 16px;
            display: flex;
            align-items: center;
            gap: 12px;
            border-bottom: 1px solid var(--border);
            height: 48px;
            box-sizing: border-box;
        }
        .toolbar button {
            background-color: transparent;
            color: var(--text);
            border: 1px solid var(--border);
            padding: 6px 12px;
            cursor: pointer;
            border-radius: 6px;
            font-size: 13px;
            font-weight: 500;
            display: flex;
            align-items: center;
            gap: 8px;
            transition: all 0.2s;
            font-family: inherit;
        }
        .toolbar button:hover {
            background-color: #21262d;
            border-color: #8b949e;
            color: #fff;
        }
        .toolbar button.stop-btn {
            color: var(--red);
            border-color: rgba(248, 81, 73, 0.4);
        }
        .toolbar button.stop-btn:hover {
            background-color: rgba(248, 81, 73, 0.1);
            border-color: var(--red);
        }
        .status {
            font-size: 12px;
            color: var(--muted);
            margin-left: auto;
            display: flex;
            align-items: center;
            gap: 6px;
        }
        .status-dot {
            width: 8px;
            height: 8px;
            background-color: #3fb950;
            border-radius: 50%;
        }
        iframe {
            width: 100%;
            height: calc(100% - 48px);
            border: none;
            background-color: var(--bg);
        }
    </style>
</head>
<body>
    <div class="toolbar">
        <button onclick="refresh()">
            <span>↻</span> Refresh
        </button>
        <button onclick="stopInspector()" class="stop-btn">
            <span>⏹</span> Stop Server
        </button>
        <div class="status">
            <span class="status-dot"></span>
            Connected to port ${this._port}
        </div>
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
