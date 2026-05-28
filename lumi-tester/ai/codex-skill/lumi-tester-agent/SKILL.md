---
name: lumi-tester-agent
description: Design testcase coverage, write, validate, run, and debug Lumi Tester YAML automation flows for Android, iOS, Android Auto, Web, macOS, and Windows. Use when Codex is asked to create test cases from requirements, generate grouped test folders, create or fix Lumi YAML tests, run Lumi Tester from a repo or installed binary, inspect validate/list/doctor/schema JSON output, debug failed commands using run.json/events.jsonl/test-results.json/screenshots/UI XML/logs, or rerun a failing command by command index.
---

# Lumi Tester Agent

Use this skill to operate Lumi Tester as an AI test author and debugger. It is
for writing and running tests, not for extending the Lumi Tester framework
itself. For framework development, use the `lumi-tester` development skill.

## Platform Coverage

Support Android, Android Auto, iOS, Web, macOS, and Windows workflows. Do not
specialize the skill, helper, or flow patterns for only one platform unless the
user's target is explicitly platform-specific. Always set or infer the target
platform before selecting commands, selectors, devices, and debug artifacts:

- Android: app package `appId`, Android device serial, UIAutomator XML,
  `id`/`resourceId`, `accessibilityId`/`contentDesc`, `text`, OCR fallback.
- Android Auto: `platform: android_auto`, Android device serial, DHU runtime,
  point-only tap, dpad/key commands, screenshot/log artifacts, no UI hierarchy.
- iOS: bundle id `appId`, simulator/device UDID, accessibility tree,
  `accessibilityId`/`label`, `text`, OCR fallback.
- Web: `url`/browser, DOM selectors such as `css`, `role`, `placeholder`,
  `text`, and browser artifacts.
- macOS: local desktop app path or bundle id in `appId`, Accessibility
  hierarchy, `text`/`id`/`description`/`role`/`type` best-effort selectors,
  screenshot/pixel commands, `point` fallback, Accessibility and Screen
  Recording permissions.
- Windows: local executable path in `appId`, UI Automation hierarchy for the
  foreground window, `text`/`id`/`description`/`role`/`type` best-effort
  selectors, screenshot/pixel commands, `point` fallback, interactive desktop
  session.

## Find Lumi Tester

Prefer MCP tools when a `lumi-tester-mcp` server is configured. Use the MCP
tools in this order:

1. `doctor`
2. `validate_yaml`
3. `list_tests`
4. `schema` when command/header shape is unclear
5. `run_test`
6. `read_report`, `read_events`, `read_artifact`
7. `inspector_get` when a Lumi Inspector server is running
8. `suggest_selectors` when a UI XML artifact is available

If MCP tools are not available, use the CLI flow below.

Prefer the repo-local CLI when the workspace contains `lumi-tester/Cargo.toml`:

```bash
cd lumi-tester
cargo run -- <command>
```

Use an installed binary only when available and the repo source is not present:

```bash
lumi-tester <command>
```

If no CLI is installed, install the AI pack:

```bash
brew install nghi-nv/tap/lumi-tester
lumi-tester ai install
```

Or use the one-line installer:

```bash
curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install-ai.sh | bash
```

Windows:

```powershell
iwr https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install-ai.ps1 -UseB | iex
```

You can use the bundled helper without loading it:

```bash
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-validate path/to/test.yaml
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-list path/to/test.yaml
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-doctor --platform android
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-run path/to/test.yaml --platform android --device <serial> --output ./output
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-run path/to/auto.yaml --platform android_auto --device <serial> --output ./output
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-debug path/to/test.yaml --platform android --device <serial> --command-index 3 --output ./output
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-run path/to/desktop.yaml --platform macos --output ./output
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-run path/to/desktop.yaml --platform windows --output ./output
```

The helper prefers repo-local `cargo run` and falls back to an installed
`lumi-tester` binary. `agent-run` and `agent-debug` always include
`--report --snapshot --events-jsonl` so failures have artifacts. Raw Lumi
commands such as `validate`, `list`, `doctor`, and `run` are still available as
passthrough commands when custom flags are needed. The helper prints
stdout/stderr and exits with the Lumi command exit code.

## Authoring Loop

1. Search `references/cli.csv` first when choosing a Lumi CLI command.
2. Search `references/commands.csv` first when choosing a YAML command.
3. Search `references/selectors.csv` first when choosing a selector.
4. Run `schema --json` when the YAML shape is unclear, but treat it as a
   guardrail, not a strict contract. Some command params/selectors are
   permissive; a successful validation means parseable, not proof that every
   field is semantically used.
5. Read `references/command-catalog.md` when examples or command intent are
   still unclear.
6. Read `references/testcase-design.md` before generating a suite from product
   requirements, user stories, screenshots, API specs, or exploratory findings.
7. Read `references/patterns.md` when the request matches a common workflow
   such as login, onboarding, search, settings, permission, GPS, or web form.
