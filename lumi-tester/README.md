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

The easiest way is to download the installer from the [Releases](https://github.com/Nghi-NV/nl-tester/releases) page:

- **Windows**: Download `lumi-tester-setup.exe`.
- **macOS (Native)**: Download `lumi-tester-apple-silicon.pkg` (for M1/M2/M3 chips) or `lumi-tester-intel.pkg`.

Upon installation, the tool will automatically configure the necessary environment (ADB, Playwright) during its first run.

### GitHub CLI Installation (Advanced)
If you have the [GitHub CLI (gh)](https://cli.github.com/) installed, you can use the following scripts:

#### macOS / Linux
```bash
gh api repos/Nghi-NV/nl-tester/contents/lumi-tester/scripts/install.sh -H "Accept: application/vnd.github.v3.raw" | bash
```

#### Windows (PowerShell)
```powershell
gh api repos/Nghi-NV/nl-tester/contents/lumi-tester/scripts/install.ps1 -H "Accept: application/vnd.github.v3.raw" | powershell -
```

> [!IMPORTANT]
> **Important Notes:**
> 1. The `.exe` and `.pkg` installers automatically add the tool to your system PATH.
> 2. If you encounter driver issues, run `lumi-tester system install --all` to repair the environment.

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

