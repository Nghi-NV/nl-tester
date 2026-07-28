# Lumi Tester - VSCode Extension

VSCode extension for [lumi-tester](https://github.com/Nghi-NV/nl-tester) - A powerful mobile and web UI testing framework.

## Features

### 🔧 YAML Autocomplete
- **Command suggestions**: Type `-` and get suggestions for all 60+ lumi-tester commands
- **Parameter hints**: Get parameter suggestions with types and descriptions
- **Smart snippets**: Auto-insert common patterns

### ▶️ Run Tests
- **Run File**: Click the ▶ button in editor title bar to run entire test file
- **Run Command**: Click ▷ on any command line to run just that command
- **Stop Test**: Cancel running tests anytime

### 📊 Status Display
- ⚪ Pending - Not yet executed
- ⏳ Running - Currently executing
- ✅ Passed - Command succeeded
- ❌ Failed - Command failed

## Installation

Install the native Lumi Tester CLI first. The installer downloads the Windows
binary plus common Android/Web dependencies; Rust and Cargo are not required.

```powershell
iwr https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.ps1 -UseB | iex
```

Then install the extension VSIX:

```powershell
code --install-extension lumi-tester-0.1.19.vsix
```

Reload VS Code after installation. The extension automatically checks `PATH`
and `%USERPROFILE%\.lumi-tester\bin\lumi-tester.exe`.

## Configuration

| Setting | Description | Default |
|---------|-------------|---------|
| `lumi-tester.lumiTesterPath` | Optional CLI executable or source directory | Auto-detect |
| `lumi-tester.outputDirectory` | Output directory for artifacts | `./output` |

The default CLI installation needs no setting. For an explicit override, use an
absolute executable path:

```json
{
  "lumi-tester.lumiTesterPath": "C:\\Users\\QueDT\\.lumi-tester\\bin\\lumi-tester.exe"
}
```

`${workspaceFolder}` and `${userHome}` are supported. Unresolved variables are
reported as configuration errors instead of being resolved relative to the VS
Code installation directory.

If an older extension version configured
`${workspaceFolder}\\tools\\lumi-inspector-shim`, clear that setting and reload
VS Code. The installed CLI is detected automatically.

## Requirements

- [lumi-tester](https://github.com/Nghi-NV/nl-tester) native CLI installed
- A connected Android device with USB debugging enabled for Android tests

Rust, Cargo, and Node.js are required only for extension development, not for
installed-extension use.

## Development

```bash
# Clone and install dependencies
cd lumi-tester-vscode
npm install

# Compile TypeScript
npm run compile

# Run extension in debug mode
# Press F5 in VSCode
```

## Commands

| Command | Description |
|---------|-------------|
| `Lumi: Run Test File` | Run all commands in current YAML file |
| `Lumi: Run Single Command` | Run command at current line |
| `Lumi: Stop Test` | Stop running test |
| `Lumi: Open Element Inspector` | Start Inspector through the installed CLI |
| `Lumi: Select Device` | Select an Android, iOS, or Web target |
| `Lumi: Diagnose Setup` | Show resolved CLI/ADB paths and versions |

## Troubleshooting

Run `Lumi: Diagnose Setup` from the Command Palette. It reports the CLI runtime,
version, resolved ADB path, and Android device count.

## License

MIT
