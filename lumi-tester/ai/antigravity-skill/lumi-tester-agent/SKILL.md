---
name: lumi-tester-agent
description: Design testcase coverage, write, validate, run, and debug Lumi Tester YAML automation flows for Android, iOS, Android Auto, Web, macOS, Windows, and Hardware Jigs. Use when AI (Codex, Antigravity) is asked to create test cases from requirements, generate grouped test folders, create or fix Lumi YAML tests, run Lumi Tester from a repo or installed binary, inspect validate/list/doctor/schema JSON output, debug failed commands using run.json/events.jsonl/test-results.json/screenshots/UI XML/logs, or rerun a failing command by command index.
---

# Lumi Tester Agent

Operate Lumi Tester as an autonomous AI test author and debugger across mobile, web, desktop, and hardware platforms.

## 1. Platform & Runtime Support

- **Android**: `platform: android`, app package `appId`, Android device serial, UIAutomator XML, `id`, `desc`, `text`, OCR.
- **Android Auto**: `platform: android_auto`, Android device serial + DHU runtime, point-only tap, dpad/key commands, screenshots.
- **iOS**: `platform: ios`, bundle id `appId`, simulator/device UDID, accessibility tree, `accessibilityId`, `text`, OCR.
- **Web**: `platform: web`, `url`, Playwright browser engines, DOM selectors (`css`, `role`, `placeholder`, `text`).
- **macOS**: `platform: macos`, bundle id or app path in `appId`, Accessibility hierarchy, `desktopState.clear`.
- **Windows**: `platform: windows`, executable path in `appId`, UI Automation hierarchy, `desktopState.clear`.
- **Hardware Jig**: `jig: "profiles/jig.yaml"`, standardized `hw*` commands (Relay, Servo, TCS34725 LED color & blink sensors).

## 2. Invocation Priority

Prefer MCP tools when `lumi-tester-mcp` is active (`doctor`, `validate_yaml`, `list_tests`, `schema`, `run_test`, `read_report`).

When using CLI, prefer repo-local `cargo run` when `lumi-tester/Cargo.toml` exists:
```bash
cd lumi-tester
cargo run -- <command>
```
Otherwise, use the installed binary `lumi-tester <command>`.

You can also run helper scripts directly:
```bash
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-check path/to/test.yaml
```

## 3. Canonical Workflow Loop

1. **Check Environment**:
   ```bash
   cargo run -- doctor --platform <platform> --json
   ```
2. **Author YAML Flow** in canonical `header --- commands` format:
   ```yaml
   platform: android
   appId: com.example.app
   defaultTimeout: 10000
   ---
   - launchApp
   - tap: { id: "login_btn" }
   - inputText: "user@example.com"
   - see: { text: "Welcome" }
   ```
3. **Validate**:
   ```bash
   cargo run -- validate path/to/test.yaml --json
   ```
4. **Inspect Runnable Indexes**:
   ```bash
   cargo run -- list path/to/test.yaml --json
   ```
5. **Run with Artifacts**:
   ```bash
   cargo run -- run path/to/test.yaml --platform <platform> --report --snapshot --events-jsonl --output ./output
   ```
6. **Debug & Rerun Failing Step**:
   Inspect `output/run.json`, `events.jsonl`, screenshot, and UI XML. Rerun target step:
   ```bash
   cargo run -- run path/to/test.yaml --platform <platform> --command-index <N> --output ./output
   ```

## 4. Selector Priority & Regex Power

Prioritize user-facing, multilingual resilient selectors:

1. **`regex` / Direct Shorthand (Recommended)**: Universal across all platforms, resilient to multi-language and dynamic text:
   ```yaml
   - tap: "name|tên"              # Shorthand regex (auto-detected on |, .*, .+, [, (, ^, $)
   - see: "Accept|Đồng ý"         # Multilingual assertion
   - waitUntilVisible: "^Order #\\d+$"
   - tap: { regex: "(Login|Đăng nhập)" }
   ```
2. **`id`**: Use when an explicit, stable resource ID exists in the hierarchy.
3. **`text` (`exact: true`)**: Single-locale fixed labels.
4. **Sub-Element Alignment**: `align: right | left | top | bottom | center` or `offset: "85%,50%"`.
5. **Platform attributes (`desc`, `accessibilityId`, `contentDesc`, `placeholder`)**: When specifically available in native tree.
6. **`role` / `type`** with `index`: Structural fallback.
7. **`ocr` / `point`**: Fallback only when hierarchy is missing.

## 5. Reference Map

Consult specialized reference files for in-depth rules and syntax (all under 200 lines):

- [references/index.md](file:///references/index.md): Central index and fast lookup guide.
- [references/cli.csv](file:///references/cli.csv): Full CLI options, parameters, and machine-readable output.
- [references/headers.csv](file:///references/headers.csv): YAML header fields (`platform`, `appId`, `jig`, `desktopState.clear`).
- [references/commands.csv](file:///references/commands.csv): Full command matrix, parameters, aliases, and platforms.
- [references/command-catalog.md](file:///references/command-catalog.md): Examples and intent for parameterized commands.
- [references/selectors.csv](file:///references/selectors.csv): Selector priority, semantic alignments (`align`, `offset`), and rules.
- [references/selector-discovery.md](file:///references/selector-discovery.md): Discovering selectors via Inspector, UI XML, and snapshots.
- [references/testcase-design.md](file:///references/testcase-design.md): Coverage design, test suite layout, and data-driven testing.
- [references/patterns.md](file:///references/patterns.md): Common flow templates (Login, Onboarding, Permission, GPS, Web).
- [references/hardware.md](file:///references/hardware.md): Hardware Jig automation, `hw*` commands, shared profiles, and LED blink detection.
- [references/desktop.md](file:///references/desktop.md): Native macOS and Windows desktop automation rules.
- [references/android-auto.md](file:///references/android-auto.md): Android Auto DHU setup and point interaction rules.
- [references/debug-artifacts.md](file:///references/debug-artifacts.md): Analyzing runtime failures, crash logs, and event JSONL.
