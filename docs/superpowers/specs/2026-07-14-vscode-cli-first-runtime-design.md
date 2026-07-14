# VS Code CLI-First Runtime and Publishing Design

## Goal

Allow Windows users who already installed the native `lumi-tester` CLI to
install the Lumi Tester VSIX and use Run, Android device selection, and
Inspector without installing Rust, Cargo, Node.js, or the source repository.

## Scope Guard

Implementation is limited to:

- `lumi-tester-vscode/src/`
- `lumi-tester-vscode/package.json`, tests, README, and packaging excludes
- One dedicated VS Code extension workflow under `.github/workflows/`

It must not modify:

- Rust runtime code under `lumi-tester/src/`
- CLI installers or GitHub CLI release assets
- Camera, parser, executor, report, or YAML behavior
- Existing non-extension workflows
- Existing user changes outside `lumi-tester-vscode`

The CLI binary remains separately installed. This change only makes the
extension discover and use that binary consistently.

## User Experience

The supported Windows setup is:

1. Install the native Lumi Tester CLI with the existing PowerShell installer.
2. Install `lumi-tester-0.1.17.vsix` or install the same version from the VS
   Code Marketplace later.
3. Open a YAML test and use Run, device selection, or Inspector without
   configuring Rust/Cargo.

No extension setting is required when the CLI is installed at the default
location or available through `PATH`.

If discovery fails, the extension reports every checked location and offers an
action to open the `lumi-tester.lumiTesterPath` setting.

## Runtime Resolution

### One resolver owns all paths

Create a focused runtime module that resolves the CLI and Android ADB paths.
Run, Inspector, Device Manager, and any retained runner code must use this
module instead of implementing separate lookup behavior.

CLI lookup order:

1. `lumi-tester.lumiTesterPath` after variable expansion.
2. `where.exe lumi-tester` on Windows or `which lumi-tester` elsewhere.
3. `%USERPROFILE%\.lumi-tester\bin\lumi-tester.exe` on Windows, or the
   equivalent home-directory install path on macOS/Linux.
4. A workspace `lumi-tester/Cargo.toml` source directory as a development-only
   fallback.

The resolver returns an explicit runtime kind:

- `binary`: execute `lumi-tester` directly.
- `source`: execute `cargo run --` from the source directory.

Production behavior never chooses Cargo when an installed binary exists.

### Configuration variables

The configured path supports:

- `${workspaceFolder}` resolved from the workspace containing the active YAML.
- `${userHome}` resolved from `os.homedir()`.

After expansion, any remaining `${...}` token is an error. Relative configured
paths resolve against the active workspace, never against the VS Code process
directory. The resolver validates that a configured path exists before
returning it.

### Command construction

All command arguments remain arrays rather than shell strings.

Run commands:

```text
binary: lumi-tester.exe run <yaml> [--command-index N] [device arguments]
source: cargo run -- run <yaml> [--command-index N] [device arguments]
```

Inspector commands:

```text
binary: lumi-tester.exe inspect --platform <platform> --port <port> [--device id]
source: cargo run -- inspect --platform <platform> --port <port> [--device id]
```

Use `ProcessExecution` for Run and `child_process.spawn` with `shell: false` for
Inspector. Paths and device IDs are never interpolated into shell commands.

## Android Device Discovery

The extension continues parsing `adb devices -l`, but resolves the ADB
executable explicitly.

ADB lookup order:

1. `where.exe adb` on Windows or `which adb` elsewhere.
2. `%USERPROFILE%\.lumi-tester\platform-tools\adb.exe` on Windows, or the
   equivalent CLI-managed install location elsewhere.

Device Manager uses `execFile(adbPath, ["devices", "-l"])` with no shell. It
does not require the integrated terminal or VS Code process to inherit a newly
updated PATH.

iOS discovery remains unchanged because this Windows-focused fix must not alter
macOS/Xcode behavior.

## Inspector

Inspector receives the resolved Lumi runtime instead of a raw path. It builds
the `inspect` invocation through the same command builder used by Run.

