# AI Authoring Contract

This document defines the Lumi YAML style expected from AI agents and automated
test generators.

## Required Loop

```bash
lumi-tester validate ./test.yaml --json
lumi-tester list ./test.yaml --json
lumi-tester doctor --platform <platform> --json
lumi-tester run ./test.yaml --platform <platform> --report --snapshot --events-jsonl --output ./output
```

When using the source checkout:

```bash
cd lumi-tester
cargo run -- validate ./test.yaml --json
cargo run -- list ./test.yaml --json
cargo run -- doctor --platform <platform> --json
cargo run -- run ./test.yaml --platform <platform> --report --snapshot --events-jsonl --output ./output
```

Use an explicit platform for every device/browser/desktop run:

```bash
lumi-tester doctor --platform android --json
lumi-tester doctor --platform android_auto --json
lumi-tester doctor --platform ios --json
lumi-tester doctor --platform web --json
lumi-tester doctor --platform macos --json
lumi-tester doctor --platform windows --json
```

`doctor --platform all --json` is useful for environment audits, but AI agents
should still run the exact target platform before executing a flow.

## Canonical File Shape

```yaml
platform: web
url: "https://example.com"
browser: Chrome
defaultTimeout: 10000
tags:
  - smoke
---
- launchApp
- tap:
    text: "Sign in"
    exact: true
- inputText: "user@example.com"
- see:
    text: "Dashboard"
```

Agents should emit this `header --- commands` shape unless the user explicitly
asks for top-level `steps:`.

Before choosing header fields, search
`ai/codex-skill/lumi-tester-agent/references/headers.csv` for platform support,
aliases, examples, and desktop `desktopState.clear` schema. After
`lumi-tester ai install`, the same file is available at
`~/.codex/skills/lumi-tester-agent/references/headers.csv`.

Always set the platform and app identity explicitly:

- Android: `platform: android` with package name in `appId`.
- Android Auto: `platform: android_auto` with package name in `appId` and DHU
  available.
- iOS: `platform: ios` with bundle id in `appId`.
- Web: `platform: web` with `url`.
- macOS: `platform: macos` with `.app` path or bundle id in `appId`.
- Windows: `platform: windows` with executable path in `appId`.

## State Reset

Use `clearState: true` only when the test intentionally needs first-run or
fresh-session behavior. Prefer shared setup flows, seeded data, or grouped suite
execution when later test files depend on login/session state.

Android and iOS can clear state from the app identity directly:

```yaml
platform: android
appId: com.example.app
---
- launchApp:
    clearState: true
```

For macOS and Windows, always pair `launchApp: { clearState: true }` with a
header-level `desktopState.clear` plan. Do not use Android-only `clearAppData`
for desktop apps.

```yaml
platform: macos
appId: /Applications/MyApp.app
desktopState:
  clear:
    mode: autoSafe
---
- launchApp:
    clearState: true
```

```yaml
platform: windows
appId: C:\Program Files\Example\Example.exe
desktopState:
  clear:
    mode: autoSafe
---
- launchApp:
    clearState: true
```

Use `mode: autoSafe` by default. Use `mode: manual` only when explicit
app-scoped paths, macOS Keychain services, or Windows `HKCU:\Software\...`
registry keys are known and documented in the test header.

## Launch Readiness And Shared Setup

After `launchApp`, wait for a stable screen element with `waitUntilVisible` or
`waitSee`; do not use a fixed `wait` as launch readiness. Android Auto is the
exception because the DHU driver has no UI hierarchy; use a bounded `wait` plus
screenshot/log assertions there.

Use `permissions` only when the testcase requires a pre-granted or pre-denied
state. Do not assume `permissions: { all: allow }` is correct for every flow.
For permission behavior, write separate allow/deny cases or reusable
permission setup flows.

When tests depend on login, permission setup, seeded data, `setup.yaml`, or
`clearState`, validate/list/run the folder or group instead of a leaf file:

```bash
lumi-tester validate tests/generated/account --json
lumi-tester list tests/generated/account --json
lumi-tester run tests/generated/account --platform <platform> --report --snapshot --events-jsonl --output ./output/account
```

Use `runFlow` for reusable login, permission, and cleanup blocks. Keep generated
test files under a feature folder such as `tests/generated/<feature>/` so setup,
data, subflows, and reports stay together.

