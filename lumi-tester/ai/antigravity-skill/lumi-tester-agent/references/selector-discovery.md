# Selector Discovery Playbook

Use this when creating tests for an unfamiliar app/page or when a selector
fails. The goal is to find the most stable selector quickly, then verify it with
the smallest run.

## Contents

- Fast path
- Selector priority
- Terse YAML style
- Platform gotchas
- Inspector workflow
- MCP selector suggestions
- Snapshot workflow
- Android selector examples
- iOS selector examples
- Web selector examples
- Desktop selector examples
- Relative selector pattern
- OCR and image fallback
- Selector debug checklist
- Anti-patterns

## Fast Path

1. Run `doctor` for the platform.
2. Confirm the target device and app identity. Use Android package name, iOS
   bundle id, Web URL/browser, macOS `.app` path/bundle id, or Windows
   executable path. If the user asks for the current Android app, read
   `mCurrentFocus`/`mFocusedApp` from that device before choosing `appId`.
3. Start from a skeleton flow with `launchApp`.
4. Add a selector-based launch readiness wait for a stable screen element.
5. **Unfamiliar screen, do not know where to tap yet:** start Inspector and
   click anywhere on the live screenshot if a human is driving; if not (an
   agent working non-interactively), call `suggest_selectors` with **no**
   `query`/`point` - it lists every visible/clickable element on the current
   screen with bounds and a ranked selector, so you do not need to guess a
   coordinate or element name first. See "MCP Selector Suggestions" below.
6. If Inspector is not available, run with `--snapshot` and inspect UI XML.
7. Convert the target element into the highest-priority stable selector.
8. Validate YAML, list indexes, then rerun only the command being tested.

## Selector Priority

Use the first stable selector available (same order as `SKILL.md`'s Canonical
YAML section - keep both in sync):

1. `regex` / bare-string shorthand (e.g. `tap: "name|tên"`) for dynamic or
   multi-language visible text
2. `id` when a stable resource id exists in the hierarchy
3. exact `text` (`exact: true`) when stable and single-locale
4. Platform fields: `desc`/`accessibilityId`/`contentDesc`, `placeholder`,
   `role`, `css` for Web
5. relative selector (`below`/`above`/`rightOf`/`leftOf`) near a stable anchor
6. `type` with `index` (omit `index` when it is 0)
7. `ocr` on Android, iOS, or Web
8. `image` on Android, iOS, or Web
9. `point`

"Prefer regex first" does not mean writing `regex:` by hand for ordinary
stable text. `tap`/`see`/`waitUntilVisible`/`scrollUntilVisible` and other
shorthand-supporting commands auto-detect this: a bare string is parsed as
`regex` only when it contains a regex metacharacter (`.*`, `.+`, `^...$`,
`\d+`, `\d{`, `[`, `(`, or `|`); otherwise it resolves to plain `text`. So the
actual rule is: **always try the bare-string shorthand first** and let the
parser classify it - reach for `id`/`desc` only when text alone is not unique
or stable enough. See `SKILL.md`'s "Terse YAML Style" section for the full
shorthand rules.

Coordinates are allowed only when no semantic selector exists or when testing
canvas/Android Auto/graphics-heavy UI.

If a selector fails, do not immediately replace it with `point`. First inspect
the failure XML and screenshot. If the element exists in XML, keep a semantic
selector and disambiguate with `index`, `type`, or a relative anchor.

For Flutter/Compose Android apps, visible text is often exposed as
`content-desc` instead of `text`. In those cases use `accessibilityId`, `desc`,
or `contentDesc`.

For composite items (e.g. `Switch` toggles, list rows with icons/buttons on edges),
use `align` (`left`, `right`, `top`, `bottom`, `center`) or `offset` (`"X%,Y%"`)
instead of raw screen `point` coordinates:
```yaml
# Tap toggle switch on right edge of row
- tap:
    type: "Switch"
    index: 1
    align: right
```

If the failure screenshot/UI XML belongs to a different package than the
expected `appId`, stop selector tuning and debug app launch, crash, or wrong
target selection.

## Terse YAML Style

Prefer the shortest form that still expresses the selector correctly. Two
independent shorthand mechanisms exist - do not confuse them:

**1. Auto-regex-detecting string shorthand** - a bare string is parsed as
`regex` only if it contains a regex metacharacter (`.*`, `.+`, `^...$`,
`\d+`, `\d{`, `[`, `(`, or `|`); otherwise it resolves to plain `text`.
Applies to:

- `tap` / `longPress` / `doubleTap`
- `see` / `assertVisible` / `assertNotVisible` / `waitUntilVisible` /
  `waitSee` / `waitUntilNotVisible`
