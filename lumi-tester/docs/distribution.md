# Distributing lumi-tester

`lumi-tester` is packaged as native CLI binaries for the main desktop platforms:

- `lumi-tester-x86_64-unknown-linux-gnu`
- `lumi-tester-aarch64-unknown-linux-gnu`
- `lumi-tester-x86_64-apple-darwin`
- `lumi-tester-aarch64-apple-darwin`
- `lumi-tester-x86_64-pc-windows-msvc.exe`
- `lumi-tester-aarch64-pc-windows-msvc.exe`

## Release

Create a Git tag from the repository root:

```bash
git tag v0.1.6
git push origin v0.1.6
```

The GitHub Actions release workflow builds all targets, uploads the binaries, publishes `SHA256SUMS`, includes install scripts, and generates Homebrew/Scoop/Winget manifest files as release assets.

## AI Pack Install

Use this path when the target machine should be ready for AI-assisted Lumi test authoring, execution, and debugging.

For Homebrew users, the simplest flow is:

```bash
brew install nghi-nv/tap/lumi-tester
lumi-tester ai install
```

macOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install-ai.sh | bash
```

Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install-ai.ps1 -UseB | iex
```

The AI installer:

- Installs the `lumi-tester` CLI.
- Downloads the matching `lumi-tester-mcp-<target>.tgz` release asset.
- Installs the Codex skill into `$CODEX_HOME/skills/lumi-tester-agent`.
- Writes MCP config snippets under `$HOME/.lumi-tester/ai`.
- Adds a Codex MCP server entry unless `LUMI_AI_CONFIGURE_CODEX=0`.

Requirements:

- `node` and `npm`.
- Restart the AI client after install.
- Android/iOS/Web device dependencies as needed.

Pin a release:

```bash
curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install-ai.sh | LUMI_TESTER_VERSION=v0.1.6 bash
lumi-tester ai install --version v0.1.6
```

## One-line Install

macOS and Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.sh | bash
```

Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.ps1 -UseB | iex
```

Install a specific version:

```bash
curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.sh | LUMI_TESTER_VERSION=v0.1.6 bash
```

Skip `system install --all`:

```bash
curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.sh | LUMI_SKIP_SYSTEM_INSTALL=1 bash
```

After installation, users can run:

```bash
lumi-tester --version
lumi-tester system install --all
```

## Package Managers

Package-manager distribution should wrap the same release assets.

The release workflow generates package-manager manifests. The package-manager workflow publishes Homebrew and Scoop manifests to:

- `Nghi-NV/homebrew-tap` at `Formula/lumi-tester.rb`
- `Nghi-NV/scoop-bucket` at `bucket/lumi-tester.json`

Set repository secret `PACKAGE_MANAGER_TOKEN` with `repo` scope so the workflow can create/update those repositories. Override target repositories with GitHub Actions variables:

- `HOMEBREW_TAP_REPO`
- `SCOOP_BUCKET_REPO`

### Homebrew

Expected UX:

```bash
brew install nghi-nv/tap/lumi-tester
lumi-tester ai install
```

Formula behavior:

- Download the correct release tar/binary for macOS.
- Install `lumi-tester` into Homebrew's `bin`.
- Do not run `system install --all` automatically; print caveats telling users to run it.
- Users who want Codex/MCP support run `lumi-tester ai install`.

### Scoop

Expected UX:

```powershell
scoop bucket add Nghi-NV https://github.com/Nghi-NV/scoop-bucket.git
scoop install lumi-tester
```

Manifest behavior:

- Download the Windows release asset.
- Install as `lumi-tester.exe`.
- Include `bin` entry for PATH integration.

### Winget

Expected UX:

```powershell
winget install NghiNV.LumiTester
```

Winget should point to the GitHub Release installer or portable binary and map
the package id to `NghiNV.LumiTester`. Winget manifests are generated into release assets, but `winget install NghiNV.LumiTester` only works after those manifests are submitted to and accepted by `microsoft/winget-pkgs`.

Generated release assets:

- `winget-NghiNV.LumiTester.yaml`
- `winget-NghiNV.LumiTester.locale.en-US.yaml`
- `winget-NghiNV.LumiTester.installer.yaml`

## MCP Package

`lumi-tester-mcp` is a Node MCP server for AI agents. Release assets include platform-specific packages named `lumi-tester-mcp-<target>.tgz`; the AI installer downloads the right one automatically.

Runtime resolution order:

1. `LUMI_TESTER_BIN`
2. Bundled binary inside the package under `binaries/<platform>-<arch>/`
3. Repo-local `cargo run --`
4. `lumi-tester` from `PATH`

To create a self-contained MCP package for one OS/CPU:

```bash
cd lumi-tester-mcp
npm install
npm run stage-binary -- ../lumi-tester/target/release/lumi-tester
npm pack
```

For cross-platform release, stage each binary on the matching platform before
publishing that platform-specific MCP package, or keep the MCP package thin and
require the one-line installer above.
