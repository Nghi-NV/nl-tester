# Selector Discovery Playbook

Use this when creating tests for an unfamiliar app/page or when a selector
fails. The goal is to find the most stable selector quickly, then verify it with
the smallest run.

## Fast Path

1. Run `doctor` for the platform.
2. Start from a skeleton flow with `launchApp`.
3. Use Inspector if interactive discovery is possible.
4. If Inspector is not available, run with `--snapshot` and inspect UI XML.
5. Convert the target element into the highest-priority stable selector.
6. Validate YAML, list indexes, then rerun only the command being tested.

## Selector Priority

Use the first stable selector available:

1. `id`, `accessibilityId`, `contentDesc`, `desc`
2. exact `text`
3. `placeholder`, `role`, `css` for Web
4. `regex` for dynamic visible text
5. relative selector near a stable anchor
6. `type` with `index`
7. `ocr`
8. `image`
9. `point`

Coordinates are allowed only when no semantic selector exists or when testing
canvas/Android Auto/graphics-heavy UI.

## Inspector Workflow

Start Inspector from the CLI:

```bash
lumi-tester inspector --platform android --app-id com.example.app --port 9333
```

For Web:

```bash
lumi-tester inspector --platform web --url https://example.com --port 9333
```

With MCP, query a running Inspector:

```text
inspector_get /api/screenshot
inspector_get /api/hierarchy
inspector_get /api/element-at?x=100&y=200
```

Use `/api/element-at?x=<x>&y=<y>` when the user can point to the visual target
or when screenshot coordinates are known. Prefer the top selector suggestion
unless it is clearly unstable, generated, localized, or duplicated.

## MCP Selector Suggestions

When a failure XML exists, prefer `suggest_selectors` before manually reading a
large hierarchy:

```text
suggest_selectors outputDir=./output file=fail_login_cmd3_ui.xml query=Login
suggest_selectors outputDir=./output file=fail_login_cmd3_ui.xml point=540,960
```

Use `query` when you know visible text, resource id, content description, or
class. Use `point` when you know where the target appears in the screenshot.
The tool returns ranked selector candidates with YAML snippets.

## Snapshot Workflow

Run a small flow that reaches the screen and captures artifacts:

```bash
lumi-tester run ./flow.yaml --platform android --report --snapshot --events-jsonl --output ./output
```

Then inspect:

- `output/run.json` for failed command and artifact paths.
- `fail_*_cmdN_*.xml` for native hierarchy.
- `fail_*_cmdN_*.png` for visual confirmation.

In UI XML, look for:

- Android: `resource-id`, `text`, `content-desc`, `class`, `clickable`, bounds.
- iOS: accessibility identifier, label/name, value, type, visible state.
- Web: use CSS/role/text where available.

## Android Selector Examples

Resource id:

```yaml
- tap:
    id: "com.example:id/login_button"
```

Content description:

```yaml
- tap:
    desc: "Open menu"
```

Exact text:

```yaml
- see:
    text: "Welcome"
    exact: true
```

Dynamic text:

```yaml
- see:
    regex: "^Order #[0-9]+$"
```

Class/type fallback:

```yaml
- tap:
    type: "android.widget.EditText"
    index: 0
```

## iOS Selector Examples

Accessibility id:

```yaml
- tap:
    accessibilityId: "loginButton"
```

Visible label:

```yaml
- tap:
    text: "Log In"
    exact: true
```

Type fallback:

```yaml
- tap:
    type: "XCUIElementTypeTextField"
    index: 0
```

## Web Selector Examples

CSS:

```yaml
- tap:
    css: "[data-testid='login-button']"
```

Role:

```yaml
- tap:
    role: "button"
    text: "Sign in"
```

Placeholder:

```yaml
- tap:
    placeholder: "Email"
```

## Relative Selector Pattern

Use relative selectors when repeated rows share the same text/class and there
is a stable nearby label.

```yaml
- tap:
    relative:
      anchor:
        text: "Bedroom"
        exact: true
      direction: rightOf
      target:
        type: "android.widget.Switch"
```

Prefer a relative selector over `type + index` when the screen is a list, table,
settings page, or repeated card layout.

## OCR And Image Fallback

Use OCR when the text is visible in the screenshot but absent from hierarchy:

```yaml
- tap:
    ocr: "Continue"
```

Use image matching only for icon-only UI with no id/description:

```yaml
- tap:
    image: ./assets/settings-icon.png
    imageRegion: top-right
```

## Selector Debug Checklist

When `Element not found` happens:

1. Open the failure screenshot and confirm the element is visible.
2. Open the failure XML and check whether the element is exposed natively.
3. If visible in XML, replace selector with `id`, `desc`, or exact `text`.
4. If visible only in screenshot, try `ocr` or `image`.
5. If element appears after delay, add `waitUntilVisible` before interaction.
6. If inside a list, use `scrollUntilVisible`.
7. If duplicated, add `index`, `type`, or a relative anchor.
8. Rerun only the failed command with `--command-index`.

## Anti-Patterns

- Do not start with `point` unless testing Android Auto/canvas.
- Do not use translated text if stable ids/accessibility ids exist.
- Do not use broad regex like `.*Login.*` when exact text is stable.
- Do not fix selector failures by adding long `wait` first.
- Do not count command indexes manually; use `list --json`.
