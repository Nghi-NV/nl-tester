# Codex Goal Prompt — Complete Lumi IDE

Continue the implementation in `lumi-tester-studio` and finish Lumi IDE as an independent, lightweight desktop IDE built with Tauri 2, Rust, React/TypeScript, and Monaco. Treat `docs/lumi-ide-roadmap.md` as the current status and source of outstanding work. Inspect repository `AGENTS.md` and relevant project instructions before changing code.

The intended experience is familiar to VS Code users while remaining a Lumi-branded product with no VS Code runtime or extension-host dependency. The installed app must not require Node.js or Rust/Cargo at runtime.

## Work to complete

1. Finish the workbench: recent workspace picker, lazy Explorer, `.gitignore` and dotfile controls, workspace search, safe create/rename/move/delete, dirty/save/save-all/discard behavior, external file-change conflict handling, menus, shortcuts, and resizable panels.
2. Finish Lumi YAML authoring: use the canonical shared command/selector metadata, Monaco completion/snippets/hover/diagnostics/quick fixes, path links for flows and environment files, and reliable validation of the content the user intends to run.
3. Finish test workflows: useful Test Explorer filters, run file/folder/command, rerun failures, host-aware platform and device choices, guided `doctor`, Inspector screenshots/hierarchy/selector suggestions, and insertion of selected selectors into the active YAML flow.
4. Make run state trustworthy: structured events and engine final summaries are authoritative; count nested flows once; support cancellation, unique artifacts, saved/unsaved semantics, history, skipped/cancelled states, flaky/repeated failures, platform/tag breakdowns, and working artifact links.
5. Complete terminal and local AI: keep interactive PTY separate from Lumi Test Output; reliably clean up child processes. Support optional local Codex CLI and AGY with detection/path settings, explicit context selection, bounded execution, cancellation, and a syntax-aware proposal diff. AI must not write files directly; require user review before applying changes.
6. Complete the Lumi extension manager: manifest validation and API compatibility, install/update/enable/disable/remove, usable docs/snippets/templates/selector packs/report views, and a trusted catalog or repository source only if an authoritative registry can be established. Do not execute arbitrary extension code.
7. Harden and release: audit workspace paths, symlinks, subprocess handling, permissions, CSP, cross-platform behavior, and shutdown. Lazy-load heavy features and measure packaged size, cold-start, and idle memory against recorded baselines.

## Constraints

- Preserve unrelated working-tree changes, especially existing E2E YAML edits and untracked user data. Inspect `git status` before edits and before handoff; never revert work you did not create.
- Keep CLI and VS Code extension workflows working. Avoid unrelated refactors and speculative features.
- Route workspace file access through the Rust workspace boundary. Reject traversal and paths/symlinks outside the selected workspace.
- Avoid network AI defaults. Codex and AGY run locally; Gemini remains explicitly opt-in with a user-provided API key.
- Do not commit or publish unless explicitly asked.
- Update `docs/lumi-ide-roadmap.md` with evidence, not optimistic checkmarks.

## Completion criteria

- Run the relevant frontend build, Studio Rust tests, and repository pre-commit gate before handoff.
- Build and open the actual Tauri app; verify the main flows in its native window, not only a browser preview.
- Demonstrate workspace open/search/edit/save/reopen, external-edit conflict, YAML diagnostics/path navigation, Inspector-to-flow insertion, run/cancel/report/artifacts, PTY input/output/resize/cleanup, AI proposal review/apply, and extension install/update/enable/disable/remove.
- Produce reproducible app-size/startup/memory measurements and document host-specific prerequisites.
- Report any unmet criterion as incomplete with the reason and evidence. Do not mark the goal complete while required work remains.
