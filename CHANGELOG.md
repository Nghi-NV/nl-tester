# Changelog

All notable changes to this project will be documented in this file.

## [v0.1.43] - 2026-09-25

### 🚀 Highlights & Improvements

#### 1. Flat Control Flow & Declarative DSL
- **Inline `when:` Action Modifier**: Attach `when:` directly to any command mapping (`tap`, `inputText`, `see`, `hwClick`, etc.) to eliminate nested `conditional` wrappers and keep flows 100% flat.
  - Supports UI visibility conditions (`when: { visible: "Accept Cookies" }`).
  - Supports JavaScript boolean expressions and environment variables (`when: "${ENV} == 'staging'"`).
  - Also works as a standalone block wrapper when grouping multiple actions.
- **Declarative Loops (`forEach`)**: Added `forEach:` (aliases: `for_each`, `foreach`) supporting iteration across:
  - Static lists of values (numbers, strings).
  - List of dictionaries/objects for data-driven testing (`${user.email}`, `${user.pass}`).
  - Dynamic context JSON arrays parsed from previous steps/API calls (`in: "${DISCOVERED_DEVICE_IDS}"`).
- **Pattern Branching (`match`)**: Added `match:` with flat `cases:` and `default:` mapping, replacing cumbersome procedural switch-case blocks without deep indentation.

#### 2. CLI Data-Driven Testing (`--data`)
- Added `--data <file.csv|json>` to `lumi-tester run`.
- Runs single-record test flows across entire datasets by automatically iterating each record in the CSV or JSON file and injecting row fields into the runtime context as `${column_name}` variables.

#### 3. Cross-Engine Variable Substitution
- Extended `${VAR}` variable substitution across all test commands, including hardware controls (`hwClick`, `hwPress`, `hwPowerOn`, `hwRotate`, `hwSeeLed`), navigation, and assertions.
- Enhanced context evaluation with unquoted identifier lookup fallback in expressions.

#### 4. Ecosystem & Peripheral Synchronization
- **VS Code Extension (`lumi-tester-vscode` v0.1.43)**: Added completion, snippets, and parameter hints for `when`, `forEach`, and `match`.
- **MCP Server (`lumi-tester-mcp` v0.1.8)**: Added `data` parameter to the `run_test` tool.
- **AI Skills**: Synchronized `commands.csv`, `cli.csv`, `command-catalog.md`, and `patterns.md` across Codex, Antigravity, and `.agents`.
- **Documentation**: Updated Command Catalog, CLI Reference, Writing Tests guide, and web docs.

## [v0.1.40] - 2026-09-10

### 🚀 Improvements

- **Every screenshot the CLI writes to disk is now lossless WebP, not PNG** - meaningfully lighter storage (~20-25% smaller on real device screenshots, verified live) with zero pixel loss, since this is debugging/comparison evidence where lossy compression would undermine the point:
  - `--snapshot` failure-evidence screenshots (`fail_*.webp`, was `.png`).
  - The `screenshot:` YAML command - the file is written as WebP regardless of the extension given in `path` (the actual extension is corrected to match the real bytes, so a `.png`-named path never silently contains WebP data).
  - Camera evidence frames (`raw`/`warped`/`annotated`/`crop_*.webp`, was `.png`).
  - `assertScreenshot: <name>` (no extension given) now looks up a `<name>.webp` baseline first, falling back to `<name>.png` - existing repos with committed PNG baselines keep working unchanged; an explicit `.png`/`.webp` extension in the testcase is always respected literally.
- New shared `utils::image_convert` module (`convert_to_webp_in_place`, `save_rgb_as_webp_lossless`, `guess_image_mime`) backing all of the above, with unit tests for the lossless round-trip, failure-leaves-original-untouched behavior, and MIME guessing.
- Fixed two report-generation spots (`report::html`, `report::summary_html`) that previously hard-coded `image/png` as the embedded-image MIME type regardless of the actual file - they now detect the real format from the file extension, so WebP evidence renders correctly in HTML reports.

### 📚 AI Skill Documentation

- Updated `debug-artifacts.md`, `command-catalog.md`, `commands.csv`, `patterns.md`, `desktop.md`, `android-auto.md`, and `selector-discovery.md` to reflect the WebP change (`fail_*.webp` instead of `.png`, corrected `screenshot`/`assertScreenshot` examples and lookup-order notes).
- Added a rule to `testcase-design.md`'s "YAML Authoring Rules From Testcases": every flow file must end by navigating back to one known, documented screen rather than assuming the next flow in the suite will re-establish state - found live when a flow that ended mid-navigation made the *next* flow's first assertion fail only when the suite ran end-to-end (not when each file ran alone). Verify with a full-suite run, not just per-file runs.

## [v0.1.39] - 2026-09-10

### 🐛 Fixes

- **Android `waitUntilVisible`/`waitUntilNotVisible` no longer abort on a single transient hierarchy-dump failure.** Found live while testing a real embedded smart-display app: `launchApp`'s `am start` on an already-foreground activity that is registered as the device's `HOME`/launcher (common on kiosk/smart-display devices) can leave hierarchy queries failing for 20+ seconds afterward even though the app displays correctly the whole time. `wait_for_element`/`wait_for_absence` previously propagated a single dump error as a hard failure (`?`), defeating the entire purpose of a polling wait loop; they now treat a transient dump error the same as "not visible yet" and keep polling until the real timeout, exactly like a normal not-yet-rendered screen.

### 📚 AI Skill Documentation

