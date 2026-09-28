# Lumi IDE implementation roadmap

**Status:** active implementation. The working tree now contains the first usable Lumi IDE vertical slice, but release acceptance is not complete.

## Product outcome

Build an independent, lightweight Tauri desktop IDE with a familiar VS Code-style workbench and Lumi-first support for workspace editing, YAML authoring, inspection, test execution, reports, terminal, local AI, and Lumi extensions. The installed app must not require VS Code, Node.js, or Rust/Cargo at runtime. Keep the CLI and VS Code extension usable.

## Delivered in the current working tree

- [x] Tauri 2 + React/TypeScript + Monaco workbench with Activity Bar, Explorer, tabs, bottom panel, status bar, command palette, keyboard shortcut, and Lumi IDE branding.
- [x] Rust workspace boundary for file listing and CRUD, lazy directory expansion, `.gitignore`, visible dotfiles, traversal checks, and out-of-root symlink rejection.
- [x] Workspace text search from the Activity Bar and `⌘⇧F`/`Ctrl+Shift+F`, honoring `.gitignore`, including hidden files, and opening the matching file at its line and column.
- [x] Dirty editor buffers, explicit save/save-all/discard actions, and save/discard prompts for workspace and app closing.
- [x] YAML path links for flow, command, and file-valued environment/variable paths.
- [x] Monaco command/selector metadata imported from the VS Code extension, completion, hover, parser diagnostics, and Lumi validation.
- [x] Lumi Inspector hosted from the Rust engine behind a localhost-only session.
- [x] Test Explorer, per-run output directories, cancellation, final engine summary, reports that respect failed/skipped/cancelled status, and artifact opening.
- [x] xterm.js terminal backed by native PTY sessions, separated from structured Lumi Test Output, with resize and process cleanup.
- [x] Optional Codex CLI and AGY local providers, binary detection/configuration, `@` context selection, and a side-by-side proposal preview. Proposed code is staged in the editor only after user review.
- [x] Lumi extension manager for installing/updating JSON manifests, enabling/disabling/removing, viewing contributions, copying snippets, and creating templates in the workspace.
- [x] Extension API/schema v1 documentation. Extension content is declarative and is not executed.
- [x] CSP is enabled and Tauri's filesystem and shell plugins are removed; app-owned filesystem operations go through Rust commands.
- [x] Lazy-loaded reports, Inspector, test explorer, extension manager, and AI panel. Production frontend build and Studio Rust tests pass.

## Remaining work

### Workbench and authoring

- [ ] Add a recent-workspace picker with multiple entries.
- [ ] Watch for external file changes and show compare/reload/keep-my-edits actions.
- [ ] Add editor quick fixes and template/snippet integration directly in YAML completion. Current extension panel copies snippets and creates template files from the workspace root.
- [ ] Improve keyboard/menu coverage and resizable pane behavior; verify primary flows in the real app.

### Inspector and execution

- [ ] Complete the Inspector-to-editor selector insertion flow and verify screenshots/hierarchy across supported targets.
- [ ] Make platform choices reflect the host's actual runtime dependencies and add a guided `doctor` setup flow.
- [ ] Validate saved/unsaved run behavior and nested flow accounting with representative real test runs.
- [ ] Add flaky/repeated failure analysis and platform/tag report breakdowns.

### Local AI and extensions

- [x] Bound local AI subprocesses to 120 seconds, cap captured output at 1 MiB, and kill the provider process group on Unix timeout.
- [ ] Add request cancellation in the AI UI and Windows process-tree cleanup.
- [ ] Add an extension catalog or repository source once the registry location and trust/publishing model are defined.
- [ ] Define runtime behavior for selector-pack and report-view contributions. They are browsable metadata in v1; they do not change engine behavior or add executable report views.
- [ ] Replace the side-by-side AI proposal preview with a syntax-aware in-editor diff and add automated extension lifecycle checks.

### Release and hardening

- [x] Build the packaged macOS app and smoke-check its native workbench, extension manager, and Codex/AGY detection.
- [x] Record one macOS arm64 package-size and idle-memory sample; a pre-optimization comparison baseline and repeatable cold-start measurement are still missing.
- [ ] Verify Windows/Linux builds and document platform-specific automation dependencies.
- [ ] Audit process lifecycle, host permissions, workspace change conflicts, and extension import behavior.
- [ ] Verify open/search/edit/save/reopen, YAML paths, completion/diagnostics, inspect, run/cancel, reports/artifacts, AI proposal review, terminal, and extension lifecycle in the real Tauri app.

## Verification evidence so far

- `yarn build` passes. Vite reports the initial JavaScript chunk at 720.62 KB (199.29 KB gzip); AI and report chunks load separately. The primary chunk still triggers the 500 KB advisory.
- `cargo test --manifest-path src-tauri/Cargo.toml` passes 12 tests, including workspace search line/column mapping, `.gitignore` and hidden-file handling, bounded AI output, workspace traversal/symlink protection, and extension manifest/version/path validation.
- `yarn tauri build` produced an arm64 `Lumi IDE.app` (46 MB) and DMG (17 MB). The app launched at `tauri://localhost`; the Extensions screen rendered, extension listing returned empty without error, and AI settings detected Codex and AGY. The bundle is ad-hoc signed and is not notarized.
- One idle RSS sample after about one minute was 33,504 KB. Cold-start timing and a baseline comparison are not yet measured.
- `yarn tauri dev` also compiles and launches the local process.

## Release acceptance

- Open, search, edit, save, close, and reopen a real workspace without losing changes; resolve external edits explicitly.
- YAML completion/docs/diagnostics come from the shared Lumi command source; path links and validation resolve the intended files.
- Inspector insertion and run/test counts, reports, cancellation, and artifacts agree for nested flows, failures, skips, and reruns.
- Interactive terminal is separate from test output and is cleaned up on close.
- Codex or AGY can use explicitly selected context and propose a path-specific change; the user sees a diff and approves before any file write.
- Extensions can install, update, enable, disable, and uninstall within declared declarative capabilities.
- Packaged startup, memory, and app size are measured and compared against a recorded baseline.
