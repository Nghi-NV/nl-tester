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
- `read_report`
- `read_events`
- `read_artifact`
- `inspector_get`
- `suggest_selectors`
