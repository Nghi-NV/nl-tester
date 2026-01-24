# Lumi Tester - VSCode Extension

VSCode extension for [lumi-tester](https://github.com/lumi/lumi-tester) - A powerful mobile and web UI testing framework.

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

1. Open VSCode Extensions (Ctrl+Shift+X)
2. Search for "Lumi Tester"
3. Click Install

Or install from VSIX:
```bash
code --install-extension lumi-tester-0.1.0.vsix
```

## Configuration

| Setting | Description | Default |
|---------|-------------|---------|
| `lumi-tester.lumiTesterPath` | Path to lumi-tester project directory | Auto-detect |
| `lumi-tester.outputDirectory` | Output directory for artifacts | `./output` |

## Requirements

- [lumi-tester](https://github.com/lumi/lumi-tester) installed
- Rust/Cargo (for building lumi-tester)
- Node.js 18+ (for development)

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

## License

MIT