8. Read `references/android-auto.md` for Android Auto DHU tests.
9. Read `references/desktop.md` for native macOS or Windows desktop app tests.
10. For device-backed or desktop-backed requests, confirm the target device/app,
   local desktop host/app, or browser before writing or running a flow. If the
   user says "current app", inspect current focus/frontmost app instead of
   assuming an appId from an existing YAML file.
11. Discover the app identity before launch: Android package, iOS bundle id,
   Web URL/browser target, macOS `.app` path/bundle id, or Windows executable
   path.
12. After `launchApp`, wait for a stable screen element with `waitUntilVisible`
   or `waitSee`; do not use a fixed delay as launch readiness. Android Auto is
   the exception because DHU has no UI hierarchy; use bounded `wait` plus
   screenshot/log assertions there.
13. Write YAML in canonical `header --- commands` format.
14. Run validation before any device/browser/desktop execution.
15. Use `list --json` to discover command indexes.
16. Run with reports, snapshots, and event JSONL for debug-friendly artifacts.
17. On failure, inspect artifacts and rerun the smallest failing command index.

## Preflight Before Running

Before any real device/browser/desktop execution, do this checklist:

1. Run `doctor --platform <platform> --json` and fix missing runtime
   dependencies first.
2. Run `validate <file-or-folder> --json`; stop on `valid: false`.
3. Run `list <file-or-folder> --json` to confirm collected files, setup/hooks,
   skipped subflows, and command indexes.
4. If the suite depends on login, permissions, seeded data, or `clearState`,
   run the folder/group instead of a leaf file.
5. Run with `--report --snapshot --events-jsonl --output <dir>` so debug
   artifacts are available.

State reset rules:

- Use `launchApp: { clearState: true }` only for first-run/reset cases.
- Android/iOS clear state uses the app identity directly.
- macOS/Windows clear state requires header-level `desktopState.clear`; do not
  use Android-only `clearAppData` for desktop apps.
- For desktop, prefer `mode: autoSafe`; use `mode: manual` only when explicit
  app-scoped paths, Keychain services, or HKCU registry keys are known.

Canonical commands:

```bash
cargo run -- validate ./test.yaml --json
cargo run -- list ./test.yaml --json
cargo run -- doctor --platform android --json
cargo run -- doctor --platform android_auto --json
cargo run -- devices --platform android
cargo run -- schema --json
cargo run -- run ./test.yaml --platform android --report --snapshot --events-jsonl --output ./output
cargo run -- run ./test.yaml --platform android --command-index 3 --report --snapshot --events-jsonl --output ./output
```

For Web:

```bash
cargo run -- doctor --platform web --json
cargo run -- run ./test.yaml --platform web --report --snapshot --events-jsonl --output ./output
```

For iOS:

```bash
cargo run -- doctor --platform ios --json
cargo run -- run ./test.yaml --platform ios --report --snapshot --events-jsonl --output ./output
```

For Android Auto:

```bash
cargo run -- doctor --platform android_auto --json
cargo run -- run ./auto.yaml --platform android_auto --device <serial> --report --snapshot --events-jsonl --output ./output
```

For desktop:

```bash
cargo run -- doctor --platform macos --json
cargo run -- doctor --platform windows --json
cargo run -- run ./desktop.yaml --platform macos --report --snapshot --events-jsonl --output ./output
cargo run -- run ./desktop.yaml --platform windows --report --snapshot --events-jsonl --output ./output
```

## Canonical YAML

Prefer stable selectors over coordinates:

```yaml
platform: android
appId: com.example.app
tags:
  - smoke
defaultTimeout: 10000
---
- launchApp
- waitUntilVisible:
    id: "login_button"
- tap:
    id: "login_button"
- tap:
    id: "email"
- inputText: "test@example.com"
- see:
    text: "Welcome"
    exact: true
```

Selector priority:

1. `id`, `desc`, `accessibilityId`, `contentDesc`
2. `text` with `exact: true` when stable
3. `regex` for dynamic text
4. relative selectors near a stable anchor
5. `type` with `index`
6. `ocr` when native hierarchy cannot expose text
7. `point` only as a last resort, preferably percentages like `"50%,80%"`

For macOS and Windows desktop tests, `ocr` and `image` are not selector
fallbacks in the current runtime. Use Accessibility/UI Automation selectors
first, then screenshot/pixel assertions and `point` only when needed.

Do not switch to coordinates just because a selector failed once. First inspect
the UI XML/screenshot, choose the best semantic selector, and only use `point`
for canvas/graphics UI or when no native/visual semantic selector exists.

When a screen has duplicate accessibility labels, keep the semantic selector and
disambiguate with `index`, `type`, or a relative anchor before considering
coordinates.

For text entry, focus first, then type:

```yaml
- tap:
    id: "email"
- inputText: "user@example.com"
```

Do not put selector fields inside `inputText` unless the local parser explicitly
supports that form.

## App Identity Discovery

Find the app identity before writing `launchApp`:

- Android uses package name in `appId`, for example `com.example.app`.
- iOS uses bundle id in `appId`, for example `com.example.app`.
- Web uses `url` plus optional `browser`.
- macOS uses a `.app` path or bundle id in `appId`, for example
  `/Applications/MyApp.app` or `com.example.MyApp`.
