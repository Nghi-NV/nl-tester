import * as vscode from 'vscode';
import * as cp from 'child_process';
import * as path from 'path';
import { EventEmitter } from 'events';
import { TestStatus, CommandStatus } from './decorationProvider';

export class LumiTestRunner extends EventEmitter {
  private currentProcess: cp.ChildProcess | null = null;
  private outputChannel: vscode.OutputChannel;

  constructor() {
    super();
    this.outputChannel = vscode.window.createOutputChannel('Lumi Tester');
  }

  public async runFile(filePath: string): Promise<void> {
    return this.run(filePath);
  }

  public async runCommand(filePath: string, commandIndex: number): Promise<void> {
    return this.run(filePath, commandIndex);
  }

  private async run(filePath: string, commandIndex?: number): Promise<void> {
    return new Promise((resolve, reject) => {
      this.outputChannel.show(true);
      this.outputChannel.appendLine(`\n${'='.repeat(60)}`);
      this.outputChannel.appendLine(`▶ Running: ${path.basename(filePath)}`);
      this.outputChannel.appendLine(`${'='.repeat(60)}\n`);

      // Find lumi-tester path
      const lumiPath = this.findLumiTesterPath(filePath);
      if (!lumiPath) {
        reject(new Error('Could not find lumi-tester. Please set lumi-tester.lumiTesterPath in settings.'));
        return;
      }

      // Build command
      const args = ['run', '--', 'run', filePath];
      if (commandIndex !== undefined) {
        args.push('--command-index', commandIndex.toString());
      }

      this.outputChannel.appendLine(`> cargo ${args.join(' ')}\n`);

      // Initialize status
      const status: TestStatus = {
        filePath,
        commandStatuses: []
      };

      // Spawn process
      this.currentProcess = cp.spawn('cargo', args, {
        cwd: lumiPath,
        shell: true
      });

      let currentCommandIndex = 0;

      this.currentProcess.stdout?.on('data', (data: Buffer) => {
        const output = data.toString();
        this.outputChannel.append(output);

        // Parse output for status updates
        const lines = output.split('\n');
        for (const line of lines) {
          // Match: "    ⠼ [0] launchApp..." (running)
          const runningMatch = line.match(/\[(\d+)\]\s+(\w+).*\.\.\.$/);
          if (runningMatch) {
            const index = parseInt(runningMatch[1], 10);
            status.commandStatuses[index] = {
              index,
              status: 'running',
              message: `Running: ${runningMatch[2]}`
            };
            this.emit('statusChange', status);
          }

          // Match: "    ✓ [0] launchApp... (2395ms)" (passed)
          const passedMatch = line.match(/✓\s+\[(\d+)\]\s+(\w+).*\((\d+)ms\)/);
          if (passedMatch) {
            const index = parseInt(passedMatch[1], 10);
            status.commandStatuses[index] = {
              index,
              status: 'passed',
              message: `Passed: ${passedMatch[2]}`,
              duration: parseInt(passedMatch[3], 10)
            };
            this.emit('statusChange', status);
            currentCommandIndex = index + 1;
          }

          // Match: "    ❌ [1] tapOn..." (failed)
          const failedMatch = line.match(/❌\s+\[(\d+)\]\s+(\w+)/);
          if (failedMatch) {
            const index = parseInt(failedMatch[1], 10);
            status.commandStatuses[index] = {
              index,
              status: 'failed',
              message: `Failed: ${failedMatch[2]}`
            };
            this.emit('statusChange', status);
          }

          // Match: "  📍 Loaded X GPS points from file" (mockLocation started)
          const gpsStartMatch = line.match(/📍\s+Loaded\s+(\d+)\s+GPS\s+points/i);
          if (gpsStartMatch) {
            const pointCount = parseInt(gpsStartMatch[1], 10);
            this.emit('mockLocationStarted', { pointCount });
          }

          // Match: GPS stop patterns
          const gpsStopMatch = line.match(/(?:stopGps|stopMockLocation|GPS\s+stopped)/i);
          if (gpsStopMatch) {
            this.emit('mockLocationStopped');
          }
        }
      });

      this.currentProcess.stderr?.on('data', (data: Buffer) => {
        const output = data.toString();
        // Filter out Rust warnings for cleaner output
        if (!output.includes('warning:') && !output.includes('Compiling')) {
          this.outputChannel.append(output);
        }
      });

      this.currentProcess.on('close', (code) => {
        this.currentProcess = null;
        this.outputChannel.appendLine(`\n${'='.repeat(60)}`);

        if (code === 0) {
          this.outputChannel.appendLine('✅ Test completed successfully!');
          resolve();
        } else {
          this.outputChannel.appendLine(`❌ Test failed with exit code: ${code}`);
          reject(new Error(`Test failed with exit code: ${code}`));
        }
      });

      this.currentProcess.on('error', (error) => {
        this.currentProcess = null;
        this.outputChannel.appendLine(`\n❌ Error: ${error.message}`);
        reject(error);
      });
    });
  }

  private findLumiTesterPath(testFilePath: string): string | null {
    // Check configuration first
    const config = vscode.workspace.getConfiguration('lumi-tester');
    const configPath = config.get<string>('lumiTesterPath');
    if (configPath && configPath.length > 0) {
      return configPath;
    }

    // Try to find lumi-tester in parent directories
    let dir = path.dirname(testFilePath);
    for (let i = 0; i < 10; i++) {
      const cargoPath = path.join(dir, 'Cargo.toml');
      try {
        const fs = require('fs');
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
          const fs = require('fs');
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

  public stop(): void {
    if (this.currentProcess) {
      this.currentProcess.kill('SIGTERM');
      this.currentProcess = null;
      this.outputChannel.appendLine('\n⚠ Test stopped by user');
    }
  }

  public onStatusChange(callback: (status: TestStatus) => void): void {
    this.on('statusChange', callback);
  }

  public onMockLocationStarted(callback: (data: { pointCount: number }) => void): void {
    this.on('mockLocationStarted', callback);
  }

  public onMockLocationStopped(callback: () => void): void {
    this.on('mockLocationStopped', callback);
  }
}
