# 🚀 lumi-tester

`lumi-tester` is a high-performance command-line tool (CLI) written in Rust, designed for automating mobile and web application testing. It empowers testers to define test scenarios using simple, intuitive YAML files.

## ✨ Highlights

- 📝 **YAML DSL**: Script-free test authoring with straightforward syntax.
- 📍 **Mock Location**: Simulate GPS coordinates via GPX, KML, or JSON files.
- 🎨 **Visual Assertion**: Verify UI consistency with color (`assertColor`) and pixel-level precision.
- 📹 **Recording & Screenshots**: Automatically capture images and videos during test execution.

### Support Platforms
- **Android**: ADB, UiAutomator (full control)
- **iOS**: IDB (simulators & devices), XCUITest
- **Web**: Playwright (Chrome/Firefox/WebKit)

### Key Features
- **Cross-platform DSL**: Write once, run everywhere (Android, iOS, Web).
- **Smart Selectors**: Support for `text`, `id`, `css`, `xpath`, `regex` (including advanced patterns like `\d+`, `[...]`, `(...)`), and `point`.
- **Control Flow**: Advanced logic with `repeat`, `conditional`, `runFlow`, and `variables`.
- **Media Support**: Integrated screenshots and video recording (video currently Android only).
- **Professional Reports**: Automated HTML/JSON reports with failure context and embedded media.
- 🛠️ **Parallel Execution**: Scale testing by running concurrently on multiple devices.

## 📦 Installation

### One-line Install

macOS / Linux:

```bash
curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.sh | bash
```

Windows PowerShell:

```powershell
iwr https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.ps1 -UseB | iex
```

The scripts detect OS/CPU, download the matching GitHub Release binary, add it to PATH, verify `SHA256SUMS` when available, and run `lumi-tester system install --all`.

Install a specific version:

```bash
LUMI_TESTER_VERSION=v0.1.3 curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.sh | bash
```

Skip driver/browser initialization:

```bash
LUMI_SKIP_SYSTEM_INSTALL=1 curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.sh | bash
```

### Package Managers

Planned distribution channels:

```bash
brew install nghi-nv/tap/lumi-tester
```

```powershell
scoop install lumi-tester
winget install NghiNV.LumiTester
```

### Manual Download

Download native binaries from [Releases](https://github.com/Nghi-NV/nl-tester/releases):

- **Windows**: `lumi-tester-x86_64-pc-windows-msvc.exe` or `lumi-tester-aarch64-pc-windows-msvc.exe`.
- **macOS**: `lumi-tester-aarch64-apple-darwin` for Apple Silicon, or `lumi-tester-x86_64-apple-darwin` for Intel.
- **Linux**: `lumi-tester-x86_64-unknown-linux-gnu` or `lumi-tester-aarch64-unknown-linux-gnu`.

## 🚀 Getting Started

Once installed, use the `lumi-tester` command directly in your terminal.

### 1. Run Tests
Execute a single test file or an entire directory:

```bash
# Run a specific file
lumi-tester run ./e2e/workspaces/login_flow.yaml

# Run all tests in a directory
lumi-tester run ./e2e/workspaces/
```

### 2. Environment Management
If you encounter ADB or driver issues, use the following command to repair/reinstall:

```bash
lumi-tester system install --all
```

### 3. List Connected Devices
```bash
lumi-tester devices
```

## 📚 Documentation

Deep-dive into our guides located in the `docs/` directory:

1. [**Writing Tests Guide**](docs/writing_tests.md): Master YAML syntax and selectors.
2. [**Commands Reference**](docs/api/commands.md): Comprehensive details on all supported commands (`tap`, `see`, `mockLocation`, `assertColor`, etc.).
3. [**Test Flow Structure**](docs/flows/test_execution_flow.md): Understand the lifecycle and flow of a test session.

---

## 💡 Quick Example

```yaml
appId: com.example.app
---
- open: "com.example.app"
- see: "Welcome"
- tap: "Login"
- inputText:
    id: "email_field"
    text: "test@example.com"
- assertColor:
    point: "50%,50%"
    color: "#4CAF50"
- takeScreenshot: "completed.png"
```
