---
name: lumi-tester-agent
description: Write, validate, run, and debug Lumi Tester YAML automation flows for Android, iOS, Android Auto, and Web. Use when Codex is asked to create or fix Lumi YAML tests, run Lumi Tester from a repo or installed binary, inspect validate/list/doctor/schema JSON output, debug failed commands using run.json/events.jsonl/test-results.json/screenshots/UI XML/logs, or rerun a failing command by command index.
---

# Lumi Tester Agent

Use this skill to operate Lumi Tester as an AI test author and debugger. It is
for writing and running tests, not for extending the Lumi Tester framework
itself. For framework development, use the `lumi-tester` development skill.

## Find Lumi Tester

Prefer MCP tools when a `lumi-tester-mcp` server is configured. Use the MCP
tools in this order:

1. `doctor`
2. `validate_yaml`
3. `list_tests`
4. `run_test`
5. `read_report`, `read_events`, `read_artifact`
6. `suggest_selectors` when a UI XML artifact is available

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
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py validate path/to/test.yaml
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py list path/to/test.yaml
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py doctor --platform android
```

The helper prints stdout/stderr and exits with the Lumi command exit code.

## Authoring Loop

1. Search `references/commands.csv` first when choosing a command.
2. Search `references/selectors.csv` first when choosing a selector.
3. Read `references/command-catalog.md` when examples or command intent are
   still unclear.
4. Read `references/patterns.md` when the request matches a common workflow
   such as login, onboarding, search, settings, permission, GPS, or web form.
5. Write YAML in canonical `header --- commands` format.
6. Run validation before any device/browser execution.
7. Use `list --json` to discover command indexes.
8. Run with reports, snapshots, and event JSONL for debug-friendly artifacts.
9. On failure, inspect artifacts and rerun the smallest failing command index.

Canonical commands:

```bash
cargo run -- validate ./test.yaml --json
cargo run -- list ./test.yaml --json
cargo run -- doctor --platform android --json
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
4. `ocr` when native hierarchy cannot expose text
5. `point` as last resort, preferably percentages like `"50%,80%"`

For text entry, focus first, then type:

```yaml
- tap:
    id: "email"
- inputText: "user@example.com"
```

Do not put selector fields inside `inputText` unless the local parser explicitly
supports that form.

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
4. Patch only the smallest YAML selector/timing issue.
5. Rerun the failed command with `--command-index`.
6. Rerun the whole flow after the targeted command passes.

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

## Extra Reference

- Read `references/command-catalog.md` before writing unfamiliar commands.
- Read or search `references/commands.csv` for fast command lookup by alias,
  category, parameter shape, selector support, platform, and example.
- Read or search `references/selectors.csv` for selector priority, platform
  support, examples, and anti-patterns.
- Read `references/patterns.md` for common end-to-end flow templates and
  adaptation rules.
- Read `references/selector-discovery.md` when the app/page is unfamiliar,
  selectors are unknown, or a selector fails.
- Read `references/debug-artifacts.md` only when interpreting runtime files or
  building an agentic debug report.
