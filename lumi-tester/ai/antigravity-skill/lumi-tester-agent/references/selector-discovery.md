# Selector Discovery Playbook

Practical guide for discovering, disambiguating, and refining stable UI selectors across platforms.

## 1. Selector Priority Matrix

Always prefer semantic and accessibility identifiers over coordinates:

| Priority | Selector Type | Example / Shorthand | Platform Support |
| :--- | :--- | :--- | :--- |
| **1 (Highest)** | `id` / `accessibilityId` / `desc` | `id: "login_button"`<br>`accessibilityId: "Save"` | Android, iOS, Desktop |
| **2 (High)** | **`regex` / Shorthand** | **`tap: "name|tên"`**<br>`see: "Accept|Đồng ý"`<br>`regex: "^Order #\\d+$"` | All platforms |
| **3** | Exact `text` | `text: "Submit", exact: true` | All platforms |
| **4** | `role`, `placeholder`, `css` | `role: "button", text: "Login"`<br>`css: ".primary-btn"` | Web |
| **5** | `type` with `index` | `type: "input", index: 0` | All platforms |
| **6** | `ocr` / `image` | `ocr: "Camera Feed"` | Fallback when tree is unavailable |
| **7 (Last Resort)**| `point` | `point: "50%,80%"` | Canvas / DHU only |

### Regex Shorthand Syntax
Lumi Tester automatically converts strings containing regex tokens (`|`, `.*`, `.+`, `[`, `(`, `^`, `$`) into regex selectors.
```yaml
# Direct string shorthand:
- tap: "name|tên"
- see: "Submit|Xác nhận"
- waitUntilVisible: "^(Loading|Đang tải)..."

# Structured YAML:
- tap:
    regex: "(Save|Lưu|Confirm)"
```

## 2. Sub-Element Positioning (`align` & `offset`)

For composite controls (e.g. toggle switches or edge icons on list rows):

```yaml
# Target switch toggle on right edge of row
- tap:
    type: "Switch"
    index: 0
    align: right # Presets: left (10%), right (90%), top (10%), bottom (90%), center (50%)

# Percentage offset relative to element bounding box
- tap:
    id: "settings_row"
    offset: "85%,50%"
```

## 3. Discovery Workflows

### A. Interactive Web Inspector
Launch the visual inspector to explore UI hierarchies:
```bash
lumi-tester inspect --platform android --device <serial> --port 9333
# Or in VS Code: Cmd+Shift+P -> "Lumi: Open Element Inspector"
```

### B. Command-line Snapshot Discovery
Dump UI hierarchy and capture screenshot without full flow execution:
```bash
# Android
adb exec-out uiautomator dump /dev/tty

# Or run single step with snapshot:
lumi-tester run ./test.yaml --platform android --snapshot --output ./output
# Inspect output/ui_hierarchy.xml and screenshots
```

## 4. Platform Quirks & Rules

- **Android (Compose/Flutter)**: Text often appears under `content-desc` instead of `text`. Use `accessibilityId` or `desc`.
- **iOS**: Use `accessibilityId` (maps to `accessibilityIdentifier`) for stable test automation.
- **Web**: Prefer accessible roles (`role: "button"`) and `text` before falling back to `css`.
- **macOS / Windows**: Use `role` and `title`/`text` exposed via Accessibility / UI Automation tree.

## 5. Anti-Patterns to Avoid
- ❌ Do not use screen coordinates (`point: 500,800`) for standard buttons.
- ❌ Do not use fixed sleeps (`wait: 5000`) instead of `waitUntilVisible`.
- ❌ Do not rely on unstable XPath selectors.