- `scrollUntilVisible`
- `drag.from` / `drag.to`
- the relative-anchor sub-fields `rightOf` / `leftOf` / `above` / `below`

```yaml
# Prefer:
- tap: "Login"
- see: "Order #[0-9]+"        # auto-detected as regex (has [0-9])
- tap:
    type: "Switch"
    rightOf: "Bedroom"          # anchor shorthand, auto-detected as text

# Over:
- tap:
    text: "Login"
- see:
    regex: "Order #[0-9]+"
- tap:
    type: "Switch"
    rightOf:
      text: "Bedroom"
```

Escalate to the full object form only when you need a field the shorthand
cannot express: `index > 0`, `exact: true`, `timeout`, `soft`, a
platform-specific field (`id`, `desc`, `accessibilityId`, `css`, `role`), or
an explicit `regex:`/`text:` override when auto-detection would pick wrong
(rare - only when the visible text itself legitimately contains characters
like `(` or `|`).

**2. Plain-value shorthand (not regex-related)** - a bare value replaces a
single-field struct:

- `wait: 1500` (a number, milliseconds) instead of `wait: { ms: 1500 }`
- `ocr: "Continue"` instead of `ocr: { text: "Continue" }`

`copyTextFrom` and most other multi-field selector params have **no**
shorthand - always use the full object form for those.

**Omit default fields.** `index: 0` is the default; omit it entirely rather
than writing `index: 0`. Only write `index` when it is greater than 0 (i.e.
targeting the 2nd+ match). Do the same for any other field whose default is
already correct for the case at hand - do not restate defaults just to be
explicit.

```yaml
# Prefer (index 0 is default, omitted):
- tap:
    type: "Text"

# Over:
- tap:
    type: "Text"
    index: 0
```

After simplifying an existing flow to this style, run `validate --json` (and
`list --json` if selectors changed) to confirm the shorthand still resolves
to the same command shape you intended - do not assume the rewrite is
behavior-preserving without checking.

## Platform Gotchas

Real-device findings that are easy to misdiagnose as selector or tool bugs:

- **iOS real devices**: `clearState`, `setPermissions`, and `clearKeychain`
  are genuine no-ops on a real (non-jailbroken) device - Apple gives no API
  for one app to wipe another app's sandboxed data. Only full uninstall +
  reinstall actually resets state. Do not treat a "clean" flag as proof the
  app was reset; verify with a visible first-run indicator instead.
