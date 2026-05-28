# Desktop App Testing

Use this reference for native desktop flows using `platform: macos` or
`platform: windows`.

## Contents

- Platform model
- macOS flow
- Windows flow
- Desktop state reset
- Selector guidance
- Commands

## Platform Model

- macOS uses local desktop automation through built-in OS tools, Apple Events,
  Accessibility, screenshots, clipboard, and best-effort Accessibility
  hierarchy for the frontmost app.
- Windows uses local desktop automation through PowerShell, Win32/.NET APIs,
  screenshots, clipboard, and best-effort UI Automation hierarchy for the
  foreground window.
- Desktop automation is local-host only in the MVP. Do not assume remote macOS
  or remote Windows execution.
- Desktop apps vary heavily in Accessibility/UI Automation exposure. Prefer
  semantic selectors when exposed. For custom-rendered/canvas apps, use
  screenshot/pixel checks for assertions and `point` only when no semantic
  selector exists.

## macOS Flow

```yaml
platform: macos
appId: /System/Applications/Calculator.app
tags:
  - desktop
  - macos
---
- launchApp
- waitUntilVisible:
    text: "Calculator"
    timeout: 5000
- setClipboard: "lumi desktop macos smoke"
- assertClipboard: "lumi desktop macos smoke"
- screenshot: output/calculator.png
```

macOS requirements:

- Run on a macOS host.
- Grant Accessibility permission to the terminal, runner, or `lumi-tester`
  process before using input automation or Accessibility selectors.
- Grant Screen Recording permission when screenshots are blocked.
- `open`, `osascript`, `swift`, `screencapture`, and `log` should be available.
- `clearState: true` is supported through the desktop state reset planner below.

## Windows Flow

```yaml
platform: windows
appId: C:\Windows\System32\notepad.exe
tags:
  - desktop
  - windows
---
- launchApp
- inputText: "Hello from lumi-tester"
- waitUntilVisible:
    text: "Hello from lumi-tester"
    timeout: 5000
- screenshot: output/notepad.png
```

Windows requirements:

- Run on a Windows host with an interactive foreground desktop session.
- PowerShell and .NET UI Automation assemblies must be available.
- `lumi-tester doctor --platform windows` checks the basic host prerequisites.
- Elevated apps may require running the terminal with sufficient permissions.
- `clearState: true` is supported through the desktop state reset planner below.

## Desktop State Reset

Use `desktopState.clear` when a desktop flow needs a first-run app state. Put
the state plan in the YAML header, then use `launchApp: { clearState: true }`.
Do not use mobile-only state commands such as `clearAppData` for desktop apps.

macOS auto-safe reset derives app state paths from the bundle id or `.app` name:

```yaml
platform: macos
appId: /Applications/MyApp.app
desktopState:
  clear:
    mode: autoSafe
    paths:
      - "~/Library/Application Support/MyApp"
    keychainServices:
      - "com.example.MyApp"
---
- launchApp:
    clearState: true
```

Windows auto-safe reset derives `%APPDATA%\<app>` and
`%LOCALAPPDATA%\<app>` from the executable name:

```yaml
platform: windows
appId: C:\Program Files\Example\Example.exe
desktopState:
  clear:
    mode: autoSafe
    paths:
      - "%APPDATA%\\Example"
    registryKeys:
      - "HKCU:\\Software\\Example"
---
- launchApp:
    clearState: true
```

Safety rules:

- `mode: autoSafe` adds common app-scoped state paths. `mode: manual` uses only
  the explicit paths/keys you provide.
- macOS manual paths must stay under the current user's home directory and
  cannot target broad roots such as `~/Library`, `~/Library/Application Support`,
  `~/Library/Caches`, `~/Library/Preferences`, or `~/Library/Containers`.
- Windows manual paths must stay under `%APPDATA%` or `%LOCALAPPDATA%`.
- Windows registry clearing only allows current-user software keys such as
  `HKCU:\Software\Vendor\App`; machine-wide hives such as `HKLM` are rejected.

## Selector Guidance

Prefer native desktop selectors when the Accessibility/UI Automation hierarchy
exposes them:

- `text`
- `id`
- `description`
- `role`
- `type`

For macOS, selectors come from Accessibility attributes. For Windows, selectors
come from UI Automation properties on the foreground window. If the hierarchy
does not expose the target, inspect the screenshot, then use screenshot/pixel
assertions or `point` as a documented fallback. `ocr` and `image` selectors are
not implemented for macOS/Windows desktop drivers in the current runtime.

## Commands

Useful desktop commands:

```bash
lumi-tester doctor --platform macos --json
lumi-tester doctor --platform windows --json
lumi-tester devices --platform macos
lumi-tester devices --platform windows
lumi-tester run ./desktop.yaml --platform macos --report --snapshot --events-jsonl --output ./output/desktop
lumi-tester run ./desktop.yaml --platform windows --report --snapshot --events-jsonl --output ./output/desktop
```

Manual smoke examples in this repo live under `lumi-tester/e2e/desktop/`.
