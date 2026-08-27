# Changelog

All notable changes to this project will be documented in this file.

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
