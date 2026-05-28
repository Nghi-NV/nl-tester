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
git tag v0.1.3
git push origin v0.1.3
```

The GitHub Actions release workflow builds all targets, uploads the binaries, and publishes `SHA256SUMS`.

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
LUMI_TESTER_VERSION=v0.1.3 curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.sh | bash
```

Skip `system install --all`:

```bash
LUMI_SKIP_SYSTEM_INSTALL=1 curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.sh | bash
```

After installation, users can run:

```bash
lumi-tester --version
lumi-tester system install --all
```

## Package Managers

Package-manager distribution should wrap the same release assets.

### Homebrew

Expected UX:

```bash
brew install nghi-nv/tap/lumi-tester
```

Formula behavior:

- Download the correct release tar/binary for macOS.
- Install `lumi-tester` into Homebrew's `bin`.
- Do not run `system install --all` automatically; print caveats telling users to run it.

### Scoop

Expected UX:

```powershell
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
the package id to `NghiNV.LumiTester`.

## MCP Package

`lumi-tester-mcp` is a Node MCP server for AI agents.

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