- Windows uses an executable path in `appId`, for example
  `C:\Program Files\Example\Example.exe`.

When a user or existing YAML already provides an `appId`, verify it is installed
and launchable before debugging selectors:

```bash
adb -s <serial> shell pm path <appId>
adb -s <serial> shell cmd package resolve-activity --brief <appId>
xcrun simctl listapps <udid-or-booted> | rg '<bundleId>'
mdls -name kMDItemCFBundleIdentifier /Applications/MyApp.app
powershell -NoProfile -Command "Get-Item 'C:\Program Files\Example\Example.exe'"
```

After launch, compare the foreground/frontmost app with the expected `appId`.
If it does not match, debug install, launch, crash, or device selection before
tuning selectors.

Android foreground package/activity:

When the user references a connected device by exclusion, such as "not the LM
device", list devices and choose by serial/model explicitly:

```bash
adb devices -l
```

When the user asks to test the current Android app, identify the foreground app
from the selected device:

```bash
adb -s <serial> shell dumpsys window | rg -i 'mCurrentFocus|mFocusedApp|topResumed'
adb -s <serial> exec-out uiautomator dump /dev/tty
```

Use the package/activity from current focus as the flow `appId`. Do not reuse an
unrelated YAML file just because it exists in the workspace.

Flutter/Compose screens often expose visible labels as Android `content-desc`
instead of `text`; prefer `accessibilityId`/`desc` when the XML shows
`content-desc="..."`.

iOS bundle id discovery:

```bash
xcrun simctl list devices
xcrun simctl listapps booted | rg -i 'CFBundleIdentifier|CFBundleDisplayName|CFBundleName'
idb list-apps --udid <udid>
```

Web target discovery:

- Use the requested URL as `url`.
- If a local dev server is needed, start it first and use its localhost URL.
- Use `browser: chromium`, `firefox`, or `webkit` only when the browser matters.

## Launch Readiness

Immediately after opening an app or page, wait for a stable element that proves
the expected screen is ready:

```yaml
- launchApp:
    appId: com.example.app
- waitUntilVisible:
    accessibilityId: "Home"
    timeout: 15000
```

For Web:

```yaml
- launchApp
- waitUntilVisible:
    css: "[data-testid='home']"
    timeout: 15000
```

Use fixed `wait` after launch only as a last resort after a selector-based wait
cannot represent readiness. If launch restores a nested screen, inspect the UI
first and navigate with a real command such as `back` or a semantic button tap;
do not add coordinates.

`conditional.condition.visible` and `visibleRegex` currently check text only.
Do not use `conditional` to detect an Android `content-desc`/`accessibilityId`
unless the local runner has been verified to support that selector form.

## Debugging Loop

Use machine-readable files first:

- `output/run.json`: always written after executor finalization.
- `output/events.jsonl`: written when `--events-jsonl` is passed.
- `output/test-results.json`: written when `--report` is passed.
- Failed commands may contain `screenshotPath`, `uiHierarchyPath`, and `logPath`
  when `--snapshot` or `--report` is enabled.

Debug process:

1. Read the first failed command from `run.json` or `test-results.json`.
2. Check its `index`, `commandName`, `status.error`, and artifact paths.
3. Inspect the screenshot/UI XML/log excerpt if present.
4. Read `references/debug-artifacts.md` when the cause is not obvious,
   especially for wrong app, crash, permission dialog, or platform-specific
   failures.
5. Patch only the smallest YAML selector/timing/setup issue.
6. Rerun the failed command with `--command-index`.
7. Rerun the whole flow after the targeted command passes.

Do not count command indexes by hand; use `list --json`.

## Validation Rules

Treat these as authoring bugs:

- `validate --json` returns `valid: false`.
- Unknown command errors.
- Missing referenced files.
- A command index is absent from `list --json`.

Treat these as runtime/debug bugs:

- Element not found despite valid YAML.
- Assertion timeout.
- App crash event.
- Screenshot/UI hierarchy mismatch.
- Current focus or failure screenshot shows a different app than the expected
  appId.

## Extra Reference

- Read or search `references/cli.csv` for fast Lumi CLI lookup by purpose,
  platform, options, output, and when an AI agent should use it.
- Read `references/command-catalog.md` before writing unfamiliar commands.
- Read `references/testcase-design.md` before turning requirements or
  exploratory findings into a folder of generated tests.
- Read or search `references/commands.csv` for fast command lookup by alias,
  category, parameter shape, selector support, platform, and example.
- Read or search `references/selectors.csv` for selector priority, platform
  support, examples, and anti-patterns.
- Read `references/patterns.md` for common end-to-end flow templates and
  adaptation rules.
- Read `references/desktop.md` for macOS/Windows app identity, permissions,
  selectors, and `desktopState.clear` examples.
- Read `references/selector-discovery.md` when the app/page is unfamiliar,
  selectors are unknown, or a selector fails.
- Read `references/android-auto.md` for DHU setup, point-only interaction, and
  Android Auto command limits.
- Read `references/debug-artifacts.md` only when interpreting runtime files or
  building an agentic debug report.
