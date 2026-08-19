# Changelog

All notable changes to this project will be documented in this file.

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
