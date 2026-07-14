import * as assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  expandRuntimePath,
  resolveAdbExecutable,
  resolveLumiRuntime,
  RuntimeResolverOptions
} from './runtimeResolver';

function options(overrides: Partial<RuntimeResolverOptions> = {}): RuntimeResolverOptions {
  return {
    platform: 'win32',
    homeDir: 'C:\\Users\\QueDT',
    workspaceFolder: 'D:\\work\\mobile-tests',
    configuredPath: undefined,
    pathLookup: () => undefined,
    exists: () => false,
    isFile: () => false,
    isLumiSourceDirectory: () => false,
    ...overrides
  };
}

test('expands workspace and home variables before filesystem access', () => {
  assert.equal(
    expandRuntimePath(
      '${workspaceFolder}\\tools\\lumi-tester.exe',
      'D:\\work\\mobile-tests',
      'C:\\Users\\QueDT'
    ),
    'D:\\work\\mobile-tests\\tools\\lumi-tester.exe'
  );
  assert.equal(
    expandRuntimePath(
      '${userHome}\\.lumi-tester\\bin\\lumi-tester.exe',
      undefined,
      'C:\\Users\\QueDT'
    ),
    'C:\\Users\\QueDT\\.lumi-tester\\bin\\lumi-tester.exe'
  );
});

test('rejects an unresolved configured variable', () => {
  assert.throws(
    () => expandRuntimePath('${unknown}\\lumi-tester.exe', 'D:\\work', 'C:\\Users\\QueDT'),
    /Unresolved variable: \$\{unknown\}/
  );
});

test('resolves relative configured paths against workspace', () => {
  const binary = 'D:\\work\\mobile-tests\\tools\\lumi-tester.exe';
  const runtime = resolveLumiRuntime(options({
    configuredPath: 'tools\\lumi-tester.exe',
    exists: value => value === binary,
    isFile: value => value === binary
  }));

  assert.deepEqual(runtime, { kind: 'binary', executable: binary, argsPrefix: [] });
});

test('keeps an absolute Windows UNC configured path', () => {
  assert.equal(
    expandRuntimePath(
      '\\\\server\\share\\lumi-tester.exe',
      undefined,
      'C:\\Users\\QueDT'
    ),
    '\\\\server\\share\\lumi-tester.exe'
  );
});

test('uses PATH before the default Windows install location', () => {
  const binary = 'C:\\tools\\lumi-tester.exe';
  const runtime = resolveLumiRuntime(options({
    pathLookup: name => name === 'lumi-tester' ? binary : undefined,
    exists: value => value === binary,
    isFile: value => value === binary
  }));

  assert.equal(runtime.executable, binary);
});

test('finds the default Windows CLI and ADB installs without PATH', () => {
  const cli = 'C:\\Users\\QueDT\\.lumi-tester\\bin\\lumi-tester.exe';
  const adb = 'C:\\Users\\QueDT\\.lumi-tester\\platform-tools\\adb.exe';
  const resolver = options({
    exists: value => value === cli || value === adb,
    isFile: value => value === cli || value === adb
  });

  assert.equal(resolveLumiRuntime(resolver).executable, cli);
  assert.equal(resolveAdbExecutable(resolver), adb);
});

test('uses a workspace source directory only as the final fallback', () => {
  const sourceDir = 'D:\\work\\mobile-tests\\lumi-tester';
  const runtime = resolveLumiRuntime(options({
    exists: value => value === sourceDir,
    isLumiSourceDirectory: value => value === sourceDir
  }));

  assert.deepEqual(runtime, {
    kind: 'source',
    executable: 'cargo',
    argsPrefix: ['run', '--'],
    cwd: sourceDir
  });
});

test('does not silently ignore an invalid explicit setting', () => {
  assert.throws(
    () => resolveLumiRuntime(options({
      configuredPath: '${workspaceFolder}\\missing.exe',
      pathLookup: () => 'C:\\tools\\lumi-tester.exe'
    })),
    /lumi-tester\.lumiTesterPath does not exist/
  );
});
