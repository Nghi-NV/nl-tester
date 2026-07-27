# VS Code CLI-First Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship Lumi Tester VS Code extension `0.1.17` so Windows users with the native CLI can run tests, select Android devices, and open Inspector without Rust/Cargo.

**Architecture:** Add one pure runtime resolver for configured paths, PATH lookup, default CLI/ADB install locations, and source-development fallback. Route Run, Inspector, Device Manager, and diagnostics through explicit executable-plus-argument invocations with no shell. Package the extension through an isolated `extension-v*` GitHub workflow.

**Tech Stack:** TypeScript, VS Code Extension API, Node.js `child_process`/`fs`/`os`/`path`, Node test runner, `@vscode/vsce`, GitHub Actions.

---

## Scope and File Structure

- Create `lumi-tester-vscode/src/runtimeResolver.ts`: pure CLI/ADB resolution and path expansion.
- Create `lumi-tester-vscode/src/runtimeResolver.test.ts`: Windows and source fallback tests.
- Modify `lumi-tester-vscode/src/commandInvocation.ts`: build Run and Inspector commands from a resolved runtime.
- Modify `lumi-tester-vscode/src/commandInvocation.test.ts`: binary/source command tests.
- Create `lumi-tester-vscode/src/deviceDiscovery.ts`: pure `adb devices -l` parser.
- Create `lumi-tester-vscode/src/deviceDiscovery.test.ts`: physical/emulator/offline/unauthorized parser tests.
- Modify `lumi-tester-vscode/src/deviceManager.ts`: resolved ADB plus `execFile`.
- Modify `lumi-tester-vscode/src/inspectorPanel.ts`: resolved CLI invocation instead of Cargo.
- Modify `lumi-tester-vscode/src/extension.ts`: shared resolver, Run integration, and diagnostics command.
- Modify `lumi-tester-vscode/package.json` and `package-lock.json`: version, test list, command, and local VSCE dependency.
- Modify `lumi-tester-vscode/README.md`: CLI-first installation and troubleshooting.
- Modify `lumi-tester-vscode/.vscodeignore`: keep test JavaScript out of VSIX.
- Create `.github/workflows/vscode-extension.yml`: extension-only package/release workflow.

Do not modify any file under `lumi-tester/src`, `lumi-tester/e2e`, CLI installer scripts, or existing CLI workflows. Every Git command below stages explicit paths only.

### Task 1: Resolve Installed CLI and ADB Paths

**Files:**
- Create: `lumi-tester-vscode/src/runtimeResolver.ts`
- Create: `lumi-tester-vscode/src/runtimeResolver.test.ts`
- Modify: `lumi-tester-vscode/package.json:102-109`

- [ ] **Step 1: Add the resolver test file and exact test script**

Change the test script so every listed test runs identically on Windows and Unix:

```json
"test": "npm run compile && node --test out/commandInvocation.test.js out/runtimeResolver.test.js"
```

Create tests using injected filesystem/PATH dependencies:

```typescript
import * as assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  expandRuntimePath,
  resolveAdbExecutable,
  resolveLumiRuntime,
  RuntimeResolverOptions
} from './runtimeResolver';

function options(overrides: Partial<RuntimeResolverOptions> = {}): RuntimeResolverOptions {
  const files = new Set<string>();
  return {
    platform: 'win32',
    homeDir: 'C:\\Users\\QueDT',
    workspaceFolder: 'D:\\work\\mobile-tests',
    configuredPath: undefined,
    pathLookup: () => undefined,
    exists: value => files.has(value),
    isFile: value => files.has(value),
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
    expandRuntimePath('${userHome}\\.lumi-tester\\bin\\lumi-tester.exe', undefined, 'C:\\Users\\QueDT'),
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
```

- [ ] **Step 2: Add compile-only stubs and verify RED**

Create `runtimeResolver.ts` with exported types and functions that throw:

```typescript
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

export function expandRuntimePath(_value: string, _workspace: string | undefined, _home: string): string {
  throw new Error('Not implemented');
}

export function resolveLumiRuntime(_options: RuntimeResolverOptions): LumiRuntime {
  throw new Error('Not implemented');
}

export function resolveAdbExecutable(_options: RuntimeResolverOptions): string | undefined {
  throw new Error('Not implemented');
}
```

