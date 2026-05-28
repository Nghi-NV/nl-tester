# Native Desktop Drivers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add native `macos` and `windows` desktop app testing support to `lumi-tester`.

**Architecture:** Desktop support follows the existing `PlatformDriver` boundary. The parser accepts `platform: macos` and `platform: windows`, the runner selects a platform driver, and each desktop driver implements a dependency-light native MVP using OS commands/APIs without Appium.

**Tech Stack:** Rust, async-trait, `std::process::Command`, macOS Accessibility/System Events/screencapture, Windows PowerShell/Win32 shell commands, Windows UI Automation.

---

## File Map

- Modify `src/parser/types.rs`: add `Macos` and `Windows` platform enum variants.
- Modify `src/parser/yaml.rs`: add parser coverage for desktop platform headers.
- Modify `src/driver/mod.rs`: export desktop driver modules and list local desktop targets.
- Create `src/driver/macos/mod.rs`: macOS native driver implementation.
- Create `src/driver/windows/mod.rs`: Windows native driver implementation with non-Windows compile-safe fallback.
- Modify `src/runner/mod.rs`: instantiate desktop drivers from `platform` YAML or CLI argument.
- Modify `src/main.rs`: update CLI help, shell routing, and doctor platform handling.
- Create `docs/desktop-testing.md`: document supported commands, setup requirements, and limitations.

## MVP Behavior

- `launchApp`: macOS opens `.app`, bundle id, or path; Windows starts executable/path.
- `stopApp`: macOS quits app by name/bundle best effort; Windows stops process by executable stem.
- `tap`: supports `point` natively, element selectors best-effort through macOS `AXUIElement` and Windows UI Automation where available.
- `inputText`: types text into current focus.
- `pressKey`, `back`, `home`: map common desktop keys where reasonable.
- `takeScreenshot`, `compareScreenshot`, `getPixelColor`: use existing image utilities where possible.
- `dumpUiHierarchy`: return best-effort desktop accessibility/window information.
- Mobile-only features return clear unsupported errors.

## Tasks

### Task 1: Parser and Routing

**Files:**
- Modify `src/parser/types.rs`
- Modify `src/parser/yaml.rs`
- Modify `src/driver/mod.rs`
- Modify `src/runner/mod.rs`
- Modify `src/main.rs`

- [x] Write failing parser test for `platform: macos` and `platform: windows`.
- [x] Add `Platform::Macos` and `Platform::Windows`.
- [x] Wire driver exports and runner routing after driver modules exist.
- [x] Add doctor/shell/list-devices support for desktop platforms.

### Task 2: macOS Native Driver

**Files:**
- Create `src/driver/macos/mod.rs`

- [x] Implement `MacosDriver::new()`.
- [x] Implement `PlatformDriver` required methods for MVP behavior.
- [x] Keep mobile-only operations as explicit unsupported errors.
- [x] Add `AXUIElement` hierarchy for `text`, `id`, `description`, `role`, and `type` selectors.
- [x] Add real Calculator smoke coverage for `12+30=42`.
- [x] Verify `cargo check --locked` on macOS.

### Task 3: Windows Native Driver

**Files:**
- Create `src/driver/windows/mod.rs`

- [x] Implement `WindowsDriver::new()`.
- [x] Implement `PlatformDriver` required methods for MVP behavior.
- [x] Use host checks and fallback errors so macOS/Linux builds still compile.
- [x] Add foreground-window UI Automation hierarchy for `text`, `id`, `description`, `role`, and `type` selectors.
- [x] Add Windows UIA selector smoke YAML and CI wiring.
- [x] Verify `cargo check --locked` on non-Windows and document Windows verification command.
- [x] Route native GUI smoke CI to explicit self-hosted runners instead of hosted PR runners.
- [x] Add hosted `windows-latest` validation for Windows driver unit tests, `doctor --platform windows`, YAML validation, and PowerShell parser checks.
- [x] Extend `doctor --platform windows` to verify UI Automation assemblies, not just PowerShell availability.
- [ ] Verify Windows UIA smoke on an actual Windows host or GitHub Actions Windows runner.

### Task 4: Documentation and Self-Test

**Files:**
- Create `docs/desktop-testing.md`

- [x] Add sample YAML for macOS and Windows.
- [x] Document accessibility/security permissions.
- [x] Run `cargo test --locked`.
- [x] Run `cargo check --locked`.
- [x] Run a focused parser test: `cargo test parses_desktop_platform_headers --locked`.
- [x] Run real macOS AX selector smoke.
- [x] Run real macOS Calculator smoke with arithmetic assertion.
- [x] Validate Windows native and UIA smoke YAML on macOS.
- [x] Add a Windows PowerShell self-test script for `doctor`, validation, native smoke, and UIA selector smoke.
- [x] Add CI PowerShell parser coverage for the Windows desktop smoke script.
- [ ] Capture Windows runtime smoke evidence.

## Review Checklist

- Platform names are lowercase in YAML: `macos`, `windows`.
- Existing `android`, `android_auto`, `ios`, and `web` behavior remains unchanged.
- Native desktop drivers do not introduce Appium/Node dependencies.
- Unsupported commands fail with actionable messages instead of panics.
- Runtime completion is not proven until Windows smoke commands pass on Windows.
