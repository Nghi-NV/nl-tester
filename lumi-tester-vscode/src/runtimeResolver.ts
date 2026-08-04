import * as path from 'path';

export type RuntimeKind = 'binary' | 'source';

export interface LumiRuntime {
  kind: RuntimeKind;
  executable: string;
  argsPrefix: string[];
  cwd?: string;
}

export interface RuntimeResolverOptions {
  platform: NodeJS.Platform;
  homeDir: string;
  workspaceFolder?: string;
  configuredPath?: string;
  pathLookup(name: string): string | undefined;
  exists(value: string): boolean;
  isFile(value: string): boolean;
  isLumiSourceDirectory(value: string): boolean;
}

export function expandRuntimePath(value: string, workspace: string | undefined, home: string): string {
  let expanded = value.trim().split('${userHome}').join(home);
  if (expanded.includes('${workspaceFolder}')) {
    if (!workspace) {
      throw new Error('Cannot resolve ${workspaceFolder}: no workspace is open');
    }
    expanded = expanded.split('${workspaceFolder}').join(workspace);
  }

  const unresolved = expanded.match(/\$\{[^}]+\}/)?.[0];
  if (unresolved) {
    throw new Error(`Unresolved variable: ${unresolved}`);
  }

  const usesWindowsPaths = /^[A-Za-z]:[\\/]/.test(expanded)
    || /^\\\\/.test(expanded)
    || /^[A-Za-z]:[\\/]/.test(workspace ?? '')
    || /^\\\\/.test(workspace ?? '');
  const pathApi = usesWindowsPaths ? path.win32 : path.posix;
  if (!pathApi.isAbsolute(expanded)) {
    if (!workspace) {
      throw new Error(`Relative lumiTesterPath requires an open workspace: ${expanded}`);
    }
    expanded = pathApi.resolve(workspace, expanded);
  }
  return pathApi.normalize(expanded);
}

function runtimeAt(candidate: string, options: RuntimeResolverOptions): LumiRuntime | undefined {
  if (!options.exists(candidate)) {
    return undefined;
  }
  if (options.isFile(candidate)) {
    return { kind: 'binary', executable: candidate, argsPrefix: [] };
  }
  if (options.isLumiSourceDirectory(candidate)) {
    return {
      kind: 'source',
      executable: 'cargo',
      argsPrefix: ['run', '--'],
      cwd: candidate
    };
  }
  return undefined;
}

export function resolveLumiRuntime(options: RuntimeResolverOptions): LumiRuntime {
  const pathApi = options.platform === 'win32' ? path.win32 : path.posix;

  if (options.configuredPath?.trim()) {
    const configured = expandRuntimePath(
      options.configuredPath,
      options.workspaceFolder,
      options.homeDir
    );
    const runtime = runtimeAt(configured, options);
    if (!runtime) {
      throw new Error(
        `lumi-tester.lumiTesterPath does not exist or is unsupported: ${configured}`
      );
    }
    return runtime;
  }

  const onPath = options.pathLookup('lumi-tester');
  if (onPath) {
    const runtime = runtimeAt(onPath, options);
    if (runtime) {
      return runtime;
    }
  }

  const binaryName = options.platform === 'win32' ? 'lumi-tester.exe' : 'lumi-tester';
  const installedCandidates = [
    pathApi.join(options.homeDir, '.lumi-tester', 'bin', binaryName),
    pathApi.join(options.homeDir, '.local', 'bin', binaryName),
    pathApi.join(options.homeDir, '.cargo', 'bin', binaryName)
  ];
  for (const installed of installedCandidates) {
    const installedRuntime = runtimeAt(installed, options);
    if (installedRuntime) {
      return installedRuntime;
    }
  }

  if (options.workspaceFolder) {
    const candidates = [
      pathApi.join(options.workspaceFolder, 'lumi-tester'),
      options.workspaceFolder
    ];
    for (const candidate of candidates) {
      const runtime = runtimeAt(candidate, options);
      if (runtime) {
        return runtime;
      }
    }
  }

  throw new Error(
    `Could not find lumi-tester CLI. Checked PATH and ${installedCandidates.join(', ')}. `
    + 'Install it with the Lumi Tester PowerShell installer or configure '
    + 'lumi-tester.lumiTesterPath.'
  );
}

export function resolveAdbExecutable(options: RuntimeResolverOptions): string | undefined {
  const onPath = options.pathLookup('adb');
  if (onPath && options.isFile(onPath)) {
    return onPath;
  }

  const pathApi = options.platform === 'win32' ? path.win32 : path.posix;
  const binaryName = options.platform === 'win32' ? 'adb.exe' : 'adb';
  const installed = pathApi.join(
    options.homeDir,
    '.lumi-tester',
    'platform-tools',
    binaryName
  );
  return options.isFile(installed) ? installed : undefined;
}