- Added a real "Systematic App Exploration Loop" to `references/testcase-design.md` - a concrete, tool-grounded process (BFS traversal using `suggest_selectors`, a written screen-map artifact, explicit budget/status tracking) for testing an app with no spec/existing tests, which is the common case for a real assignment, not an edge case. Previously this scenario was covered by a single vague sentence.
- Added self-critique amendments after dogfooding the loop against a real, previously-untested app: scroll-to-exhaust each screen before considering it catalogued (virtualized lists don't expose off-screen items), repeat the traversal per role/account-state rather than assuming one pass generalizes, a new "What Clicking Can't Discover" checklist (time-based behavior, feature flags, concurrency, third-party integration failures, accessibility, localization, performance, security), and re-running/diffing the screen map across app updates.
- New "Platform Gotchas" entries, all verified live against a real embedded Android smart-speaker app: apps registered as the device's `HOME`/launcher need special `launchApp` handling; D-pad-focus-navigable lists (common on smart-displays/set-top-boxes) make `swipe` silently no-op - use `pressKey: DPAD_DOWN/UP`; `--json` output is stdout-only, merging stderr with `2>&1` before parsing breaks it even though the tool works correctly; `suggest_selectors`'s `query` is a plain substring match, not `|`-alternation like the `tap`/`see` shorthand.

## [v0.1.38] - 2026-09-10

### 🚀 Highlights & Improvements

#### 1. Non-Intrusive Desktop Automation (macOS + Windows)
- **Windows now tries a non-intrusive UI Automation action first** (`InvokePattern`/`TogglePattern`/`SelectionItemPattern`/`ExpandCollapsePattern`) before falling back to a real `SetCursorPos`/`mouse_event` click - matching the pattern macOS's `tap` already used (`AXPress`). Windows previously always did a real physical click with no non-intrusive path at all.
- **Extended macOS's non-intrusive path** to `double_tap` (`AXOpen`) and `right_click` (`AXShowMenu`), not just `tap`.
- **Cursor restoration added to Windows**: every physical-fallback click now saves and restores the real cursor position afterward, matching macOS's existing behavior (which Windows previously had none of).
- **User-activity guard (both platforms)**: right before any physical-fallback click, samples the real cursor position twice ~80ms apart - if it's moving on its own, skips the click with a clear error instead of fighting a person actively using the mouse. Best-effort, not a hard guarantee: it can't protect actions where no non-intrusive path exists at all.
- **New `focusApp`/`switchApp: <target>` command** (macOS/Windows): switches which already-running app/window subsequent selectors resolve against, without launching it and **without bringing it to the foreground** - same non-disruptive philosophy as the click changes above. Verified live with positive and negative controls. On macOS this also reaches non-`.regular`-activation-policy processes like `com.apple.dock` by direct bundle-id match (excluded from normal app discovery, but reachable directly) - `focusApp: "com.apple.dock"` then a normal `tap`/`see` on a Dock icon works, verified live against the real Dock.

#### 2. Sensitive Text Redaction for Desktop Hierarchy Reads
- Visible text read back from macOS/Windows hierarchy dumps (Inspector, `suggest_selectors`, hierarchy dumps) is now redacted at the source - before it reaches any consumer - not filtered downstream. A real "secure field" signal from the OS (macOS `AXSecureTextField`) always wins; otherwise a shape heuristic catches password-like strings, digit runs ≥4 (including formatted numbers like `4242 4242 4242 4242` or `555-123-4567`), and email addresses, replacing them with a fixed placeholder (`***MASKED***`) rather than a partial/truncated value.
- New shared `utils::redact` module (separate from the existing `record`-command credential masking, whose tested output format is unchanged).

### 🐛 Fixes

- Live testing while building the redaction feature found and fixed a real gap: the initial digit-run heuristic missed formatted numbers because separators (`.`, `-`, `,`, space) reset the run count - a decimal/grouped number displayed on a real app (`512.345`) was not redacted. Separators inside an in-progress digit run no longer reset it.

### 🔧 Known Limitations (Documented, Not Yet Closed)
- Windows changes in this release are code-complete and unit-tested but not live-verified on a real Windows host (none available in this environment) - macOS changes were all verified live.
- Windows hierarchy dumps do not yet surface a native `IsPassword` signal, so Windows text redaction currently relies on the shape heuristic only.
- App menu bars (File/Edit/View) are not yet reachable via the non-intrusive action path - only each app's windows are walked, not its `AXMenuBar`.

## [v0.1.37] - 2026-09-10

### 🚀 Highlights & Improvements

#### 1. Cross-Platform `suggest_selectors`
- **Rewired onto the real scoring engine**: `suggest_selectors` (MCP tool, new `lumi-tester suggest-selectors` CLI subcommand, new `/api/suggest-selectors` Inspector endpoint) now shares the same `SelectorScorer` engine already used by `record` and `/api/element-at`, instead of a separate, Android-only, hand-rolled XML parser/scorer.
- **Fixes real bugs in the old implementation**: duplicate `text` across elements now gets a correct, real `index` per occurrence (was always unindexed/ambiguous); `type` selector candidates now get the element's true on-screen position among same-type siblings (was hardcoded `index: 0` regardless of position); resource-ids that look auto-generated are now deprioritized and marked unstable (were unconditionally scored 100).
- Works on Android, iOS, macOS, Windows, and Web - proven live on real Android and iOS devices (a real `tap:` selector generated by the tool, run via `lumi-tester run`, confirmed to hit the correct element) plus a full MCP stdio round-trip test.
- New CLI subcommand: `lumi-tester suggest-selectors --platform <p> [--device <d> | --file <dump>] [--query <q>] [--point <x,y>] [--json]` - works from a saved hierarchy dump even with no Inspector/device connected.

#### 2. New CLI Commands
- **`lumi-tester docs`**: opens the hosted documentation site in your default browser (`--print` to just print the URL).
- **`lumi-tester which`** (alias `where`): shows the currently-running binary's path, the binary `PATH` would resolve to (flags a mismatch - the exact root cause behind "I rebuilt it but the fix isn't showing up" style issues), and which AI skill/config directories exist.
- **`lumi-tester ai install`** now also installs the Antigravity skill, at both the workspace level (`./.agents/skills/lumi-tester-agent`) and the global level (`~/.gemini/config/skills/lumi-tester-agent`), alongside the existing Codex and Claude Code installs.

#### 3. AI Skill Documentation
- Rewrote the AI skill reference set (`SKILL.md` + `references/*`) against the real Rust parser/runner source: fixed a real contradiction in selector-priority ordering, documented the exact bare-string shorthand rules (which commands auto-detect `regex` vs `text`, which have a separate plain-value shorthand, which have none), and added a "Platform Gotchas" section capturing verified real-device findings (iOS state-reset no-ops, accessible-label-vs-visible-text mismatches, Flutter/Compose semantics-tree races).
- Documented real, previously-unwritten runner behavior found while verifying the above: `setup.yaml`/`teardown.yaml` run **once** for the whole batch of files passed to `run` (not once per file), do not cascade into nested feature folders, and are scoped only to the exact directory passed to `run`; `subflows/` and `screens/` are excluded from `run`'s file collection but *not* from `validate`/`list`.
- Added environment-variable/secrets guidance (`${VAR}` + real env export per shell, or header `env: { file: ".env" }`) to both the AI skill docs and the human-facing docs site.
- Kept `SKILL.md` itself lean by moving long topic-specific detail into `references/selector-discovery.md` and `references/testcase-design.md`; synced the previously-drifted `antigravity-skill/` and `.agents/skills/` copies to match the canonical `codex-skill/` bundle.

### 🐛 Fixes

- **Docs site**: fixed a Mermaid diagram in `docs/flows/test_execution_flow.md` that failed to render on GitHub (unquoted labels/old-style subgraph syntax), and corrected its (and the surrounding prose's) claim that Setup/Teardown run once per test file - they run once per `run` invocation, wrapping the whole collected batch. Added a worked `setup.yaml`/`teardown.yaml` example.
- Untracked `__pycache__/*.pyc` files that had been committed to git (`.gitignore` already covered them going forward, but pre-existing tracked copies weren't removed until now).

## [v0.1.36] - 2026-09-10

### 🚀 Highlights & Improvements

#### 1. Inspector: Hex Color Picker
- **Click-to-Sample Color**: Clicking anywhere on the device screenshot now also reads the pixel color at that point (via an offscreen sampling canvas kept in sync with every capture) and shows it as a swatch + hex code in the status bar, next to "Selected X at (x, y)".
- **Click to Copy**: Clicking the swatch/hex badge copies the hex code, using the same reliable dual-path copy (native `vscode.env.clipboard` via the extension host, not just the browser's `navigator.clipboard`) as the existing selector-copy feature - the browser API alone is unreliable inside an embedded webview iframe.

### 🐛 Fixes

#### 1. Inspector Reliability
- **Stale Screenshot on Capture Failure**: A failed screen capture used to leave whatever image was already on screen untouched, indistinguishable from a genuinely successful-but-stale frame - now clears it and shows a clear "Screen Capture Failed" state with the real error reason.
- **Private Network Access CORS**: Newer Chromium-based hosts (confirmed live: Antigravity IDE) enforce a CORS preflight before allowing a page to `fetch()` a private-network address like `localhost` - the Inspector's server now opts in (`allow_private_network(true)`), fixing "Failed to fetch" errors that only affected JS-initiated requests (the initial iframe navigation was never gated by this) and only showed up on certain Chromium-based editors, not upstream VS Code.
- **Device-Switch Iframe Cache-Busting**: Added a cache-busting nonce to the Inspector iframe's URL on every device switch to rule out the webview reusing a previous navigation's DOM.

#### 2. iOS Driver
- **Mock Location Speed Control**: The VS Code extension's GPS Speed Control panel writes live speed/pause adjustments to `/tmp/lumi-gps-control.json` - Android's `mockLocation` playback already read this file, but iOS's never did, making the speed slider a silent no-op for every iOS run. iOS now reads it too.

---

## [v0.1.35] - 2026-09-01

### 🐛 Fixes

#### 1. iOS Driver Reliability
- **Placeholder Text Matching**: Text selectors now also match a `TextField`'s `placeholder` (hint text), not just its label/name/value - fixes taps on unfocused fields (e.g. search boxes) whose only visible text was the hint.
- **Post-Launch Agent Readiness**: `launchApp` now polls the on-device agent's own readiness before returning, instead of assuming it's immediately available - fixes intermittent "agent is not reachable" failures on the command right after a `clearState` launch under system load.
- **Login Submit Tap Retry**: Retries the final login-submit tap once if the login screen is still showing afterward - guards against the same Flutter gesture-arena tap drop already mitigated elsewhere in the driver.

#### 2. lm-ios-tester Agent
- **Tap Duration**: Increased the synthesized tap's touch-down/up duration from 50ms to 300ms - a short touch could intermittently lose Flutter's gesture-arena resolution and never fire the widget's tap handler, even though the touch itself reported success. 300ms stays well under the ~500ms long-press threshold. Bundled the updated agent into the auto-extracted zip shipped with the CLI.

---

## [v0.1.34] - 2026-08-28

### 🚀 Highlights & Improvements

#### 1. Inspector & Extension Device Bar Integration
- **Dynamic Device Header Bar**: Added a status bar in Element Inspector displaying the connected device name and platform badge, with a 1-click "Switch Device" button.
- **Dynamic Device Switching**: Changing target devices in VS Code status bar immediately refreshes and reconnects active Inspector panels.
- **Target App Auto-Attach**: Passes `appId` and `platform` from YAML files into the Inspector iframe to auto-select and highlight the target application package.
- **Authoritative Platform Warning**: Surfaces helpful warnings when the VS Code selected device conflicts with the flow's authoritative `platform:` declaration.

#### 2. Zero-Setup iOS Automation (Auto-Extracted lm-ios-tester)
- **Embedded iOS Agent Project**: Automatically unpacks and manages `lm-ios-tester` Xcode project to `~/.lumi-tester/lm-ios-tester/` on-demand (parity with Android APK automatic extraction).
- **iOS Simulator / Real Device Enhancements**: Fixed runloop handling, physical device volume buttons, and process lifecycle stability.

---

## [v0.1.33] - 2026-08-27

### 🚀 Highlights & Improvements

#### 1. Adaptive Terminal Resize & Non-Overlapping Progress Rendering
- **Dynamic Auto-Sizing (`wide_bar`)**: Upgraded CLI updater progress bar to use `{wide_bar}` and throttled redraw rates (`stdout_with_hz(15)`), automatically adjusting bar width to terminal dimensions without line-wrapping or duplicate output lines when resizing terminal windows.
- **Strict TTY / Non-TTY Segmentation**: Completely isolates interactive visual bars from headless/CI milestone logs (`[25%]`, `[50%]`, `[75%]`, `[100%]`), ensuring clean single-line output everywhere.

---

## [v0.1.32] - 2026-08-27

### 🚀 Highlights & Improvements

#### 1. Smart OCR Fallback for Flutter Anomalous Bounds
- **Near-Full-Screen Bounds Detection**: Detects Flutter/dynamic framework semantics anomalies where a container or scrollable region accidentally inherits a child button's label (`content-desc`) while retaining near-full-screen bounds ($\ge 90\%$ screen area).
- **Automated Visual OCR Fallback**: When an anomalous text/regex match is detected, the driver automatically performs an on-screen OCR visual scan to locate the exact rendered pixel coordinates of the button, ensuring taps land precisely on the target element instead of empty screen space.

---

## [v0.1.31] - 2026-08-27

### 🚀 Highlights & Improvements

#### 1. Zero-Stale UI Hierarchy Architecture (Android & iOS)
- **Eliminated UI Hierarchy Caching**: Completely removed stale UI cache locks across both `AndroidDriver` and `IosDriver`. Every visibility check, assertion, and tap/input interaction now performs a fresh, real-time hierarchy dump (`get_ui_hierarchy()`).
- **Flutter & Dynamic UI Precision**: Fixes race conditions and phantom misses in dynamic frameworks (Flutter, React Native, SwiftUI, Jetpack Compose) where the accessibility semantics tree updates asynchronously after navigation or animation.
- **Fast-Path Agent Performance**: Leverages the high-speed in-process `lm-android-tester` (~10-20ms) and `lm-ios-tester` (~100ms) agents for maximum execution speed without compromising selector accuracy.

---

## [v0.1.30] - 2026-08-27

### 🚀 Highlights & Improvements

#### 1. Unified Light-Theme Summary Report & Interactive Dashboard
- **Modern Light-Theme Test Summary (`summary.html`)**: Transitioned default single-session reporting to a sleek, modern light-themed `summary.html` report with rich step-by-step telemetry, embedded screenshot diffs, and structured timing metrics.
- **Enhanced Sessions History Dashboard (`output/index.html`)**: Added interactive date range filtering (`Từ` / `Đến`), pass rate trends, and one-click access directly to each test session's summary report.
- **Robust Report Re-generation**: Added `generated_at` timestamp metadata ensuring `lumi-tester report <file>` deserializes and regenerates reports seamlessly.

#### 2. Android & iOS Driver Reliability
- **Flutter & Async Hierarchy Cache Fix**: Enhanced `wait_for_element` on Android to invalidate cache on every poll cycle, completely preventing stale tap coordinates caused by asynchronous semantics tree lags in Flutter apps.
- **Multi-Device Android Agent Port Routing**: Dynamically routes TCP agent socket connections via `agent_port_for(serial)` across multiple connected physical devices / emulators.
- **iOS Agent Lifecycle Enhancements**: Improved WebDriverAgent setup, connection verification, and device lifecycle handling.

#### 3. Full Batch Execution Resilience (`--continue-on-failure`)
- **Fatal Init Error Handling**: `continue_on_failure` now catches and logs fatal file/infrastructure errors (such as disconnected hardware jigs or invalid file targets) without halting the remainder of the directory test batch.

#### 4. VS Code Extension & Inspector
- **Auto Device Selection Prompt**: Automatically prompts user to select target device when opening Element Inspector if multiple mobile devices are connected.
- **WebP Image Support**: Upgraded image processing pipeline to support WebP formats alongside GIF and PNG.

---

## [v0.1.29] - 2026-08-24

### 🚀 Highlights & Improvements

#### 1. Real-Time Download Progress Indicator Reliability
- **Explicit Stdout Progress Rendering**: Configured `ProgressBar` with `ProgressDrawTarget::stdout()` and enabled steady ticking (`80ms`), ensuring streaming progress bars render smoothly on all terminals (Windows CMD, PowerShell, Git Bash, macOS zsh, Linux bash).
- **Dual TTY & Non-TTY Fallback Logging**: Automatically outputs periodic percentage checkpoints (`[25%]`, `[50%]`, `[75%]`, `[100%]`) when running in redirected streams, CI environments, or background jobs.
- **Paced Installation Transition**: Added visual pacing across permissions, binary swap, and completion steps so processing percentage indicators remain clearly readable.

---

## [v0.1.28] - 2026-08-24

### 🚀 Highlights & Improvements

#### 1. Zero-Setup Android UI Automation (Automatic Embedded Agent Unpacking)
- **Automatic On-Demand Agent Extraction**: `find_apk` now automatically extracts the embedded `lm-android-tester.apk` to `~/.lumi-tester/apk/lm-android-tester.apk` (or temp directory) if not already present on disk across Windows, macOS, and Linux. This eliminates the "agent APK not found locally" warning completely on fresh installations.
- **Drag Point Coordinate Support**: Fully supports percentage and absolute point coordinates (`point: "28%,45%"`) in `from` and `to` selectors of `drag` commands for smooth continuous wheel/slider control.
- **Automatic macOS Codesigning on Upgrade**: Auto-applies ad-hoc codesign signature (`codesign -s - -f`) when replacing binaries on macOS arm64.

---

## [v0.1.27] - 2026-08-24

### 🚀 Highlights & Improvements

#### 1. Embedded Android Agent APK & Binary Resolution
- **Embedded Agent APK**: Bundled `lm-android-tester.apk` directly inside the CLI binary using compile-time embedding (`include_bytes!`), ensuring reliable fast UI automation on machines installed via official packaging without relying on loose source files.
- **Nested APK Discovery for Windows**: Added support for nested Tauri bundle resource directories (`resources/resources/apk/`).

#### 2. Enhanced Upgrade CLI with Progress Indicators & Multi-IDE Extension Installer
- **Real-Time Progress Bars**: Added percentage, speed, and elapsed time indicators for CLI binary download and VSIX extension download/installation.
- **Multi-IDE Auto-Discovery & Installation**: Automatically detects and installs the Lumi Tester extension into all available IDEs (VS Code, Antigravity IDE, Cursor, Windsurf, VSCodium, VS Code Insiders) across Windows, macOS, and Linux.

---

## [v0.1.26] - 2026-08-24

### 🚀 Highlights & Improvements

#### 1. Hardware-Native Blink Detection (`hwSeeNativeLedBlink`)
- **STM32 Hardware-Timed Blink Counter**: Implemented `hwSeeNativeLedBlink` polling the firmware's real-time hardware blink event log (`color blink_cursor?` / `color blink?`) instead of sampling RGBC over serial and edge-detecting client-side.
- **Zero Sycall Jitter / Dropped Pulses**: Eliminates host-side serial polling overhead (especially under Windows COM drivers), ensuring 100% reliable pulse counting using firmware-calibrated Flash thresholds.

#### 2. Cross-Platform Element Inspector & Driver Enhancements
- **macOS Desktop Inspector**: Added fast active/frontmost window traversal with `CGWindowList` fallback to inspect running apps without deep recursive AX hangs.
- **Android Inspector Bounding Box Resolution**: Fixed serial/package target mapping ensuring precise element bounds, breadcrumb hierarchy, and selector scoring.
- **Web & iOS Inspector Support**: Seamless element hierarchy extraction across Web, macOS, Android, and iOS.

#### 3. Serial Transport Throughput Optimization
- **Chunked Serial Buffer**: Switched serial response reading from byte-by-byte syscalls to chunked buffers (`[0u8; 512]`), significantly reducing read latency and OS overhead.

---

## [v0.1.25] - 2026-08-23

### 🚀 Highlights & Improvements

#### 1. Adaptive Port Forward Reconnection for Android Agent
- **Instant Port Forward Recovery**: Retries `adb forward` socket mapping up to 3 times before attempting full agent restarts, resolving stale connection states across consecutive test runs in ~200ms without restarting the on-device process.

#### 2. Animation-Resilient Focus & Text Input Retries
- **Last Tap Point Tracking**: Records exact tap coordinates to recover from mid-animation taps (e.g. Flutter/React Native/Compose entrance transitions) where accessibility semantics become available before the render tree is hit-testable.
- **Adaptive Re-Tap Backoff**: Automatically re-taps the target field with backoff intervals (300ms, 600ms) if `set_text` fails to find a focused field, dramatically increasing text input reliability on animated UI.

---

## [v0.1.24] - 2026-08-23

### 🚀 Highlights & Improvements

#### 1. High-Speed Android Execution via `lm-android-tester` Agent Service
- **Real-Time On-Device Agent**: Integrated `lm-android-tester` agent service to bypass slow ADB process spawn and file I/O overhead.
- **Ultra-Fast UI Hierarchy & Text Input**: Substantially accelerates hierarchy retrieval, text input, full-field erasing, and keyboard management without multi-second IME polling loops.
- **Automatic Fallback Safety**: Maintains full fallback to standard ADB commands when the agent service is unavailable.
- **Massive Performance Boost**: Cuts test execution time on complex flows (e.g. login, forms, navigation) from 20-30s down to 3-5s while preserving 100% test accuracy.

#### 2. iOS & WDA Driver Optimizations
- **WDA JSON Source Parsing**: Direct support for WDA JSON source format alongside XML hierarchy parsing.
- **Enhanced Coordinate & Accessibility Matching**: Improved element matching speed and stability.

---

## [v0.1.23] - 2026-08-22

### 🚀 Highlights & Improvements

#### 1. Dynamic Ambient Baseline Calibration & Robust LED Blink Detection
- **Dynamic Ambient Baseline (Delta RGBC)**: Implemented adaptive baseline sampling before blink sequences (`hwSeeLedBlink`, `hwSeeLed`) to calculate $\Delta R, \Delta G, \Delta B, \Delta C$ relative to ambient room illumination.
- **Eliminated Hardcoded OFF Thresholds**: Replaced fixed Clear-channel thresholds with adaptive Delta-based optical energy detection across all color sensor commands (`read_color`, `verify_color`, `wait_for_color`, `wait_for_blink`).
- **Rich Diagnostic & Per-Pulse Breakdown**: Detailed real-time logging of each detected blink pulse with exact duration, peak RGBC, ambient baseline, and Delta optical deltas.
- **Enhanced Pink/Magenta Optical Matching**: Tuned color classification for RGB diffuser LEDs with low saturation or high ambient bleed.

---

## [v0.1.22] - 2026-08-22

### 🚀 Highlights & Improvements

#### 1. Interactive UI Hierarchy Bounding Box Visual Inspector
- **Overlay Bounding Boxes on Failure Screenshots**: Extracts element bounds from UI hierarchy XML and renders interactive, color-coded bounding boxes directly on top of failure screenshots.
- **Smart Color Coding**:
  - 🟢 **Green (Emerald)**: Elements containing text labels.
  - 🔵 **Blue / Cyan**: Clickable / interactive elements (`clickable=true`).
  - 🟣 **Purple**: View containers and structural layouts.
- **Rich Hover Tooltips**: Hovering on any bounding box reveals element Text, Resource-ID, Class name, and pixel Bounds dimensions (`[left, top][right, bottom]` and `WxH`).
- **Interactive Element Sidebar & Real-time Filter**: Side panel listing all detected UI elements with instant search by text, ID, or class, with bidirectional hover/click synchronization.
- **Overlay Controls**: Toggle button (`👁️ Bounding Boxes`) to easily show or hide bounding boxes.

#### 2. Self-Contained Base64 Failure Screenshot Embedding
- **Embedded Base64 Data URIs**: Encodes failure screenshots as base64 data URIs directly inside HTML reports (`report.html`), eliminating broken relative paths across nested session folders (`./output/<serial>/sessions/...`).
- **100% Standalone Reports**: Reports can now be viewed anywhere, emailed, or uploaded as CI/CD artifacts without losing screenshot evidence.

#### 3. Fixed Sessions Dashboard False-Failure Parsing
- **Support CamelCase & SnakeCase**: Corrected parsing in `generate_sessions_dashboard` to handle both naming conventions in `session.json`.
- **Accurate Pass/Fail Determination**: Ensured all-passed test runs are marked as `PASSED` instead of false failures.

---

## [v0.1.21] - 2026-08-22

### 🚀 Highlights & Improvements

#### 1. Human-Readable Session IDs & Timestamped Folder Organization
- **Replaced Random UUIDs with ISO Timestamps**: Session directories and IDs are now structured as `session_<target_or_flow>_YYYY-MM-DD_HH-MM-SS` (e.g. `session_slider_2026-08-22_10-40-44`).
- **Easy Sorting & Identification**: Multiple test runs no longer create confusing random UUID folders; users can instantly sort, filter, and identify sessions chronologically by flow name.

#### 2. Test Sessions History Dashboard (`output/index.html`)
- **Centralized Overview Hub**: Automatically generates and updates `output/index.html` and `output/sessions/index.html` across all historical sessions.
- **Interactive Metrics**: Live filtering by status (All, Passed, Failed), instant text search, and direct links (`View Report ↗`) to open individual session reports.
- **Flow Reliability & Stability Breakdown**: Aggregates statistics per test flow across all recorded sessions to highlight `STABLE`, `FLAKY`, or `FAILING` test suites.

#### 3. Rich Failure Inspector & Evidence Viewer in HTML Reports
- **Inline Failure Screenshot**: Embeds failure screenshot thumbnails with full-size click-to-zoom modal support.
- **Interactive UI Hierarchy XML Viewer**: Collapsible `<details>` container rendering the raw UI hierarchy XML at the exact step of failure.
- **Device System Logs**: Embeds recent device crash and system logcat snippets at failure points.
- **Retry Count Badge**: Displays explicit `↻ Retried N time(s)` indicators for commands configured with automatic retries.
- **Flow Execution & Stability Matrix**: Real-time pass rate and flakiness metrics across multi-run / `--repeat` flows.

#### 4. Clickable Terminal Output Links
- **1-Click Browser Opening**: Final executor output prints absolute `file://` scheme URLs to JSON reports, latest HTML report, individual session reports, and the Sessions Dashboard for direct Cmd+Click / Ctrl+Click opening.

---

## [v0.1.20] - 2026-08-22

### 🚀 Highlights & Improvements

#### 1. Dynamic Screen Resolution & Robust Android Relative Selectors (`above`, `below`, `rightOf`, `leftOf`)
- **Dynamic Screen Resolution Resolution**: Fixed hardcoded screen dimensions in UIAutomator relative search by passing the device's actual screen resolution (`self.screen_size`) retrieved dynamically via ADB.
- **Support for High-DPI & Wide-Screen Devices**: Prevents elements on 1440p (QHD+) or large-screen Android devices from being falsely flagged and filtered as oversized background containers.
- **Refined Container Filtering**: Preserves thin, full-width UI components (e.g. Sliders, SeekBars, ProgressBars) whose width spans across the display (`width > 95%`) while maintaining protection against actual background layout containers (`height > 25%`).

---

## [v0.1.19] / [extension-v0.1.31] - 2026-08-20

### 🚀 Highlights & Features

#### 1. Dynamic Hardware Jig Button & Relay Mapping (`buttons:`, `relays:`)
- **Semantic Button Names in Flows**: Test authors can now use friendly button names (`NC1`, `NC2`, `NC3`, `mainPower`, `220V`) directly in test YAML flows instead of memorizing physical pin numbers.
- **Decoupled Servo & Sensor Channels**:
  - Each named button (e.g. `NC3`) in `jig_profile.yaml` can independently define its physical `servo:` channel and optical `sensor:` channel.
  - Servo commands (`hwClick`, `hwPress`, `hwRelease`, `hwRotate`, `hwRepeatClick`) automatically resolve to the configured servo channel.
  - Optical sensor commands (`hwReadColor`, `hwSeeLed`, `hwSeeLedBlink`, `hwSeeLedOff`, `hwSensorLight`, `hwReadSensorLight`) automatically resolve to the configured sensor channel.
- **Relay Group Mapping**: Support mapping friendly labels (e.g. `220V`) to multi-relay arrays (`[3, 4]`) for concurrent multi-channel power operations (`hwPowerOn`, `hwPowerOff`, `hwPowerCycle`).

#### 2. Enhanced Color Sensor Diagnostics & Red Hue Boundary
- **Detailed Timeout Diagnostics**: When `hwSeeLed` or `wait_for_color` times out, output now explicitly includes the expected color, the actual detected color, and raw RGBC sample data (e.g. `Timeout (3.0s) waiting for expected color [BLUE] on channel 6 (current actual: RED, RGBC=[R:130 G:78 B:64 C:222])`).
- **Hue Boundary Tuning**: Fine-tuned Red LED hue boundaries ($0..28^\circ$) in smart color classification for higher accuracy with warm LED emitters.
- **Illumination Control**: Fixed PB15 sensor light LED synchronization (`hwSensorLight`).

#### 3. VS Code Extension `v0.1.31`
- **Jig Profile Auto-Completion & Hover Resolver**: Full hover inspection and auto-completion for semantic button names (`NC1`, `NC2`, `NC3`) and relay groups (`220V`) defined in referenced Jig profile YAMLs.
- **Built & Packaged**: `lumi-tester-0.1.31.vsix`.

---

## [v0.1.18] / [extension-v0.1.28] - 2026-08-19

### 🚀 Highlights & Features

#### 1. Continuous Drag & Slider Control (`drag`)
- **Universal Multi-Platform Support**: Added `drag` command across **Android** (ADB drag gestures), **iOS** (WDA/idb drag), **Web** (Playwright mouse continuous actions), **macOS** (MacosBridge drag), and **Windows** (UIAutomation drag).
- **Flexible Drag Points**: Supports dragging from/to semantic selectors, relative positioning, offsets, and coordinates with customizable `duration` (ms).
- **Seekbar & Progress Control**: Easily control continuous UI sliders (e.g. brightness, volume, seekbars, reorderable lists).

#### 2. Relative Positioning & Sibling Indexing (`below`, `above`, `rightOf`, `leftOf`)
- **Flutter & Compose Label Discovery**: Automatically detects `content_desc` labels (e.g. `"30%"`, `"Brightness"`) as valid relative anchor points alongside standard `text`.
- **Automatic Relative Index Calculation**:
  - Distance-based sorting from anchors.
  - Automatically emits `index: N` if and only if multiple matching sibling elements exist (`index > 0`), keeping `index == 0` YAML minimal and clean.
- **Inspector UI Enhancement**: Displays clear relation titles on relative cards (e.g. `type: View, below: "30%" (index 1)`).

#### 3. Standard Hardware Jig Profile (`profiles/jig_config.yaml`) & Flexible Color Assertions
- **Standard Profile**: Created [`profiles/jig_config.yaml`](file:///Users/nghinguyen/Desktop/MyOpenSource/nl-tester/profiles/jig_config.yaml) containing complete connection parameters and Servo channel definitions.
- **Flexible `hwSeeLed`**: Accepts both single string (e.g. `expected: "BLUE"`) and string arrays (e.g. `expected: ["BLUE", "GREEN"]`).

#### 4. VS Code Extension `v0.1.28`
- **Hierarchical Auto-Completion**: Unrestricted completion on all keystrokes with nested parameter tree resolution (`drag.from`, `drag.to`, `scrollable`, `permissions`, etc.).
- **Reusable `SELECTOR_PARAMS`**: Schema updated with recursive sub-properties.
- **Built & Packaged**: `lumi-tester-0.1.28.vsix`.

---

## [v0.1.17] / [extension-v0.1.25] - 2026-08-19

### 🚀 Highlights & Features

#### 1. Resilient Hardware Serial Communication & Dynamic RS485 Addressing
- **Serial Line Stabilization**: Added 100ms startup line stabilization delay and full buffer flush upon opening serial ports, preventing MCU DTR reset noise on Windows and STM32 Virtual COM.
- **Dynamic RS485 Multi-drop Addressing (`nodeId`)**:
  - Automatically prefixes wire commands with `@{node_id} ` (defaults to Node 1).
  - Configurable via YAML Header (`nodeId: 2`), Profile (`nodeId: 2`), and CLI (`lumi-tester jig ping COM5 --node 2`).
  - Response parser dynamically strips and extracts addressed node IDs from firmware output.
- **Wire Framing Template Engine (`wireFormat`)**:
  - Customizable wire framing template in Jig profiles (`wireFormat: "@{node} {command}\n"`).
  - Allows seamless adaptation to future firmware protocol format changes (`[NODE:{node}] {command}`, `NODE#{node}>{command}`, etc.) without altering any test YAML flow.

#### 2. VS Code Extension `v0.1.25`
- Added `nodeId` and `wireFormat` parameters to `hwConnect` autocomplete schema and snippet suggestions.
- Added `Lumi: Check for Updates` and `Lumi: Update CLI & Extension` commands to Command Palette.
- Integrated automated marketplace publishing pipeline via GitHub Actions using `secrets.VSCE_PAT`.

#### 3. In-Place Self-Update & Version Checking CLI (`lumi-tester update` & `lumi-tester version`)
- **Direct CLI Self-Update**: Added `lumi-tester update` (aliases: `self-update`, `upgrade`) to download and replace binary in-place from GitHub Releases across macOS, Linux, and Windows without manual downloads.
- **Cross-Component Version Checker**: Added `lumi-tester version` and `lumi-tester update --check` with machine-readable `--json` to inspect installed vs latest GitHub releases for both CLI and VS Code Extension.
- **Extension Update Support**: Added `lumi-tester update --extension` / `--all` to automatically fetch `.vsix` and install it via `code --install-extension`.

---

## [v0.1.16] / [extension-v0.1.24] - 2026-08-19

### 🚀 Highlights & Features

#### 1. Hardware Automation Standardization (`hw*`)
- **Normalized Prefix**: Standardized all hardware interaction commands with `hw*` prefix (e.g. `hwClick`, `hwPress`, `hwRelease`, `hwPowerOn`, `hwSeeLedBlink`, `hwSensorLight`, `hwReadServo`, etc.), removing redundant aliases.
- **Shared Reusable Jig Profiles & Servos**:
  - Declare shared Jig and Servo configuration in YAML header: `jig: "profiles/jig_switch_sample.yaml"`.
  - Automatic servo calibration loading (`pressAngle`, `releaseAngle`, `pressDurationMs`) on flow startup.
  - Automatic environment variable fallback resolution (e.g. `${JIG_PORT:-COM5}`).
- **Advanced LED Blink & Sensor Verification**:
  - Added pulse duration filtering (`minPulseMs`, `maxPulseMs`, `maxGapMs`) matching `app_desktop` TCS34725 capabilities.
  - Auto I2C MUX channel switching when reading color or blinking patterns.
- **Hardware Safety Lifecycle**:
  - Automatic safe state enforcement (`ctrl.enter_safe_state()`) on test completion, failure, or teardown.

#### 2. Fast COM Port Discovery & Ping Tools
- **CLI Commands**:
  - `lumi-tester jig ports` / `lumi-tester jig ports --json`: Fast enumeration of all connected Serial / COM ports.
  - `lumi-tester jig ping <port_or_profile>`: Quick connectivity and firmware ping check.
- **VS Code Extension `v0.1.24`**:
  - Added `Lumi: Detect Hardware Jig Ports` with 1-click QuickPick to Ping, Copy, or Insert into Active YAML Header.
  - Added `Lumi: Ping Hardware Jig` with instant status notification.
  - Enhanced error diagnostics with detailed reasons (`└─ Error: ...`) and port failure descriptions.

---

## [v0.1.15] / [extension-v0.1.23] - 2026-08-17

### 🚀 Highlights & Features

#### 1. Sub-Element Positioning (`align` & `offset`)
- **Semantic Alignment Presets**: Support `align: left | right | top | bottom | center` for targeting sub-elements within composite element bounds (e.g. toggle switches on list item rows, buttons on card edges).
  - Presets: `left` (10%, 50%), `right` (90%, 50%), `top` (50%, 10%), `bottom` (50%, 90%), `center` (50%, 50%).
- **Relative Percentage Offsets**: Support `offset: "X%,Y%"` (e.g. `offset: "85%,50%"`) relative to element bounds.
- **Universal Command Support**: Available across interaction commands (`tap`, `tapOn`, `doubleTap`, `longPress`, `rightClick`).
- **Lumi Inspector Smart Suggestions**: Inspector automatically detects off-center clicks on elements and suggests `align` and `offset` candidate selectors.

#### 2. Flexible Test Flow Execution
- **Run to End (`--from-command-index`)**: Added CLI option `--from-command-index <usize>` (aliases: `--from-index`, `--start-from`) to run tests starting from index `N` to the end of the file.
- **Test File Repetition (`--repeat`)**: Added CLI option `--repeat <N>` to run full test flows repeatedly for stability and soak testing.
- **VS Code Play from Here (`▶ Run from [i]`)**: Added a CodeLens button next to `▷ Run [i]` in VS Code to execute from any command to the end of the file.

#### 3. Ecosystem & Tooling Updates
- **VS Code Extension `v0.1.23`**:
  - CodeLens buttons for `▷ Run [i]` and `▶ Run from [i]`.
  - Autocomplete & hover schemas updated for `align` and `offset`.
  - Added `lumi-tester.runFromCommand` command.
- **JSON Schema**: Updated `lumi-test.schema.json` with `align` and `offset` definitions.
- **AI Agent Guidelines**: Updated `AGENTS.md`, `SKILL.md`, `selectors.csv`, and `selector-discovery.md` with sub-element positioning priorities.
- **Documentation**: Updated `api/commands.md`, `writing_tests.md`, `ai-authoring.md`, and re-generated GitHub Pages HTML.

---

## [v0.1.14] - 2026-08-04
- Initial release with Android, iOS, Web, macOS, Windows, and Hardware Jig automation support.