Run:

```bash
cd lumi-tester-vscode
npm test
```

Expected: resolver assertions fail with `Not implemented`.

- [ ] **Step 3: Implement variable expansion and deterministic resolution**

Implement the pure module:

```typescript
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
  const pathApi = /^[A-Za-z]:[\\/]/.test(expanded) || /^[A-Za-z]:[\\/]/.test(workspace ?? '')
    ? path.win32
    : path.posix;
  if (!pathApi.isAbsolute(expanded)) {
    if (!workspace) {
      throw new Error(`Relative lumiTesterPath requires an open workspace: ${expanded}`);
    }
    expanded = pathApi.resolve(workspace, expanded);
  }
  return pathApi.normalize(expanded);
}

function runtimeAt(candidate: string, options: RuntimeResolverOptions): LumiRuntime | undefined {
  if (!options.exists(candidate)) return undefined;
  if (options.isFile(candidate)) {
    return { kind: 'binary', executable: candidate, argsPrefix: [] };
  }
  if (options.isLumiSourceDirectory(candidate)) {
    return { kind: 'source', executable: 'cargo', argsPrefix: ['run', '--'], cwd: candidate };
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
      throw new Error(`lumi-tester.lumiTesterPath does not exist or is unsupported: ${configured}`);
    }
    return runtime;
  }

  const onPath = options.pathLookup('lumi-tester');
  if (onPath) {
    const runtime = runtimeAt(onPath, options);
    if (runtime) return runtime;
  }

  const binaryName = options.platform === 'win32' ? 'lumi-tester.exe' : 'lumi-tester';
  const installed = pathApi.join(options.homeDir, '.lumi-tester', 'bin', binaryName);
  const installedRuntime = runtimeAt(installed, options);
  if (installedRuntime) return installedRuntime;

  if (options.workspaceFolder) {
    for (const candidate of [pathApi.join(options.workspaceFolder, 'lumi-tester'), options.workspaceFolder]) {
      const runtime = runtimeAt(candidate, options);
      if (runtime) return runtime;
    }
  }
  throw new Error(
    `Could not find lumi-tester CLI. Checked PATH and ${installed}. `
    + 'Install it with the Lumi Tester PowerShell installer or configure lumi-tester.lumiTesterPath.'
  );
}

export function resolveAdbExecutable(options: RuntimeResolverOptions): string | undefined {
  const pathApi = options.platform === 'win32' ? path.win32 : path.posix;
  const onPath = options.pathLookup('adb');
  if (onPath && options.isFile(onPath)) return onPath;
  const name = options.platform === 'win32' ? 'adb.exe' : 'adb';
  const installed = pathApi.join(options.homeDir, '.lumi-tester', 'platform-tools', name);
  return options.isFile(installed) ? installed : undefined;
}
```

- [ ] **Step 4: Run resolver tests and verify GREEN**

```bash
cd lumi-tester-vscode
npm test
```

Expected: all runtime resolver tests pass.

- [ ] **Step 5: Commit only resolver files**

```bash
git add lumi-tester-vscode/src/runtimeResolver.ts lumi-tester-vscode/src/runtimeResolver.test.ts lumi-tester-vscode/package.json
git commit -m "🐛 fix(vscode): resolve installed lumi runtime"
```

### Task 2: Route Run Commands Through the Shared Runtime

**Files:**
- Modify: `lumi-tester-vscode/src/commandInvocation.ts`
- Modify: `lumi-tester-vscode/src/commandInvocation.test.ts`
- Modify: `lumi-tester-vscode/src/extension.ts:1-330`

- [ ] **Step 1: Replace invocation tests with runtime-based expectations**

Use the public `LumiRuntime` shape:

