import { LumiRuntime } from './runtimeResolver';

export interface Invocation {
  executable: string;
  args: string[];
  cwd?: string;
}

export interface RunInvocationOptions {
  runtime: LumiRuntime;
  testFilePath: string;
  commandIndex?: number;
  fromCommandIndex?: number;
  targetPlatform?: string;
  device?: {
    platform: string;
    id: string;
  };
}

export interface InspectInvocationOptions {
  runtime: LumiRuntime;
  platform: string;
  port: number;
  deviceId?: string;
}

function buildCommand(runtime: LumiRuntime, name: string, args: string[]): Invocation {
  return {
    executable: runtime.executable,
    args: [...runtime.argsPrefix, name, ...args],
    ...(runtime.cwd ? { cwd: runtime.cwd } : {})
  };
}

export function parseYamlPlatform(filePath: string): string | undefined {
  try {
    const fs = require('fs');
    if (fs.existsSync(filePath)) {
      const content = fs.readFileSync(filePath, 'utf8');
      const match = content.match(/^platform:\s*([a-zA-Z0-9_-]+)/m);
      if (match) {
        return match[1].trim().toLowerCase();
      }
    }
  } catch (e) {
    // ignore
  }
  return undefined;
}

export function buildRunInvocation(options: RunInvocationOptions): Invocation {
  // Always request report generation (JSON, report.html, summary.html, JUnit) -
  // the CLI treats `--report` as opt-in (so scripted/CI callers that just want
  // pass/fail don't pay the extra write cost), but a run started interactively
  // from the editor should always leave a report behind: that's the whole reason
  // someone runs a test from a YAML file rather than a headless CI job. Without
  // this, "Run All" completed with no error but also silently produced none of
  // report.html / summary.html / index.html.
  const args = [options.testFilePath, '--report'];
  if (options.commandIndex !== undefined) {
    args.push('--command-index', options.commandIndex.toString());
  } else if (options.fromCommandIndex !== undefined) {
    args.push('--from-command-index', options.fromCommandIndex.toString());
  }
  
  const platform = (options.targetPlatform || parseYamlPlatform(options.testFilePath))?.toLowerCase();
  const isDesktopOrWeb = platform === 'macos' || platform === 'windows' || platform === 'desktop' || platform === 'web';

  if (isDesktopOrWeb) {
    // For desktop/web tests, platform is determined by the test YAML header, do not override with mobile device
  } else if (options.device && (!platform || platform === options.device.platform.toLowerCase())) {
    args.push('--platform', options.device.platform, '--device', options.device.id);
  }
  return buildCommand(options.runtime, 'run', args);
}

export function buildInspectInvocation(options: InspectInvocationOptions): Invocation {
  const args = ['--platform', options.platform, '--port', options.port.toString()];
  if (options.deviceId && options.deviceId !== 'macos' && options.deviceId !== 'chrome') {
    args.push('--device', options.deviceId);
  }
  return buildCommand(options.runtime, 'inspect', args);
}
