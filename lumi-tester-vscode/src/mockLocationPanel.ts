import * as vscode from 'vscode';

/**
 * Mock Location Control Panel
 * Shows a webview panel with speed controls when GPS mock is active
 */
export class MockLocationPanel {
  public static currentPanel: MockLocationPanel | undefined;
  private readonly _panel: vscode.WebviewPanel;
  private _disposables: vscode.Disposable[] = [];
  private _currentSpeed: number = 60;
  private _isPaused: boolean = false;
  private _lumiPath: string;
  private _onSpeedChange: ((speed: number) => void) | undefined;
  private _onPauseResume: ((paused: boolean) => void) | undefined;

  private constructor(
    panel: vscode.WebviewPanel,
    lumiPath: string,
    initialSpeed: number = 60,
    onSpeedChange?: (speed: number) => void,
    onPauseResume?: (paused: boolean) => void
  ) {
    this._panel = panel;
    this._lumiPath = lumiPath;
    this._currentSpeed = initialSpeed;  // Set speed BEFORE generating HTML
    this._onSpeedChange = onSpeedChange;
    this._onPauseResume = onPauseResume;

    this._panel.webview.html = this._getHtmlContent();

    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

    this._panel.webview.onDidReceiveMessage(
      async (message) => {
        switch (message.command) {
          case 'setSpeed':
            this._currentSpeed = message.speed;
            await this._sendControlCommand('speed', message.speed);
            break;
          case 'setSpeedMode':
            await this._sendControlCommand('speedMode', undefined, message.mode);
            break;
          case 'pause':
            this._isPaused = true;
            await this._sendControlCommand('pause');
            break;
          case 'resume':
            this._isPaused = false;
            await this._sendControlCommand('resume');
            break;
        }
      },
      null,
      this._disposables
    );
  }

  public static show(
    context: vscode.ExtensionContext,
    lumiPath: string,
    initialSpeed: number = 60,
    onSpeedChange?: (speed: number) => void,
    onPauseResume?: (paused: boolean) => void
  ): MockLocationPanel {
    const column = vscode.ViewColumn.Beside;

    if (MockLocationPanel.currentPanel) {
      MockLocationPanel.currentPanel._panel.reveal(column);
      MockLocationPanel.currentPanel.updateSpeed(initialSpeed);
      return MockLocationPanel.currentPanel;
    }

    const panel = vscode.window.createWebviewPanel(
      'mockLocationControl',
      '🛰️ GPS Speed Control',
      { viewColumn: column, preserveFocus: true },
      {
        enableScripts: true,
        retainContextWhenHidden: true
      }
    );

    MockLocationPanel.currentPanel = new MockLocationPanel(
      panel,
      lumiPath,
      initialSpeed,
      onSpeedChange,
      onPauseResume
    );

    return MockLocationPanel.currentPanel;
  }

  public static hide(): void {
    if (MockLocationPanel.currentPanel) {
      MockLocationPanel.currentPanel.dispose();
    }
  }

  public updateSpeed(speed: number): void {
    this._currentSpeed = speed;
    this._panel.webview.postMessage({ command: 'updateSpeed', speed });
  }

  public updatePauseState(isPaused: boolean): void {
    this._isPaused = isPaused;
    this._panel.webview.postMessage({ command: 'updatePauseState', isPaused });
  }

  private async _sendControlCommand(action: string, value?: number, mode?: string): Promise<void> {
    const fs = require('fs');
    const controlPath = '/tmp/lumi-gps-control.json';

    let controlData: { speed?: number; paused?: boolean; speedMode?: string } = {};

    if (action === 'speed' && value !== undefined) {
      controlData.speed = value;
      this._onSpeedChange?.(value);
    } else if (action === 'speedMode' && mode) {
      controlData.speedMode = mode;
    } else if (action === 'pause') {
      controlData.paused = true;
      this._onPauseResume?.(true);
    } else if (action === 'resume') {
      controlData.paused = false;
      this._onPauseResume?.(false);
    }

    try {
      fs.writeFileSync(controlPath, JSON.stringify(controlData));
    } catch (e) {
      console.error('Failed to write GPS control file:', e);
    }
  }