```typescript
import * as assert from 'node:assert/strict';
import { test } from 'node:test';
import { buildInspectInvocation, buildRunInvocation } from './commandInvocation';

test('runs a configured Windows executable directly', () => {
  const invocation = buildRunInvocation({
    runtime: {
      kind: 'binary',
      executable: 'C:\\Users\\QueDT\\.lumi-tester\\bin\\lumi-tester.exe',
      argsPrefix: []
    },
    testFilePath: 'C:\\tests\\login.yaml',
    commandIndex: 3,
    device: { platform: 'android', id: 'emulator-5554' }
  });
  assert.deepEqual(invocation.args, [
    'run', 'C:\\tests\\login.yaml', '--command-index', '3',
    '--platform', 'android', '--device', 'emulator-5554'
  ]);
  assert.equal(invocation.executable, 'C:\\Users\\QueDT\\.lumi-tester\\bin\\lumi-tester.exe');
});

test('keeps cargo only for a source runtime', () => {
  const invocation = buildRunInvocation({
    runtime: {
      kind: 'source',
      executable: 'cargo',
      argsPrefix: ['run', '--'],
      cwd: '/workspace/lumi-tester'
    },
    testFilePath: '/workspace/tests/login.yaml'
  });
  assert.deepEqual(invocation, {
    executable: 'cargo',
    args: ['run', '--', 'run', '/workspace/tests/login.yaml'],
    cwd: '/workspace/lumi-tester'
  });
});

test('builds inspector arguments for an installed binary', () => {
  const invocation = buildInspectInvocation({
    runtime: { kind: 'binary', executable: 'C:\\lumi-tester.exe', argsPrefix: [] },
    platform: 'android',
    port: 9333,
    deviceId: 'R5CT123'
  });
  assert.deepEqual(invocation.args, [
    'inspect', '--platform', 'android', '--port', '9333', '--device', 'R5CT123'
  ]);
});
```

- [ ] **Step 2: Run tests and verify RED**

```bash
cd lumi-tester-vscode
npm test
```

Expected: compile failure because the invocation API does not accept `runtime`
and `buildInspectInvocation` does not exist.

- [ ] **Step 3: Implement generic invocation construction**

```typescript
import { LumiRuntime } from './runtimeResolver';

export interface Invocation {
  executable: string;
  args: string[];
  cwd?: string;
}

function command(runtime: LumiRuntime, name: string, args: string[]): Invocation {
  return {
    executable: runtime.executable,
    args: [...runtime.argsPrefix, name, ...args],
    ...(runtime.cwd ? { cwd: runtime.cwd } : {})
  };
}

export function buildRunInvocation(options: {
  runtime: LumiRuntime;
  testFilePath: string;
  commandIndex?: number;
  device?: { platform: string; id: string };
}): Invocation {
  const args = [options.testFilePath];
  if (options.commandIndex !== undefined) {
    args.push('--command-index', options.commandIndex.toString());
  }
  if (options.device) {
    args.push('--platform', options.device.platform, '--device', options.device.id);
  }
  return command(options.runtime, 'run', args);
}

export function buildInspectInvocation(options: {
  runtime: LumiRuntime;
  platform: string;
  port: number;
  deviceId?: string;
}): Invocation {
  const args = ['--platform', options.platform, '--port', options.port.toString()];
  if (options.deviceId) args.push('--device', options.deviceId);
  return command(options.runtime, 'inspect', args);
}
```

- [ ] **Step 4: Add the VS Code adapter and replace `findLumiTesterPath`**

In `extension.ts`, add a production adapter around the pure resolver:

```typescript
import { execFileSync } from 'child_process';
import * as os from 'os';
import { LumiRuntime, resolveLumiRuntime, RuntimeResolverOptions } from './runtimeResolver';

function lookupCommand(name: string): string | undefined {
  const executable = process.platform === 'win32' ? 'where.exe' : 'which';
  try {
    return execFileSync(executable, [name], { encoding: 'utf8', windowsHide: true })
      .split(/\r?\n/)
      .map(value => value.trim())
      .find(Boolean);
  } catch {
    return undefined;
  }
}

function resolverOptions(uri: vscode.Uri): RuntimeResolverOptions {
  const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri)?.uri.fsPath
    ?? vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  return {
    platform: process.platform,
    homeDir: os.homedir(),
    workspaceFolder,
    configuredPath: vscode.workspace
      .getConfiguration('lumi-tester', uri)
      .get<string>('lumiTesterPath'),
    pathLookup: lookupCommand,
    exists: fs.existsSync,
    isFile: value => fs.existsSync(value) && fs.statSync(value).isFile(),
    isLumiSourceDirectory: value => {
      const manifest = path.join(value, 'Cargo.toml');
      return fs.existsSync(manifest)
        && fs.readFileSync(manifest, 'utf8').includes('lumi-tester');
    }
  };
}

function resolveRuntime(uri: vscode.Uri): LumiRuntime {
  return resolveLumiRuntime(resolverOptions(uri));
}
```