The existing port readiness, webview, output channel, and process cleanup logic
remain intact. Only executable selection, arguments, working directory, and log
text change.

## Legacy Runner

`LumiTestRunner` currently contains a Cargo-only run path even though CodeLens
uses VS Code Tasks. To prevent regression, either:

- route it through the shared runtime and invocation builder, or
- remove its unused run methods while retaining only events actually consumed
  by the extension.

The implementation chooses the smaller change after verifying call sites. It
must leave no executable Cargo-only path reachable in installed-extension use.

## Diagnostics

Add `Lumi: Diagnose Setup`. It reports:

- resolved CLI kind and path
- CLI `--version` result
- resolved ADB path
- `adb devices -l` success and detected device count
- whether Rust/Cargo fallback was selected

The command must redact no secrets because it prints only local executable
paths and version/device summaries. It does not modify installation state.

## Publishing

### Version and artifact

Bump the extension from `0.1.16` to `0.1.17`. The package must contain compiled
JavaScript and resources, but exclude TypeScript tests and source maps according
to `.vscodeignore`.

### Dedicated workflow

Add one extension-only workflow that:

1. Runs on changes to `lumi-tester-vscode/**`, manual dispatch, and tags matching
   `extension-v*`.
2. Installs Node.js and runs `npm ci`, tests, compile, and `vsce package`.
3. Uploads the VSIX as a GitHub Actions artifact for every workflow run.
4. Attaches the VSIX to a GitHub Release only for an `extension-v*` tag.

This workflow does not rebuild or republish the Rust CLI and does not change the
existing CLI release workflow.

Marketplace publishing is a follow-up. VS Code currently supports automated
publishing through `vsce`; Microsoft recommends secure automated credentials
and is retiring global Azure DevOps PATs on December 1, 2026. The immediate
`0.1.17` deliverable therefore uses a downloadable signed-by-GitHub-release
VSIX without adding a new long-lived repository secret.

Official references:

- https://code.visualstudio.com/api/working-with-extensions/publishing-extension
- https://code.visualstudio.com/api/working-with-extensions/continuous-integration

## Error Handling

- A missing configured path reports the expanded path and setting name.
- An unresolved variable reports the exact variable token.
- Missing CLI reports all lookup locations and the existing PowerShell install
  command.
- Missing ADB does not crash activation; Device Manager shows an actionable
  warning and still lists the Web target.
- Inspector spawn errors show the resolved executable and output-channel name.
- Runtime discovery never falls back from an invalid explicit setting to a
  different binary silently; explicit configuration errors must be corrected.

## Testing

All production behavior changes use red-green TDD.

Unit tests cover:

- `${workspaceFolder}` and `${userHome}` expansion on Windows-style paths
- rejection of unresolved variables
- relative path resolution against the active workspace
- CLI lookup precedence and the default Windows install location
- binary and source Run invocations
- binary and source Inspector invocations
- ADB PATH and CLI-managed fallback precedence
- `adb devices -l` parsing with physical, emulator, offline, and unauthorized
  devices

Integration verification covers:

- `npm test`
- `npm run compile`
- `npm run package`
- inspection of VSIX contents to ensure tests and source maps are excluded
- installation of the produced VSIX on a Windows machine with native CLI and
  ADB but no Rust/Cargo
- Run File, Run Command, Android device selection, Stop, and Inspector smoke
  tests

## Acceptance Criteria

- A literal `${workspaceFolder}` never reaches `fs.statSync` or process spawn.
- With the CLI at `%USERPROFILE%\.lumi-tester\bin\lumi-tester.exe`, no setting is
  required.
- Run File and Run Command work on Windows without Rust/Cargo.
- Android devices visible in `adb devices` appear in Device Manager even when
  VS Code has stale PATH state.
- Inspector runs through `lumi-tester.exe inspect` without Cargo.
- Missing dependencies produce actionable messages instead of silent empty
  device lists.
- The extension package is version `0.1.17` and a dedicated workflow produces a
  downloadable VSIX.
- No files outside the declared extension/publish scope are changed by the
  implementation.
