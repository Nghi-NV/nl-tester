# lumi-tester-mcp

MCP server for AI agents that write, run, and debug Lumi Tester YAML flows.

## Runtime Strategy

The server resolves the `lumi-tester` executable in this order:

1. `LUMI_TESTER_BIN=/absolute/path/to/lumi-tester`
2. Bundled binary in `binaries/<platform>-<arch>/lumi-tester`
3. Repo-local development checkout at `lumi-tester/Cargo.toml` via `cargo run --`
4. `lumi-tester` from `PATH`

For normal users, install the CLI first:

```bash
curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.sh | bash
```

Windows:

```powershell
iwr https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install.ps1 -UseB | iex
```

## Bundling Binary Into The MCP Package

Build or download the native `lumi-tester` binary, then stage it:

```bash
npm run stage-binary -- /path/to/lumi-tester
npm pack
```

This copies the binary into:

```text
binaries/<platform>-<arch>/lumi-tester
```

The package can then run on another machine with the same OS/CPU without a
separate `lumi-tester` install.

## Tools

- `validate_yaml`
- `list_tests`
- `doctor`
- `schema`
- `run_test`
- `run_command`
- `read_report`
- `read_events`
- `read_artifact`
- `inspector_get`
- `suggest_selectors`

Agent workflow:

1. Run `doctor` for the target platform. Supported platforms are `android`,
   `android_auto`, `ios`, `web`, `macos`, `windows`, and `all`.
2. For a one-off action (a tap, a screenshot, a pinch/zoom check, poking at a
   screen to see what's there) use `run_command` directly - it runs one or more
   commands against a real device/app session with no YAML file involved.
   Reach for a YAML file + `run_test` only once the flow is worth keeping as a
   named, repeatable test case.
3. Run `validate_yaml` and stop on invalid YAML.
4. Run `list_tests` before using a command index.
5. Use `schema` when command/header shape is unclear.
6. Use `run_test`; it enables report, snapshot, and `events.jsonl` by default.
7. On failure, read `run.json` with `read_report`, then inspect `events.jsonl`
   with `read_events`.
8. Use `read_artifact` for failure XML/log text. Use `suggest_selectors` for
   Android UIAutomator XML selector candidates before falling back to points.

`run_command` supports `android`, `ios`, `macos`, and `windows` (not `web` or
`android_auto` - the underlying `lumi-tester shell` path doesn't cover those;
use `run_test` for them instead). All commands passed in one `run_command` call
share a single device session/connection, not one per command.

`run_test` supports `android`, `android_auto`, `ios`, `web`, `macos`, and
`windows`. Native desktop tests must run on the local desktop host; macOS needs
Accessibility/Screen Recording permissions and Windows needs an interactive
desktop session.