Make `runTestFile` and `runSingleCommand` resolve a runtime once, show an
actionable error when resolution throws, and pass `runtime` to
`buildRunInvocation`. Remove the unused `exec` import, unused PATH Promise,
`findLumiTesterPath`, and direct `statSync` classification.

- [ ] **Step 5: Run tests and compile**

```bash
cd lumi-tester-vscode
npm test
npm run compile
```

Expected: invocation tests pass and TypeScript compiles with no errors.

- [ ] **Step 6: Commit Run integration**

```bash
git add lumi-tester-vscode/src/commandInvocation.ts lumi-tester-vscode/src/commandInvocation.test.ts lumi-tester-vscode/src/extension.ts
git commit -m "🐛 fix(vscode): run tests through installed cli"
```

### Task 3: Discover Android Devices Without VS Code PATH State

**Files:**
- Create: `lumi-tester-vscode/src/deviceDiscovery.ts`
- Modify: `lumi-tester-vscode/src/deviceDiscovery.test.ts`
- Modify: `lumi-tester-vscode/src/deviceManager.ts:1-380`

- [ ] **Step 1: Add failing parser tests**

First extend the exact test script:

```json
"test": "npm run compile && node --test out/commandInvocation.test.js out/runtimeResolver.test.js out/deviceDiscovery.test.js"
```

Then create the parser tests:

```typescript
import * as assert from 'node:assert/strict';
import { test } from 'node:test';
import { parseAdbDevices } from './deviceDiscovery';

test('parses physical emulator offline and unauthorized Android devices', () => {
  const output = `List of devices attached
R5CT123 device product:dm3q model:SM_S918B device:dm3q
emulator-5554 device product:sdk model:sdk_gphone64_x86_64 device:emu64
ABC offline
LOCKED unauthorized usb:1-1
`;
  assert.deepEqual(parseAdbDevices(output), [
    { id: 'R5CT123', name: 'SM S918B', platform: 'android', state: 'device', type: 'physical' },
    { id: 'emulator-5554', name: 'sdk gphone64 x86 64', platform: 'android', state: 'device', type: 'emulator' },
    { id: 'ABC', name: 'ABC', platform: 'android', state: 'offline', type: 'physical' },
    { id: 'LOCKED', name: 'LOCKED', platform: 'android', state: 'unauthorized', type: 'physical' }
  ]);
});
```

- [ ] **Step 2: Run tests and verify RED**

```bash
cd lumi-tester-vscode
npm test
```

Expected: compile failure because `deviceDiscovery.ts` does not exist.

- [ ] **Step 3: Implement the pure parser**

```typescript
export interface AndroidDevice {
  id: string;
  name: string;
  platform: 'android';
  state: string;
  type: 'physical' | 'emulator';
}

export function parseAdbDevices(output: string): AndroidDevice[] {
  const devices: AndroidDevice[] = [];
  for (const line of output.split(/\r?\n/).slice(1)) {
    if (!line.trim()) continue;
    const match = line.match(/^(\S+)\s+(device|offline|unauthorized)(?:\s+.*?model:(\S+))?/);
    if (!match) continue;
    const id = match[1];
    devices.push({
      id,
      name: (match[3] ?? id).replace(/_/g, ' '),
      platform: 'android',
      state: match[2],
      type: id.startsWith('emulator-') ? 'emulator' : 'physical'
    });
  }
  return devices;
}
```

- [ ] **Step 4: Resolve ADB and use `execFile`**

