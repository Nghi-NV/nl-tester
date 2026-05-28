# Desktop App Testing

`lumi-tester` supports native desktop testing through two local platform drivers:

- `platform: macos`
- `platform: windows`

The desktop drivers are native MVPs. They do not use Appium. They use built-in OS tools and APIs through the existing `PlatformDriver` interface.

## macOS

Example:

```yaml
platform: macos
appId: /System/Applications/Calculator.app
---
- launchApp
- tap:
    point: "120,220"
- takeScreenshot: output/calculator.png
```

Supported MVP operations include:

- launch/quit app by `.app` path or bundle id
- open links
- coordinate tap, double tap, right click, and long press
- focused text input and erase
- common key presses
- clipboard get/set
- screenshot, screenshot comparison, and pixel color checks
- Accessibility hierarchy through `AXUIElement` for visible frontmost app elements
- best-effort `text`, `id`, `description`, `role`, and `type` selectors from macOS Accessibility attributes
- recent system logs via `log show`

Requirements:

- Terminal or `lumi-tester` must be allowed under macOS Privacy & Security permissions for Accessibility when using input automation or Accessibility selectors. If permission is missing, the driver opens the Accessibility settings pane and fails with a clear message.
- Screen Recording permission may be required for screenshots on newer macOS versions.
- `open`, `osascript`, `swift`, `screencapture`, and `log` must be available.

## Windows

Example:

```yaml
platform: windows
appId: C:\Windows\System32\notepad.exe
---
- launchApp
- inputText: "Hello from lumi-tester"
- takeScreenshot: output/notepad.png
```

Supported MVP operations include:

- launch app by executable path
- stop app by process name, with `taskkill` fallback
- coordinate tap, double tap, right click, and long press
- selector tap, double tap, right click, and long press through foreground-window UI Automation bounds
- focused text input through clipboard paste
- common key presses
- clipboard get/set
- screenshot, screenshot comparison, and pixel color checks
- UI Automation hierarchy for the current foreground window
- best-effort `text`, `id`, `description`, `role`, and `type` selectors from Windows UI Automation properties
- recent Application event log output

Requirements:

- Tests must run on a Windows host. Remote Windows desktop automation is not implemented in the native MVP.
- Windows PowerShell and .NET UI Automation assemblies (`UIAutomationClient`, `UIAutomationTypes`) must be available.
- `lumi-tester doctor --platform windows` checks both `powershell` and the UI Automation assemblies.
- Some desktop automation may require running the terminal with sufficient permissions, especially when automating elevated apps.

## Selector Support

Reliable MVP selectors:

- `point`
- screenshot-based commands
- pixel color commands

macOS best-effort native selectors:

- `text`
- `id`
- `description`
- `role`
- `type`

Windows best-effort native selectors read the foreground window through UI Automation:

- `text`
- `id`
- `description`
- `role`
- `type`

This is enough for real desktop smoke tests, launch/input/clipboard/screenshot/color assertions, basic macOS Accessibility checks, and basic Windows UI Automation selector checks. It is still not enough to claim reliable automation for every arbitrary desktop app, because desktop apps differ heavily in Accessibility/UI Automation exposure and canvas/game/custom-rendered apps may need image/OCR-driven flows.

## Manual Smoke Tests

macOS:

```bash
cargo run -- run e2e/desktop/macos-native-smoke.yaml --platform macos --output /tmp/lumi-macos-native-smoke --events-jsonl
cargo run -- run e2e/desktop/macos-ax-selector-smoke.yaml --platform macos --output /tmp/lumi-macos-ax-selector-smoke --events-jsonl
cargo run -- run e2e/desktop/macos-calculator-smoke.yaml --platform macos --output /tmp/lumi-macos-calculator-smoke --events-jsonl
```

Windows:

```powershell
cargo run -- run e2e\desktop\windows-native-smoke.yaml --platform windows --output $env:TEMP\lumi-windows-native-smoke --events-jsonl
cargo run -- run e2e\desktop\windows-uia-selector-smoke.yaml --platform windows --output $env:TEMP\lumi-windows-uia-selector-smoke --events-jsonl

# Or run the full local Windows desktop gate:
powershell -ExecutionPolicy Bypass -File scripts\run-windows-desktop-smoke.ps1
```

## CI Notes

The default desktop workflow builds and validates flows on hosted CI. Native GUI smoke tests are gated behind `workflow_dispatch` input `run_native_smoke=true` and use self-hosted macOS/Windows runners because desktop permissions and interactive sessions must be configured on the host.

macOS self-hosted runners need Accessibility permission for the runner terminal/process and Screen Recording permission for screenshot capture. Windows self-hosted runners need an interactive desktop session so UI Automation, screenshots, and SendKeys target the launched app window.
