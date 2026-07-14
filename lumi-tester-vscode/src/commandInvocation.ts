export interface RunInvocationOptions {
  lumiPath: string;
  lumiPathIsFile: boolean;
  testFilePath: string;
  commandIndex?: number;
  device?: {
    platform: string;
    id: string;
  };
}

export interface RunInvocation {
  executable: string;
  args: string[];
  cwd?: string;
}

export function buildRunInvocation(options: RunInvocationOptions): RunInvocation {
  const args = ['run', options.testFilePath];

  if (options.commandIndex !== undefined) {
    args.push('--command-index', options.commandIndex.toString());
  }
  if (options.device) {
    args.push('--platform', options.device.platform, '--device', options.device.id);
  }

  if (options.lumiPathIsFile) {
    return {
      executable: options.lumiPath,
      args
    };
  }

  return {
    executable: 'cargo',
    args: ['run', '--', ...args],
    cwd: options.lumiPath
  };
}