Inject an optional resolver into `DeviceManager` while keeping the singleton API:

```typescript
import * as fs from 'fs';
import * as os from 'os';
import { execFileSync } from 'child_process';
import { parseAdbDevices } from './deviceDiscovery';
import { resolveAdbExecutable, RuntimeResolverOptions } from './runtimeResolver';

private resolveAdb(): string | undefined {
  const lookup = (name: string): string | undefined => {
    const executable = process.platform === 'win32' ? 'where.exe' : 'which';
    try {
      return execFileSync(executable, [name], { encoding: 'utf8', windowsHide: true })
        .split(/\r?\n/).map(value => value.trim()).find(Boolean);
    } catch { return undefined; }
  };
  const options: RuntimeResolverOptions = {
    platform: process.platform,
    homeDir: os.homedir(),
    pathLookup: lookup,
    exists: fs.existsSync,
    isFile: value => fs.existsSync(value) && fs.statSync(value).isFile(),
    isLumiSourceDirectory: () => false
  };
  return resolveAdbExecutable(options);
}
```

Replace `cp.exec('adb devices -l', ...)` with:

```typescript
const adb = this.resolveAdb();
if (!adb) {
  this.androidDiscoveryError = 'ADB not found in PATH or ~/.lumi-tester/platform-tools';
  resolve([]);
  return;
}
cp.execFile(adb, ['devices', '-l'], { timeout: 10000, windowsHide: true }, (error, stdout) => {
  if (error) {
    this.androidDiscoveryError = error.message;
    resolve([]);
    return;
  }
  this.androidDiscoveryError = undefined;
  resolve(parseAdbDevices(stdout));
});
```

When the device picker has only Web and `androidDiscoveryError` exists, show an
actionable warning containing the error:

```typescript
if (
  this.androidDiscoveryError
  && devices.every(device => device.platform === 'web')
) {
  void vscode.window.showWarningMessage(
    `Android devices unavailable: ${this.androidDiscoveryError}. `
    + 'Run “Lumi: Diagnose Setup” for resolved paths.'
  );
}
```

Leave the iOS `xcrun` code unchanged.

- [ ] **Step 5: Run tests and compile**

```bash
cd lumi-tester-vscode
npm test
npm run compile
```

Expected: parser tests pass and Device Manager compiles.

- [ ] **Step 6: Commit device discovery**

```bash
git add lumi-tester-vscode/src/deviceDiscovery.ts lumi-tester-vscode/src/deviceDiscovery.test.ts lumi-tester-vscode/src/deviceManager.ts lumi-tester-vscode/package.json
git commit -m "🐛 fix(vscode): find cli-managed adb"
```

### Task 4: Run Inspector Through the Installed CLI and Add Diagnostics

**Files:**
- Modify: `lumi-tester-vscode/src/inspectorPanel.ts:1-180`
- Modify: `lumi-tester-vscode/src/extension.ts:1-346`
- Modify: `lumi-tester-vscode/package.json:30-90`

- [ ] **Step 1: Wire Inspector to the tested invocation builder**

Change `InspectorPanel.show` and its constructor to accept `LumiRuntime`:

```typescript
public static async show(
  context: vscode.ExtensionContext,
  runtime: LumiRuntime,
  device?: Device
) { /* preserve panel creation */ }

private constructor(
  panel: vscode.WebviewPanel,
  private context: vscode.ExtensionContext,
  private runtime: LumiRuntime,
  device?: Device
) { /* preserve existing setup */ }
```

Build and spawn Inspector without a shell:

```typescript
const invocation = buildInspectInvocation({
  runtime: this.runtime,
  platform,
  port: this._port,
  deviceId
});
this._outputChannel.appendLine(
  `Command: ${invocation.executable} ${invocation.args.join(' ')}`
);
this._inspectorProcess = child_process.spawn(
  invocation.executable,
  invocation.args,
  {
    cwd: invocation.cwd,
    shell: false,
    windowsHide: true,
    env: { ...process.env, RUST_BACKTRACE: '1' }
  }
);
```

