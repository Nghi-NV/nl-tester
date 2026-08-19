# Lumi Tester Agent Guide

This repository contains Lumi Tester, a Rust CLI and desktop studio for Android,
iOS, Android Auto, and Web automation using YAML flows.

## Agent Workflow

Use this loop when creating or debugging tests:

1. Author YAML using the canonical `header --- commands` format.
2. Validate without launching a device:
   ```bash
   cd lumi-tester
   cargo run -- validate path/to/test.yaml --json
   ```
3. Inspect runnable command indexes:
   ```bash
   cd lumi-tester
   cargo run -- list path/to/test.yaml --json
   ```
4. Run the full flow when validation passes:
   ```bash
   cd lumi-tester
   cargo run -- run path/to/test.yaml --platform android --report --snapshot --output ./output
   ```
5. Rerun one failing command while debugging:
   ```bash
   cd lumi-tester
   cargo run -- run path/to/test.yaml --platform android --command-index 3 --report --snapshot --output ./output
   ```
6. For runtime debugging, request structured events and read the manifest:
   ```bash
   cd lumi-tester
   cargo run -- run path/to/test.yaml --platform android --report --snapshot --events-jsonl --output ./output
   ```
   Read `output/events.jsonl`, `output/run.json`, and `output/test-results.json`.

## Environment Checks

Use `doctor --json` before runtime tests on a new machine:

```bash
cd lumi-tester
cargo run -- doctor --json
```

Use `--platform ios`, `--platform web`, or `--platform all` when needed.

Use `schema --json` to retrieve the bundled YAML schema:

```bash
cd lumi-tester
cargo run -- schema --json
```

## Canonical YAML

Prefer this format:

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
- inputText: "test@example.com"
- see:
    text: "Welcome"
```

Avoid list-only YAML for new tests. Avoid top-level `steps:` unless integrating
with a system that requires it.

## Selector Priority

Prioritize user-facing, multi-language resilient selectors:

1. **`regex` / Shorthand (Recommended for most cases)**: Highly recommended for general UI, multi-language (e.g. `tap: "name|tên"`, `see: "Accept|Đồng ý"`), and dynamic text (`tap: "^Submit.*$"`).
2. **`id`**: Use when a stable, explicit resource ID / test ID actually exists in the hierarchy.
3. **`text` (`exact: true`)**: Single-locale stable text labels.
4. **Platform fields (`desc`, `accessibilityId`, `contentDesc`, `placeholder`)**: Use when specifically exposed by the platform/framework (Android, iOS, Web, Desktop).
5. **`role` / `type`** with `index`: Component structure fallback.
6. **`ocr`**: When native hierarchy cannot expose the text.
7. **`point`**: Last resort (percentages e.g. `"50%,80%"`).

### Regex Selector Shorthand

Lumi Tester automatically detects regex strings containing `|`, `.*`, `.+`, `[`, `(`, `^`, or `$`. You can write regex selectors directly in compact shorthand:

```yaml
- tap: "name|tên"
- see: "Accept|Đồng ý"
- waitUntilVisible: "^Order #\\d+$"
# Or structured format:
- tap:
    regex: "(Login|Đăng nhập)"
```

### Sub-Element Positioning (`align` / `offset`)

For composite items (e.g. `Switch` toggles, list rows with icons/buttons on edges), use `align` or `offset` with semantic selectors instead of raw screen coordinates:

```yaml
# Tap right side of switch row (where the toggle toggle is)
- tap:
    type: "Switch"
    index: 1
    align: right # Presets: left (10%), right (90%), top (10%), bottom (90%), center (50%)

# Custom percentage offset within element bounds:
- tap:
    id: "settings_row"
    offset: "85%,50%"
```

For text entry, tap/focus the field first, then use `inputText`.

```yaml
- tap:
    id: "email"
- inputText: "user@example.com"
```

## Debugging Rules

- Always run `validate --json` before running device tests.
- Use `list --json` to discover `--command-index`; do not count indexes by hand.
- Treat unknown commands and unknown parse errors as authoring bugs, not runtime bugs.
- On runtime failure, inspect `output/test-results.json`, screenshots, UI XML, and logs.
- `output/run.json` is always written after executor finalization, even when full reports are disabled.
- `output/events.jsonl` is written when `--events-jsonl` is passed.
- Prefer minimal selector patches, then rerun the failing command index.
