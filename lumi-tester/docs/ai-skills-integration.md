# AI Skills & MCP Integration Guide

Guide for integrating Lumi Tester skills into AI coding assistants (Google Antigravity, OpenAI Codex, Claude Code, Cursor, Windsurf).

## 1. Overview of AI Integrations

Lumi Tester provides first-class AI skills that teach AI assistants how to:
- Design test coverage from user stories or requirements.
- Write canonical YAML flows with stable selectors and regex shorthand (`tap: "name|tên"`).
- Validate, list runnable command indexes, and execute tests with diagnostic artifacts (`run.json`, `events.jsonl`, screenshots).
- Control hardware Jigs (`hw*` commands) and diagnose COM port connectivity.

## 2. Fast 1-Click Installation

### Built-in AI Installer (Recommended)
```bash
# If using Homebrew:
brew install nghi-nv/tap/lumi-tester
lumi-tester ai install

# macOS / Linux one-liner:
curl -fsSL https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install-ai.sh | bash

# Windows PowerShell:
iwr https://raw.githubusercontent.com/Nghi-NV/nl-tester/main/lumi-tester/scripts/install-ai.ps1 -UseB | iex
```

## 3. Google Antigravity Integration

`lumi-tester ai install` installs the skill for both roots Antigravity
discovers automatically - run it from the workspace you want the
workspace-level copy in:

```bash
lumi-tester ai install
# writes ./.agents/skills/lumi-tester-agent/ (this workspace only)
# writes ~/.gemini/config/skills/lumi-tester-agent/ (all workspaces)
```

To install manually instead:

### A. Workspace Level (Per-repository)
Place the skill inside the `.agents/skills/` directory at the root of your workspace:
```text
your-project/
├── .agents/
│   └── skills/
│       └── lumi-tester-agent/
│           ├── SKILL.md
│           ├── references/
│           └── scripts/
```

### B. Global Level (All workspaces)
Copy the skill to your global Antigravity configuration root:
```bash
mkdir -p ~/.gemini/config/skills
cp -r lumi-tester/ai/antigravity-skill/lumi-tester-agent ~/.gemini/config/skills/
```

## 4. OpenAI Codex Skill Integration

`lumi-tester ai install` also installs this. Codex discovers skills in
`~/.codex/skills/`; to install manually instead:
```bash
mkdir -p ~/.codex/skills
cp -r lumi-tester/ai/codex-skill/lumi-tester-agent ~/.codex/skills/
```

## 5. Claude Code Skill Integration

`lumi-tester ai install` also installs the skill for Claude Code, which
discovers skills in `~/.claude/skills/`:

```bash
lumi-tester ai install
# writes ~/.claude/skills/lumi-tester-agent/ (same bundle as the Codex skill)
```

To install manually without running `ai install`:

```bash
mkdir -p ~/.claude/skills
cp -r lumi-tester/ai/codex-skill/lumi-tester-agent ~/.claude/skills/
```

## 6. Lumi Tester MCP Server (`lumi-tester-mcp`)

Connect AI assistants to Lumi Tester via Model Context Protocol (MCP):

### Configuration snippet (`mcp_config.json` / Claude Desktop / Antigravity):
```json
{
  "mcpServers": {
    "lumi-tester": {
      "command": "node",
      "args": ["/path/to/lumi-tester-mcp/dist/index.js"],
      "env": {
        "PATH": "/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin"
      }
    }
  }
}
```

### Supported MCP Tools:
- `doctor`: Check platform runtime dependencies.
- `validate_yaml`: Validate syntax and parameters without launching a device.
- `list_tests`: Enumerate test flows, tags, and runnable command indexes.
- `schema`: Inspect the YAML command/header JSON shape.
- `run_test`: Execute full flow or target command index (`command_index`).
- `read_report` / `read_events` / `read_artifact`: Inspect structured JSON
  reports, event logs, and bounded text artifacts (failure XML/logs).
- `inspector_get`: Call a running Lumi Inspector REST endpoint (screenshot,
  hierarchy, element-at).
- `suggest_selectors`: Ranked, cross-platform selector suggestions for a
  query/point, from a running Inspector or a saved hierarchy dump.

## 7. Workspace Rules (`AGENTS.md` / `GEMINI.md`)

Include the [AGENTS.md](file:///AGENTS.md) guide in your project root to provide immediate context on:
- Canonical `header --- commands` YAML structure.
- Selector priority: `regex` & shorthand (`tap: "Save|Lưu"`) $\rightarrow$ explicit `id` $\rightarrow$ `exact: true text` $\rightarrow$ platform attributes.
- Self-test verification loop before claiming completion.

## 8. Verification & Health Checks

Verify your AI skill setup:
```bash
# Validate bundled schema
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-schema

# Run preflight check on a YAML flow
python3 ~/.codex/skills/lumi-tester-agent/scripts/lumi_agent.py agent-check path/to/test.yaml
```