In `extension.ts`, resolve the runtime from the active YAML URI and pass it to
`InspectorPanel.show`. Preserve the existing port readiness and webview logic.

- [ ] **Step 2: Disconnect the unused Cargo-only runner**

Verify call sites:

```bash
rg -n "testRunner\.(runFile|runCommand)|new LumiTestRunner" lumi-tester-vscode/src
```

Expected: only construction exists; Run commands use VS Code Tasks. Remove the
`LumiTestRunner` import, instance, event subscriptions, and `testRunner.stop()`
call from `extension.ts`. Do not delete or refactor unrelated status-decoration
code.

- [ ] **Step 3: Add the diagnostics command**

Import `resolveAdbExecutable` and `parseAdbDevices` into `extension.ts` from the
pure modules created in Tasks 1 and 3.

Add to `package.json`:

```json
{
  "command": "lumi-tester.diagnoseSetup",
  "title": "Lumi: Diagnose Setup",
  "icon": "$(pulse)"
}
```

Register a command that resolves the CLI and ADB, then writes results to a
dedicated output channel. Use `execFile` with argument arrays:

```typescript
const diagnostics = vscode.window.createOutputChannel('Lumi Setup');
context.subscriptions.push(diagnostics);

context.subscriptions.push(vscode.commands.registerCommand(
  'lumi-tester.diagnoseSetup',
  async () => {
    diagnostics.clear();
    diagnostics.show(true);
    const uri = vscode.window.activeTextEditor?.document.uri
      ?? vscode.workspace.workspaceFolders?.[0]?.uri;
    if (!uri) {
      diagnostics.appendLine('No workspace or active file.');
      return;
    }
    try {
      const runtime = resolveRuntime(uri);
      diagnostics.appendLine(`CLI kind: ${runtime.kind}`);
      diagnostics.appendLine(`CLI path: ${runtime.executable}`);
      const version = execFileSync(
        runtime.executable,
        [...runtime.argsPrefix, '--version'],
        { encoding: 'utf8', cwd: runtime.cwd, windowsHide: true }
      );
      diagnostics.appendLine(`CLI version: ${version.trim()}`);
      const adb = resolveAdbExecutable(resolverOptions(uri));
      diagnostics.appendLine(`ADB path: ${adb ?? 'not found'}`);
      if (adb) {
        const adbOutput = execFileSync(
          adb,
          ['devices', '-l'],
          { encoding: 'utf8', windowsHide: true }
        );
        diagnostics.appendLine(
          `Android devices: ${parseAdbDevices(adbOutput).length}`
        );
      }
    } catch (error) {
      diagnostics.appendLine(`ERROR: ${error}`);
    }
  }
));
```

- [ ] **Step 4: Run tests and compile**

```bash
cd lumi-tester-vscode
npm test
npm run compile
```

Expected: all tests pass and Inspector/diagnostics compile without Cargo-only
call sites reachable from `extension.ts`.

- [ ] **Step 5: Commit Inspector and diagnostics**

```bash
git add lumi-tester-vscode/src/inspectorPanel.ts lumi-tester-vscode/src/extension.ts lumi-tester-vscode/package.json
git commit -m "🐛 fix(vscode): launch inspector through cli"
```

### Task 5: Package and Publish Extension 0.1.17 Independently

**Files:**
- Modify: `lumi-tester-vscode/package.json`
- Modify: `lumi-tester-vscode/package-lock.json`
- Modify: `lumi-tester-vscode/README.md`
- Modify: `lumi-tester-vscode/.vscodeignore`
- Create: `.github/workflows/vscode-extension.yml`

- [ ] **Step 1: Bump version and install local VSCE dependency**

Run:

```bash
cd lumi-tester-vscode
npm version 0.1.17 --no-git-tag-version
npm install --save-dev @vscode/vsce
```

Expected: package and lockfile both report `0.1.17`; `@vscode/vsce` is a local
dev dependency, so CI does not depend on a globally installed command.

- [ ] **Step 2: Update README with the no-Rust workflow**

Document this Windows sequence exactly:

```powershell
iwr https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.ps1 -UseB | iex
code --install-extension lumi-tester-0.1.17.vsix
```