  private _getHtmlContent(): string {
    return /* html */ `
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>GPS Speed Control</title>
  <style>
    :root {
      --vscode-font: var(--vscode-font-family, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif);
      --bg-primary: var(--vscode-editor-background, #1e1e1e);
      --bg-secondary: var(--vscode-sideBar-background, #252526);
      --text-primary: var(--vscode-editor-foreground, #cccccc);
      --text-secondary: var(--vscode-descriptionForeground, #8c8c8c);
      --accent: var(--vscode-button-background, #0e639c);
      --accent-hover: var(--vscode-button-hoverBackground, #1177bb);
      --border: var(--vscode-panel-border, #3c3c3c);
      --success: #4caf50;
      --warning: #ff9800;
    }

    * {
      margin: 0;
      padding: 0;
      box-sizing: border-box;
    }

    body {
      font-family: var(--vscode-font);
      background: var(--bg-primary);
      color: var(--text-primary);
      padding: 12px;
    }

    .container {
      max-width: 320px;
      margin: 0 auto;
    }

    .header {
      text-align: center;
      margin-bottom: 12px;
    }

    .header h1 {
      font-size: 18px;
      margin-bottom: 4px;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 6px;
    }

    .header p {
      color: var(--text-secondary);
      font-size: 11px;
    }

    .speed-display {
      background: var(--bg-secondary);
      border-radius: 10px;
      padding: 14px;
      text-align: center;
      margin-bottom: 12px;
      border: 1px solid var(--border);
    }

    .speed-value {
      font-size: 42px;
      font-weight: 700;
      color: var(--accent);
      line-height: 1;
      margin-bottom: 2px;
    }

    .speed-unit {
      font-size: 11px;
      color: var(--text-secondary);
      text-transform: uppercase;
      letter-spacing: 1px;
    }

    .slider-container {
      margin-bottom: 24px;
    }

    .slider-label {
      display: flex;
      justify-content: space-between;
      margin-bottom: 8px;
      font-size: 13px;
      color: var(--text-secondary);
    }

    input[type="range"] {
      width: 100%;
      height: 8px;
      border-radius: 4px;
      background: var(--bg-secondary);
      outline: none;
      -webkit-appearance: none;
    }

    input[type="range"]::-webkit-slider-thumb {
      -webkit-appearance: none;
      width: 24px;
      height: 24px;
      border-radius: 50%;
      background: var(--accent);
      cursor: pointer;
      border: 3px solid var(--bg-primary);
      box-shadow: 0 2px 6px rgba(0,0,0,0.3);
      transition: transform 0.15s ease;
    }

    input[type="range"]::-webkit-slider-thumb:hover {
      transform: scale(1.15);
    }

    .presets {
      display: grid;
      grid-template-columns: repeat(4, 1fr);
      gap: 8px;
      margin-bottom: 24px;
    }

    .preset-btn {
      padding: 12px 8px;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--bg-secondary);
      color: var(--text-primary);
      cursor: pointer;
      transition: all 0.15s ease;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 4px;
    }

    .preset-btn:hover {
      background: var(--accent);
      border-color: var(--accent);
    }

    .preset-btn.active {
      background: var(--accent);
      border-color: var(--accent);
    }

    .preset-icon {
      font-size: 20px;
    }

    .preset-label {
      font-size: 11px;
      color: var(--text-secondary);
    }

    .preset-btn:hover .preset-label,
    .preset-btn.active .preset-label {
      color: var(--text-primary);
    }

    .preset-speed {
      font-size: 12px;
      font-weight: 600;
    }

    .controls {
      display: flex;
      gap: 12px;
    }

    .control-btn {
      flex: 1;
      padding: 14px;
      border: none;
      border-radius: 10px;
      font-size: 14px;
      font-weight: 600;
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      transition: all 0.15s ease;
    }

    .control-btn.pause {
      background: var(--warning);
      color: #000;
    }

    .control-btn.resume {
      background: var(--success);
      color: #fff;
    }

    .control-btn:hover {
      transform: translateY(-2px);
      box-shadow: 0 4px 12px rgba(0,0,0,0.3);
    }

    .status {
      margin-top: 16px;
      padding: 12px;
      background: var(--bg-secondary);
      border-radius: 8px;
      text-align: center;
      font-size: 13px;
      color: var(--text-secondary);
    }

    .status.active {
      color: var(--success);
    }

    .status.paused {
      color: var(--warning);
    }

    .speed-adjust {
      display: flex;
      gap: 8px;
      margin-bottom: 16px;
    }

    .adjust-btn {
      flex: 1;
      padding: 12px;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--bg-secondary);
      color: var(--text-primary);
      font-size: 16px;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.15s ease;
    }

    .adjust-btn:hover {
      background: var(--accent);
      border-color: var(--accent);
    }

    .mode-toggle {
      display: flex;
      gap: 8px;
      margin-bottom: 16px;
    }

    .mode-btn {
      flex: 1;
      padding: 10px;
      border: 1px solid var(--border);
      border-radius: 8px;
      background: var(--bg-secondary);
      color: var(--text-primary);
      font-size: 12px;
      cursor: pointer;
      transition: all 0.15s ease;
    }

    .mode-btn.active {
      background: var(--accent);
      border-color: var(--accent);
    }

    .mode-btn:hover {
      border-color: var(--accent);
    }
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <h1>🛰️ GPS Control</h1>
      <p>Adjust mock location speed in real-time</p>
    </div>

    <div class="speed-display">
      <div class="speed-value" id="speedValue">${this._currentSpeed}</div>
      <div class="speed-unit">km/h</div>
    </div>

    <div class="slider-container">
      <div class="slider-label">
        <span>0</span>
        <span>Speed</span>
        <span>200</span>
      </div>
      <input type="range" id="speedSlider" min="0" max="200" value="${this._currentSpeed}" />
    </div>

    <div class="speed-adjust">
      <button class="adjust-btn" id="decreaseBtn">-5</button>
      <button class="adjust-btn" id="increaseBtn">+5</button>
    </div>

    <div class="mode-toggle">
      <button class="mode-btn active" id="linearBtn" data-mode="linear">📈 Linear</button>
      <button class="mode-btn" id="noiseBtn" data-mode="noise">🎲 Noise</button>
    </div>

    <div class="presets">
      <button class="preset-btn" data-speed="5">
        <span class="preset-icon">🚶</span>
        <span class="preset-label">Walk</span>
        <span class="preset-speed">5</span>
      </button>
      <button class="preset-btn" data-speed="20">
        <span class="preset-icon">🚴</span>
        <span class="preset-label">Cycle</span>
        <span class="preset-speed">20</span>
      </button>
      <button class="preset-btn" data-speed="60">
        <span class="preset-icon">🚗</span>
        <span class="preset-label">Drive</span>
        <span class="preset-speed">60</span>
      </button>
      <button class="preset-btn" data-speed="120">
        <span class="preset-icon">🏎️</span>
        <span class="preset-label">Fast</span>
        <span class="preset-speed">120</span>
      </button>
    </div>

    <div class="controls">
      <button class="control-btn pause" id="pauseBtn">
        ⏸️ Pause
      </button>
      <button class="control-btn resume" id="resumeBtn" style="display: none;">
        ▶️ Resume
      </button>
    </div>

    <div class="status ${this._isPaused ? 'paused' : 'active'}" id="status">
      ${this._isPaused ? '⏸️ GPS Paused' : '🛰️ GPS Active'}
    </div>
  </div>

  <script>
    const vscode = acquireVsCodeApi();
    const speedSlider = document.getElementById('speedSlider');
    const speedValue = document.getElementById('speedValue');
    const pauseBtn = document.getElementById('pauseBtn');
    const resumeBtn = document.getElementById('resumeBtn');
    const status = document.getElementById('status');
    const presetBtns = document.querySelectorAll('.preset-btn');

    let debounceTimer;

    function updateSpeed(speed) {
      speedValue.textContent = speed;
      speedSlider.value = speed;
      updatePresetHighlight(speed);
    }

    function updatePresetHighlight(speed) {
      presetBtns.forEach(btn => {
        btn.classList.toggle('active', parseInt(btn.dataset.speed) === speed);
      });
    }

    speedSlider.addEventListener('input', (e) => {
      const speed = parseInt(e.target.value);
      speedValue.textContent = speed;
      updatePresetHighlight(speed);

      clearTimeout(debounceTimer);
      debounceTimer = setTimeout(() => {
        vscode.postMessage({ command: 'setSpeed', speed });
      }, 150);
    });

    presetBtns.forEach(btn => {
      btn.addEventListener('click', () => {
        const speed = parseInt(btn.dataset.speed);
        updateSpeed(speed);
        vscode.postMessage({ command: 'setSpeed', speed });
      });
    });

    // +5/-5 buttons
    const increaseBtn = document.getElementById('increaseBtn');
    const decreaseBtn = document.getElementById('decreaseBtn');
    
    increaseBtn.addEventListener('click', () => {
      const newSpeed = Math.min(200, parseInt(speedSlider.value) + 5);
      updateSpeed(newSpeed);
      vscode.postMessage({ command: 'setSpeed', speed: newSpeed });
    });

    decreaseBtn.addEventListener('click', () => {
      const newSpeed = Math.max(0, parseInt(speedSlider.value) - 5);
      updateSpeed(newSpeed);
      vscode.postMessage({ command: 'setSpeed', speed: newSpeed });
    });

    // Speed mode toggle
    const linearBtn = document.getElementById('linearBtn');
    const noiseBtn = document.getElementById('noiseBtn');
    
    linearBtn.addEventListener('click', () => {
      linearBtn.classList.add('active');
      noiseBtn.classList.remove('active');
      vscode.postMessage({ command: 'setSpeedMode', mode: 'linear' });
    });

    noiseBtn.addEventListener('click', () => {
      noiseBtn.classList.add('active');
      linearBtn.classList.remove('active');
      vscode.postMessage({ command: 'setSpeedMode', mode: 'noise' });
    });

    pauseBtn.addEventListener('click', () => {
      pauseBtn.style.display = 'none';
      resumeBtn.style.display = 'flex';
      status.textContent = '⏸️ GPS Paused';
      status.className = 'status paused';
      vscode.postMessage({ command: 'pause' });
    });

    resumeBtn.addEventListener('click', () => {
      resumeBtn.style.display = 'none';
      pauseBtn.style.display = 'flex';
      status.textContent = '🛰️ GPS Active';
      status.className = 'status active';
      vscode.postMessage({ command: 'resume' });
    });

    window.addEventListener('message', (event) => {
      const message = event.data;
      switch (message.command) {
        case 'updateSpeed':
          updateSpeed(message.speed);
          break;
        case 'updatePauseState':
          if (message.isPaused) {
            pauseBtn.style.display = 'none';
            resumeBtn.style.display = 'flex';
            status.textContent = '⏸️ GPS Paused';
            status.className = 'status paused';
          } else {
            resumeBtn.style.display = 'none';
            pauseBtn.style.display = 'flex';
            status.textContent = '🛰️ GPS Active';
            status.className = 'status active';
          }
          break;
      }
    });

    // Initial preset highlight
    updatePresetHighlight(${this._currentSpeed});
  </script>
</body>
</html>
    `;
  }

  public dispose(): void {
    MockLocationPanel.currentPanel = undefined;
    this._panel.dispose();
    while (this._disposables.length) {
      const d = this._disposables.pop();
      if (d) {
        d.dispose();
      }
    }
  }
}