- **iOS accessible label vs. visible text**: an element's accessibility
  label/placeholder can differ from what is rendered on screen (e.g. a search
  bar shows "Tìm kiếm địa chỉ" visually but its real label is "Tìm kiếm địa
  điểm"). Always select from the hierarchy dump, never by guessing the label
  from a screenshot.
- **iOS back navigation**: the generic edge-swipe `back` gesture is
  unreliable per-screen. Some screens expose a real back-button element, but
  its accessible label is inconsistent (`"backIcon"` on one screen, `"arrow
  left"` on another, absent on others). Check the hierarchy dump per screen
  instead of hard-coding one label everywhere.
- **Android Flutter/Compose**: the semantics tree can lag one snapshot behind
  what is visibly rendered (a race, not a missing selector). If a
  known-correct selector fails once right after a screen transition, retry
  before switching to OCR/point.
- **Cross-platform duplicate text**: a subtitle or label that appears
  identically on two different screens makes `waitSee`/`see` a false-positive
  readiness signal - it can pass without actually confirming navigation.
  Always pick a readiness selector that exists on exactly one screen in the
  flow.
- **Inspector inside an embedded webview (Antigravity/VS Code fork IDEs)**: a
  newer Chromium enforces Private Network Access CORS, which can block the
  Inspector's `fetch()` calls to `localhost` from inside the webview even
  though the initial page load succeeds. This is already handled by
  `inspector/server.rs`'s `allow_private_network(true)`; if screenshots load
  but every API call fails with "Failed to fetch" on a custom-built CLI, check
  this CORS setting before assuming a device/connection problem - and check
  which `lumi-tester` binary the IDE actually resolves via PATH (a stale
  separately-installed copy is a more common cause than the code itself).

## Inspector Workflow

Start Inspector from the CLI:

```bash
lumi-tester inspect --platform android --device <serial> --port 9333
```

For Web:

```bash
lumi-tester inspect --platform web --port 9333
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

`suggest_selectors` works on every platform (Android/iOS/macOS/Windows/Web) -
it is backed by the same `SelectorScorer` the recorder and Inspector use, so
ranking, uniqueness, and `index` are computed from the real element list, not
guessed. Prefer it before manually reading a large hierarchy dump.

With a running Inspector, omit `outputDir`/`file` and it queries the live
session directly:

```text
suggest_selectors query=Login
suggest_selectors point=540,960
```

**Omit both `query` and `point` on a screen you have never seen before** - it
returns every matched element on the current screen (up to `limit`) with
bounds and a ranked selector each, so you can discover what is tappable
without already knowing a coordinate or a label to search for:

```text
suggest_selectors limit=30
```

With a saved hierarchy dump (e.g. a failure artifact), pass `outputDir`,
`file`, and the required `platform`:

```text
suggest_selectors outputDir=./output file=fail_login_cmd3_ui.xml platform=android query=Login
suggest_selectors outputDir=./output file=fail_login_cmd3_ui.xml platform=android point=540,960
```

Use `query` when you know visible text, resource id, content description, or
class. Use `point` when you know where the target appears in the screenshot.
The tool returns ranked selector candidates with YAML snippets and each
candidate's real `index` among duplicates - prefer the top result unless it is
clearly unstable, auto-generated-looking, or duplicated with a lower index
than expected.

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

For current Android app discovery:

```bash
adb devices -l
adb -s <serial> shell dumpsys window | rg -i 'mCurrentFocus|mFocusedApp|topResumed'
adb -s <serial> exec-out uiautomator dump /dev/tty
```

Use `content-desc` values from the dump as `accessibilityId`/`desc` selectors.

For installed Android package discovery:

```bash
adb -s <serial> shell pm list packages | rg -i '<app-or-company-name>'
adb -s <serial> shell cmd package resolve-activity --brief <package>
```

For iOS bundle id discovery:

```bash
xcrun simctl list devices
xcrun simctl listapps booted | rg -i 'CFBundleIdentifier|CFBundleDisplayName|CFBundleName'
idb list-apps --udid <udid>
```

For Web, use the actual target URL. If the app is local, start the dev server
first and wait for a stable DOM selector after `launchApp`.

For macOS app identity and frontmost app discovery:

```bash
mdls -name kMDItemCFBundleIdentifier /Applications/MyApp.app
osascript -e 'tell application "System Events" to get name of first application process whose frontmost is true'
lumi-tester doctor --platform macos --json
```

For Windows executable and foreground window discovery:

```powershell
powershell -NoProfile -Command "Get-Process | Where-Object MainWindowTitle | Select-Object ProcessName,Id,MainWindowTitle"
lumi-tester doctor --platform windows --json
```

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

## Desktop Selector Examples

Prefer Accessibility/UI Automation selectors over coordinates on native desktop
apps.

macOS and Windows text:

```yaml
- tap:
    text: "Preferences"
    exact: true
```

Role/type with index when labels repeat:

```yaml
- tap:
    role: "button"
    text: "OK"
```

```yaml
- tap:
    type: "Button"
    index: 1
```

When desktop hierarchy cannot expose the target, inspect the screenshot and use
screenshot/pixel assertions plus a documented `point` fallback. Do not use
`ocr` or `image` selectors for macOS/Windows desktop flows; they are not
implemented for macOS/Windows desktop drivers in the current runtime:

In short: `ocr`/`image` are not implemented for macOS/Windows desktop drivers.

```yaml
- tap:
    point: "50%,80%"
```

## Relative Selector Pattern

Use relative selectors when repeated rows share the same text/class and there
is a stable nearby label.

```yaml
- tap:
    type: "android.widget.Switch"
    rightOf:
      text: "Bedroom"
      exact: true
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
3. Confirm the XML package matches the flow `appId`.
4. If visible in XML, replace selector with `id`, `desc`/`accessibilityId`, or
   exact `text`.
5. If selector is duplicated, add `index`, `type`, or a relative anchor.
6. If visible only in screenshot, try `ocr` or `image`.
7. If element appears after delay, add `waitUntilVisible` before interaction.
8. If inside a list, use `scrollUntilVisible`.
9. Rerun only the failed command with `--command-index`.

## Anti-Patterns

- Do not start with `point` unless testing Android Auto/canvas.
- Do not replace a native `content-desc`/`id` selector with coordinates.
- Do not use translated text if stable ids/accessibility ids exist.
- Do not use broad regex like `.*Login.*` when exact text is stable.
- Do not fix selector failures by adding long `wait` first.
- Do not reuse a YAML appId when the user asked for the current app; inspect
  current focus first.
- Do not count command indexes manually; use `list --json`.