State that no setting is required for the default install path. Add an optional
absolute-path override and explicitly warn against unresolved variables:

```json
{
  "lumi-tester.lumiTesterPath": "C:\\Users\\QueDT\\.lumi-tester\\bin\\lumi-tester.exe"
}
```

- [ ] **Step 3: Add the isolated workflow**

Create `.github/workflows/vscode-extension.yml`:

```yaml
name: VS Code Extension

on:
  workflow_dispatch:
  push:
    branches: [main]
    tags: ["extension-v*"]
    paths:
      - "lumi-tester-vscode/**"
      - ".github/workflows/vscode-extension.yml"

permissions:
  contents: write

jobs:
  package:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: lumi-tester-vscode
    steps:
      - uses: actions/checkout@v6
      - uses: actions/setup-node@v6
        with:
          node-version: "22"
          cache: npm
          cache-dependency-path: lumi-tester-vscode/package-lock.json
      - run: npm ci
      - run: npm test
      - run: npm run package
      - name: Verify extension tag matches package version
        if: startsWith(github.ref, 'refs/tags/extension-v')
        shell: bash
        run: |
          version="$(node -p "require('./package.json').version")"
          test "${GITHUB_REF_NAME}" = "extension-v${version}"
      - uses: actions/upload-artifact@v7
        with:
          name: lumi-tester-vsix
          path: lumi-tester-vscode/lumi-tester-*.vsix
      - name: Attach VSIX to extension release
        if: startsWith(github.ref, 'refs/tags/extension-v')
        uses: softprops/action-gh-release@v3
        with:
          tag_name: ${{ github.ref_name }}
          files: lumi-tester-vscode/lumi-tester-*.vsix
          fail_on_unmatched_files: true
```

- [ ] **Step 4: Package locally and inspect contents**

```bash
cd lumi-tester-vscode
npm ci
npm test
npm run package
unzip -l lumi-tester-0.1.17.vsix
```

Expected: package succeeds; VSIX contains compiled runtime/device/invocation
JavaScript and excludes `src/**`, `*.map`, and `*.test.js`.

- [ ] **Step 5: Commit packaging and workflow only**

```bash
git add lumi-tester-vscode/package.json lumi-tester-vscode/package-lock.json lumi-tester-vscode/README.md lumi-tester-vscode/.vscodeignore .github/workflows/vscode-extension.yml
git commit -m "👷 ci(vscode): package extension releases"
```

### Task 6: Final Scope and Windows Verification

**Files:**
- Verify only; do not perform unrelated cleanup.

- [ ] **Step 1: Run full extension verification**

```bash
cd lumi-tester-vscode
npm ci
npm test
npm run compile
npm run package
```

Expected: all tests pass, compilation succeeds, and
`lumi-tester-0.1.17.vsix` is created.

- [ ] **Step 2: Prove no installed-extension path requires Cargo**

```bash
rg -n "cargo|shell: true|\$\{workspaceFolder\}" lumi-tester-vscode/src
```

Expected: Cargo appears only in the explicit source-runtime fallback; no Run or
Inspector code hardcodes Cargo; no shell execution remains for CLI/ADB.

- [ ] **Step 3: Verify Git scope**

```bash
git diff --check
git status --short
git log --oneline -8
```

Expected: commits contain only `lumi-tester-vscode/**` and
`.github/workflows/vscode-extension.yml`; pre-existing Rust/YAML working changes
remain uncommitted and untouched.

- [ ] **Step 4: Perform Windows smoke verification**

On a Windows machine with the CLI and ADB installed but without Rust/Cargo:

```powershell
code --install-extension .\lumi-tester-0.1.17.vsix --force
where.exe lumi-tester
adb devices -l
```

Then reload VS Code and verify:

1. `Lumi: Diagnose Setup` reports binary runtime and ADB paths.
2. Run File succeeds.
3. A CodeLens Run Command succeeds.
4. Android device picker lists the same connected device as `adb devices -l`.
5. Inspector starts and loads its webview.
6. Stop terminates the active Run task.

If Windows access is unavailable, report offline package/test evidence and do
not claim these six runtime checks passed.
