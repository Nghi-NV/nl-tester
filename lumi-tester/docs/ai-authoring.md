# AI Authoring Contract

This document defines the Lumi YAML style expected from AI agents and automated
test generators.

## Required Loop

```bash
lumi-tester validate ./test.yaml --json
lumi-tester list ./test.yaml --json
lumi-tester doctor --json
lumi-tester run ./test.yaml --platform android --report --snapshot --events-jsonl --output ./output
```

When using the source checkout:

```bash
cd lumi-tester
cargo run -- validate ./test.yaml --json
cargo run -- list ./test.yaml --json
cargo run -- doctor --json
cargo run -- run ./test.yaml --platform android --report --snapshot --events-jsonl --output ./output
```

`doctor --json` defaults to Android dependency checks. Use
`doctor --platform ios --json`, `doctor --platform web --json`, or
`doctor --platform all --json` for other targets.

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

## Schema

Retrieve the bundled schema with:

```bash
lumi-tester schema --json
```

The schema is intentionally conservative and should be treated as an authoring
aid. `validate --json` remains the source of truth because it uses the Rust
parser.