## Preferred Commands

Use these names for new files:

| Purpose | Preferred command |
| --- | --- |
| Launch app or URL | `launchApp` |
| Tap | `tap` |
| Enter text into focused field | `inputText` |
| Assert visible | `see` |
| Assert not visible | `notSee` |
| Wait for visible | `waitUntilVisible` |
| Scroll until visible | `scrollTo` |
| Run subflow | `runFlow` |
| Screenshot | `takeScreenshot` |

Aliases may parse, but agents should avoid mixing aliases in generated files.

## Selector Rules

Before choosing a selector, search
`ai/codex-skill/lumi-tester-agent/references/selectors.csv` for platform
support, rank, and anti-patterns. For unfamiliar screens or selector failures,
read `ai/codex-skill/lumi-tester-agent/references/selector-discovery.md`.
After `lumi-tester ai install`, the same files are available under
`~/.codex/skills/lumi-tester-agent/references/`.

Fast selector discovery loop:

1. Use Inspector when available: `inspect`, then `inspector_get /api/hierarchy`
   or `inspector_get /api/element-at?x=<x>&y=<y>`.
2. If Inspector is not available, run with `--snapshot` and inspect
   `uiHierarchyPath`/UI XML plus the linked screenshot.
3. If MCP has a UI XML artifact, call `suggest_selectors` before manually
   reading a large hierarchy.
4. If the UI XML package or foreground app is not the expected app identity,
   debug launch/crash/wrong target before tuning selectors.

Prefer stable selectors:

```yaml
- tap:
    id: "login_button"

- tap:
    desc: "Login"

- see:
    text: "Welcome"
    exact: true

- see:
    regex: "OTP: \\d{6}"

- tap:
    ocr:
      text: "Continue"
      region: "bottom-half"
```

Use coordinates only when no semantic selector exists:

```yaml
- tap:
    point: "50%,82%"
```

## Valid Text Input Pattern

Do not put selector fields inside `inputText`. Focus first, type second.

```yaml
- tap:
    id: "email"
- inputText: "test@example.com"
```

## Machine-Readable Validation

`validate --json` returns:

```json
{
  "valid": true,
  "files": [
    {
      "path": "test.yaml",
      "platform": "android",
      "commandCount": 4,
      "commands": [
        { "index": 0, "name": "launchApp" }
      ]
    }
  ],
  "errors": []
}
```

If `valid` is false, fix the YAML before running device tests.

`list --json` returns the same file and command-index shape without the `valid`
and `errors` fields. Use those indexes for targeted reruns.

## Runtime Artifacts

After executor finalization:

- `output/run.json` is always written and contains the session summary, flows,
  commands, failures, duration, and artifact paths.
- `output/test-results.json`, `output/report.html`, and `output/junit.xml` are
  written when `--report` is enabled.
- `output/events.jsonl` is written when `--events-jsonl` is enabled.
- Failed commands may include `screenshotPath`, `uiHierarchyPath`, and `logPath`
  when `--snapshot` or `--report` is enabled.

## Failure Debug Loop

When a run fails:

1. Read the first failed command from `output/run.json` or
   `output/test-results.json`.
2. Use `list --json` to confirm the command index; do not count YAML commands
   by hand.
3. Inspect linked `screenshotPath`, `uiHierarchyPath`, and `logPath` before
   editing selectors or waits.
4. Patch the smallest YAML/setup issue.
5. Rerun only the failed command:

```bash
lumi-tester run ./test.yaml --platform <platform> --command-index <index> --report --snapshot --events-jsonl --output ./output
```

6. After the targeted rerun passes, rerun the whole flow with the same
   `--report --snapshot --events-jsonl` artifact flags.

For ambiguous failures, read
`ai/codex-skill/lumi-tester-agent/references/debug-artifacts.md` and classify
the issue as wrong target, setup/state, app/runtime, or selector before editing
YAML. On machines with the installed Codex skill, read the same file from
`~/.codex/skills/lumi-tester-agent/references/debug-artifacts.md`.

## Schema

Retrieve the bundled schema with:

```bash
lumi-tester schema --json
```

The schema is intentionally conservative and should be treated as an authoring
aid. `validate --json` remains the source of truth because it uses the Rust
parser.
