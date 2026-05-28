# Desktop App Testing

Use this reference for native desktop flows using `platform: macos` or
`platform: windows`.

## Contents

- Platform model
- macOS flow
- Windows flow
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
  semantic selectors when exposed, but expect custom-rendered/canvas apps to
  need image/OCR or `point` fallback.

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
- tap:
    point: "120,220"
- screenshot: output/calculator.png
```

macOS requirements:

- Run on a macOS host.
- Grant Accessibility permission to the terminal, runner, or `lumi-tester`
  process before using input automation or Accessibility selectors.
- Grant Screen Recording permission when screenshots are blocked.
- `open`, `osascript`, `swift`, `screencapture`, and `log` should be available.
- `clearState` is not supported by the macOS MVP driver.

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
- `clearState` is not supported by the Windows MVP driver.

## Selector Guidance

Reliable desktop MVP selectors:

- `point`
- `image`
- `ocr`
- screenshot and pixel color commands

Best-effort native selectors:

- `text`
- `id`
- `description`
- `role`
- `type`

For macOS, selectors come from Accessibility attributes. For Windows, selectors
come from UI Automation properties on the foreground window. If the hierarchy
does not expose the target, inspect the screenshot, then use OCR/image or
`point` as a documented fallback.

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
