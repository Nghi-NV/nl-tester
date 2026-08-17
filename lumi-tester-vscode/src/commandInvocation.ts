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

export function buildRunInvocation(options: RunInvocationOptions): Invocation {
  const args = [options.testFilePath];
  if (options.commandIndex !== undefined) {
    args.push('--command-index', options.commandIndex.toString());
  } else if (options.fromCommandIndex !== undefined) {
    args.push('--from-command-index', options.fromCommandIndex.toString());
  }
  if (options.device) {
    args.push('--platform', options.device.platform, '--device', options.device.id);
  }
  return buildCommand(options.runtime, 'run', args);
}

export function buildInspectInvocation(options: InspectInvocationOptions): Invocation {
  const args = ['--platform', options.platform, '--port', options.port.toString()];
  if (options.deviceId) {
    args.push('--device', options.deviceId);
  }
  return buildCommand(options.runtime, 'inspect', args);
}
