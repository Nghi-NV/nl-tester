#!/usr/bin/env python3
"""Validate AI reference CSV files against parser command names."""

from __future__ import annotations

import contextlib
import csv
import importlib.util
import io
import json
import re
import sys
from pathlib import Path, PurePosixPath


ROOT = Path(__file__).resolve().parents[2]
YAML_RS = ROOT / "lumi-tester" / "src" / "parser" / "yaml.rs"
COMMANDS_CSV = (
    ROOT
    / "lumi-tester"
    / "ai"
    / "codex-skill"
    / "lumi-tester-agent"
    / "references"
    / "commands.csv"
)
SKILL_DIR = ROOT / "lumi-tester" / "ai" / "codex-skill" / "lumi-tester-agent"
SKILL_MD = SKILL_DIR / "SKILL.md"
SELECTORS_CSV = (
    ROOT
    / "lumi-tester"
    / "ai"
    / "codex-skill"
    / "lumi-tester-agent"
    / "references"
    / "selectors.csv"
)
CLI_CSV = (
    ROOT
    / "lumi-tester"
    / "ai"
    / "codex-skill"
    / "lumi-tester-agent"
    / "references"
    / "cli.csv"
)
OPENAI_YAML = SKILL_DIR / "agents" / "openai.yaml"
TESTCASE_DESIGN_MD = SKILL_DIR / "references" / "testcase-design.md"
DEBUG_ARTIFACTS_MD = SKILL_DIR / "references" / "debug-artifacts.md"
SCHEMA_JSON = ROOT / "lumi-tester" / "schema" / "lumi-test.schema.json"
HELPER_SCRIPT = SKILL_DIR / "scripts" / "lumi_agent.py"


def parser_commands() -> set[str]:
    commands: set[str] = set()
    arm_re = re.compile(r"^\s*((?:\"[^\"]+\"\s*(?:\|\s*)?)+)\s*=>")
    for line in YAML_RS.read_text(encoding="utf-8").splitlines():
        match = arm_re.match(line)
        if not match:
            continue
        commands.update(re.findall(r'"([^"]+)"', match.group(1)))
    return commands


def cli_commands() -> set[str]:
    text = (ROOT / "lumi-tester" / "src" / "main.rs").read_text(encoding="utf-8")
    match = re.search(r"enum Commands\s*\{(.*?)\n\}", text, flags=re.DOTALL)
    if not match:
        return set()
    variants: set[str] = set()
    for line in match.group(1).splitlines():
        match_line = re.match(r"^\s*([A-Z][A-Za-z0-9]*)\s*(?:\{|,)", line)
        if match_line:
            name = match_line.group(1)
            variants.add(re.sub(r"(?<!^)([A-Z])", r"-\1", name).lower())
    return variants


def csv_command_names() -> set[str]:
    names: set[str] = set()
    with COMMANDS_CSV.open(newline="", encoding="utf-8") as fh:
        for row in csv.DictReader(fh):
            names.add(row["command"].strip())
            aliases = row.get("aliases", "")
            for alias in re.split(r"[|,]", aliases):
                alias = alias.strip()
                if alias:
                    names.add(alias)
    return names


def command_catalog_rows() -> dict[str, dict[str, str]]:
    rows: dict[str, dict[str, str]] = {}
    with COMMANDS_CSV.open(newline="", encoding="utf-8") as fh:
        for row in csv.DictReader(fh):
            names = [row["command"].strip()]
            names.extend(
                alias.strip()
                for alias in re.split(r"[|,]", row.get("aliases", ""))
                if alias.strip()
            )
            for name in names:
                rows[name] = row
    return rows


def split_field_names(value: str) -> set[str]:
    return {item.strip() for item in re.split(r"[|,]", value) if item.strip()}


def validate_csv(path: Path, required_columns: set[str]) -> list[str]:
    errors: list[str] = []
    with path.open(newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        missing = required_columns.difference(reader.fieldnames or [])
        if missing:
            errors.append(f"{path}: missing columns: {', '.join(sorted(missing))}")
        rows = list(reader)
    if not rows:
        errors.append(f"{path}: no data rows")
    return errors


def schema_command_names() -> set[str]:
    schema = json.loads(SCHEMA_JSON.read_text(encoding="utf-8"))
    return set(schema["$defs"]["commandName"]["enum"])


def schema_selector_names() -> set[str]:
    schema = json.loads(SCHEMA_JSON.read_text(encoding="utf-8"))
    return set(schema["$defs"]["selector"]["properties"])


def schema_selector_example_names() -> set[str]:
    schema = json.loads(SCHEMA_JSON.read_text(encoding="utf-8"))
    names = set(schema["$defs"]["selector"]["properties"])
    scrollable = schema["$defs"]["selector"]["properties"]["scrollable"]["oneOf"][1]
    names.update(scrollable["properties"])
    ocr_selector = schema["$defs"]["ocrSelector"]["oneOf"][1]
    names.update(ocr_selector["properties"])
    return names


def validate_skill_references() -> list[str]:
    errors: list[str] = []
    text = SKILL_MD.read_text(encoding="utf-8")
    refs = sorted(set(re.findall(r"references/[A-Za-z0-9_.-]+", text)))
    for ref in refs:
        path = SKILL_DIR / ref
        if not path.exists():
            errors.append(f"{SKILL_MD}: referenced file does not exist: {ref}")
    linked = set(refs)
    for path in sorted((SKILL_DIR / "references").glob("*")):
        if path.is_file() and f"references/{path.name}" not in linked:
            errors.append(f"{path}: reference file is not linked from {SKILL_MD}")
    return errors


def normalize_heading(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", " ", value.lower()).strip()


def validate_reference_navigation() -> list[str]:
    errors: list[str] = []
    for path in sorted((SKILL_DIR / "references").glob("*.md")):
        lines = path.read_text(encoding="utf-8").splitlines()
        if len(lines) <= 100:
            continue
        text = "\n".join(lines[:40])
        if "## Contents" not in text:
            errors.append(f"{path}: long reference should have ## Contents near the top")
        contents_index = next(
            (idx for idx, line in enumerate(lines) if line.strip() == "## Contents"),
            None,
        )
        if contents_index is None:
            continue
        bullets = [
            line[2:].strip()
            for line in lines[contents_index + 1 : contents_index + 15]
            if line.startswith("- ")
        ]
        if len(bullets) < 3:
            errors.append(f"{path}: Contents should include at least 3 bullets")
            continue
        headings = {
            normalize_heading(line.removeprefix("##").strip())
            for line in lines
            if line.startswith("## ") and line.strip() != "## Contents"
        }
        for bullet in bullets:
            label = re.sub(r"^\[([^\]]+)\]\([^)]+\)$", r"\1", bullet)
            normalized = normalize_heading(label)
            if normalized not in headings:
                errors.append(
                    f"{path}: Contents bullet does not match a ## heading: {bullet}"
                )
    return errors


def validate_agents_metadata() -> list[str]:
    errors: list[str] = []
    if not OPENAI_YAML.exists():
        return [f"{OPENAI_YAML}: file does not exist"]

    text = OPENAI_YAML.read_text(encoding="utf-8")
    required = {
        "display_name": r"^\s*display_name\s*:",
        "short_description": r"^\s*short_description\s*:",
        "default_prompt": r"^\s*default_prompt\s*:",
    }
    for field, pattern in required.items():
        if not re.search(pattern, text, flags=re.MULTILINE):
            errors.append(f"{OPENAI_YAML}: missing interface field: {field}")

    if "$lumi-tester-agent" not in text:
        errors.append(f"{OPENAI_YAML}: default_prompt should mention $lumi-tester-agent")
    for term in ("design", "run", "debug"):
        if term not in text.lower():
            errors.append(f"{OPENAI_YAML}: metadata should mention {term}")
    return errors


def validate_helper_script_reference() -> list[str]:
    errors: list[str] = []
    if not HELPER_SCRIPT.exists():
        return [f"{HELPER_SCRIPT}: file does not exist"]

    helper_text = HELPER_SCRIPT.read_text(encoding="utf-8")
    skill_text = SKILL_MD.read_text(encoding="utf-8")
    required_agent_commands = {
        "agent-validate": {"validate", "--json"},
        "agent-list": {"list", "--json"},
        "agent-doctor": {"doctor", "--platform", "--json"},
        "agent-run": {"run", "--platform", "--report", "--snapshot", "--events-jsonl", "--output"},
        "agent-debug": {
            "run",
            "--platform",
            "--command-index",
            "--report",
            "--snapshot",
            "--events-jsonl",
            "--output",
        },
    }
    for command, terms in required_agent_commands.items():
        if command not in helper_text:
            errors.append(f"{HELPER_SCRIPT}: missing helper command: {command}")
        if command not in skill_text:
            errors.append(f"{SKILL_MD}: missing helper usage for: {command}")
        for term in terms:
            if term not in helper_text:
                errors.append(f"{HELPER_SCRIPT}: {command} should include {term}")

    for command in ("validate", "list", "doctor", "run", "schema", "devices"):
        if f'"{command}"' not in helper_text:
            errors.append(f"{HELPER_SCRIPT}: missing raw Lumi passthrough command: {command}")

    if "repo-local `cargo run`" not in skill_text:
        errors.append(f"{SKILL_MD}: helper docs should explain repo-local cargo fallback")
    if "falls back to an installed" not in skill_text:
        errors.append(f"{SKILL_MD}: helper docs should explain installed binary fallback")
    return errors


def validate_helper_script_behavior() -> list[str]:
    errors: list[str] = []
    spec = importlib.util.spec_from_file_location("lumi_agent_reference", HELPER_SCRIPT)
    if spec is None or spec.loader is None:
        return [f"{HELPER_SCRIPT}: could not load helper module"]
    helper = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(helper)

    agent_run = helper.parse_agent_run(
        [
            "tests/generated/login",
            "--platform",
            "android",
            "--device",
            "emulator-5554",
            "--output",
            "./output/login",
            "--command-index",
            "4",
            "--debug",
        ]
    )
    expected_agent_run = [
        "tests/generated/login",
        "--platform",
        "android",
        "--report",
        "--snapshot",
        "--events-jsonl",
        "--output",
        "./output/login",
        "--device",
        "emulator-5554",
        "--command-index",
        "4",
        "--debug",
    ]
    if agent_run != expected_agent_run:
        errors.append(
            f"{HELPER_SCRIPT}: agent-run parse output changed: {agent_run!r}"
        )

    agent_debug = helper.parse_agent_run(
        ["tests/smoke.yaml", "--platform", "ios", "--command-index", "2"],
        require_command_index=True,
    )
    if "--command-index" not in agent_debug or "2" not in agent_debug:
        errors.append(f"{HELPER_SCRIPT}: agent-debug should require and forward command index")
    for required in ("--report", "--snapshot", "--events-jsonl"):
        if required not in agent_debug:
            errors.append(f"{HELPER_SCRIPT}: agent-debug should include {required}")

    with contextlib.redirect_stderr(io.StringIO()):
        try:
            helper.parse_agent_run(
                ["tests/smoke.yaml", "--platform", "ios"],
                require_command_index=True,
            )
        except SystemExit:
            pass
        else:
            errors.append(f"{HELPER_SCRIPT}: agent-debug should fail without --command-index")

    if helper.parse_agent_path_json(["tests/smoke.yaml"], "validate") != [
        "tests/smoke.yaml",
        "--json",
    ]:
        errors.append(f"{HELPER_SCRIPT}: agent-validate should append --json")
    if helper.parse_agent_doctor(["--platform", "web"]) != ["--platform", "web", "--json"]:
        errors.append(f"{HELPER_SCRIPT}: agent-doctor should append --json")
    if helper.parse_passthrough(["run", "tests/smoke.yaml", "--platform", "web"]) != (
        "run",
        ["tests/smoke.yaml", "--platform", "web"],
    ):
        errors.append(f"{HELPER_SCRIPT}: raw run passthrough should preserve extra args")
    return errors


def markdown_section(text: str, heading: str) -> str:
    pattern = rf"^## {re.escape(heading)}\s*$"
    match = re.search(pattern, text, flags=re.MULTILINE)
    if not match:
        return ""
    next_heading = re.search(r"^##\s+", text[match.end() :], flags=re.MULTILINE)
    if next_heading:
        return text[match.end() : match.end() + next_heading.start()]
    return text[match.end() :]


def csv_header_line(text: str) -> str:
    match = re.search(r"```csv\n(.*?)```", text, flags=re.DOTALL)
    if not match:
        return ""
    return match.group(1).strip().splitlines()[0].strip().lower()


def normalize_posix_path(path: str) -> str:
    parts: list[str] = []
    for part in PurePosixPath(path).parts:
        if part in {"", "."}:
            continue
        if part == "..":
            if parts:
                parts.pop()
            continue
        parts.append(part)
    return "/".join(parts)


def labeled_yaml_examples(section: str) -> list[tuple[str, str]]:
    examples: list[tuple[str, str]] = []
    pattern = re.compile(r"`([^`]+\.ya?ml)`:\s*\n\s*```ya?ml\n(.*?)```", re.DOTALL)
    for match in pattern.finditer(section):
        examples.append((match.group(1), match.group(2)))
    return examples


def run_flow_paths(body: str) -> list[str]:
    paths: list[str] = []
    for line in body.splitlines():
        match = re.match(r"^\s*-\s+runFlow:\s*[\"']?([^\"'\s]+)[\"']?\s*$", line)
        if match:
            paths.append(match.group(1))
    return paths


def validate_testcase_design_reference() -> list[str]:
    errors: list[str] = []
    raw_text = TESTCASE_DESIGN_MD.read_text(encoding="utf-8")
    text = raw_text.lower()
    required_terms = {
        "equivalence partitioning": "equivalence partitioning",
        "boundary value": "boundary value analysis",
        "decision tables": "decision tables",
        "state transition": "state transition testing",
        "pairwise": "pairwise/combinatorial testing",
    }
    for term, label in required_terms.items():
        if term not in text:
            errors.append(f"{TESTCASE_DESIGN_MD}: missing {label}")

    section_requirements = {
        "Research Inputs": {
            "product requirements": "product artifact research",
            "runtime exploration": "runtime exploration",
            "ui xml": "UI hierarchy research",
            "dom": "DOM research",
            "platform contracts": "platform contract research",
        },
        "Coverage Model": {
            "actors/roles": "actors/roles model",
            "entry points": "entry points model",
            "states": "states model",
            "objects/data": "objects/data model",
            "operations": "operations model",
            "oracles": "oracles model",
        },
        "App And Web Coverage Checklist": {
            "web-specific": "web-specific coverage",
            "mobile-specific": "mobile-specific coverage",
            "permissions and privacy": "permissions coverage",
            "state and data": "state/data coverage",
            "security-focused web/api smoke": "security coverage",
        },
        "Stop Conditions": {
            "every requirement": "requirement traceability stop condition",
            "high-risk": "high-risk stop condition",
            "boundary": "boundary stop condition",
            "permission": "permission stop condition",
            "reports/artifacts": "debug artifact stop condition",
        },
        "Generated Suite Example": {
            "setup.yaml": "root setup example",
            "subflows/login.yaml": "login subflow example",
            "001_toggle_notifications.yaml": "leaf regression example",
            "lumi-tester validate tests/generated": "folder validation command",
            "lumi-tester run tests/generated": "folder run command",
            "not the leaf file": "folder run warning",
        },
    }
    for heading, terms in section_requirements.items():
        section = markdown_section(raw_text, heading).lower()
        if not section:
            errors.append(f"{TESTCASE_DESIGN_MD}: missing section: {heading}")
            continue
        for term, label in terms.items():
            if term not in section:
                errors.append(f"{TESTCASE_DESIGN_MD}: missing {label}")

    header = csv_header_line(raw_text)
    for column in ("source", "entry_point"):
        if column not in {part.strip() for part in header.split(",")}:
            errors.append(f"{TESTCASE_DESIGN_MD}: cases.csv is missing {column} column")

    suite_section = markdown_section(raw_text, "Generated Suite Example")
    suite_examples = labeled_yaml_examples(suite_section)
    declared_paths = {normalize_posix_path(path) for path, _ in suite_examples}
    if len(suite_examples) < 3:
        errors.append(f"{TESTCASE_DESIGN_MD}: generated suite should include at least 3 YAML files")
    for file_path, body in suite_examples:
        base_dir = PurePosixPath(file_path).parent
        for run_flow in run_flow_paths(body):
            resolved = normalize_posix_path(str(base_dir / run_flow))
            if resolved not in declared_paths:
                errors.append(
                    f"{TESTCASE_DESIGN_MD}: {file_path} runFlow target is not "
                    f"declared in generated suite example: {run_flow}"
                )
    return errors


def validate_debug_artifacts_reference() -> list[str]:
    errors: list[str] = []
    raw_text = DEBUG_ARTIFACTS_MD.read_text(encoding="utf-8")
    text = raw_text.lower()
    skill_text = SKILL_MD.read_text(encoding="utf-8").lower()
    debug_loop = markdown_section(SKILL_MD.read_text(encoding="utf-8"), "Debugging Loop").lower()
    if "references/debug-artifacts.md" not in skill_text:
        errors.append(f"{SKILL_MD}: missing debug artifacts reference")
    if "references/debug-artifacts.md" not in debug_loop:
        errors.append(f"{SKILL_MD}: debug loop should point to debug-artifacts.md")
    for term in ("wrong app", "crash", "permission", "platform-specific"):
        if term not in debug_loop:
            errors.append(f"{SKILL_MD}: debug loop should mention {term} failures")

    required_terms = {
        "run.json": "run summary artifact",
        "test-results.json": "report artifact",
        "events.jsonl": "event stream artifact",
        "commandfailed.index": "failed command rerun guidance",
        "wrong app": "wrong target diagnosis",
        "element not found": "selector failure diagnosis",
        "assertion timeout": "assertion timeout diagnosis",
        "runtime dependency failure": "runtime dependency diagnosis",
        "app launch/crash/abort": "launch/crash diagnosis",
    }
    for term, label in required_terms.items():
        if term not in text:
            errors.append(f"{DEBUG_ARTIFACTS_MD}: missing {label}")

    section = markdown_section(raw_text, "Common Failure Diagnosis").lower()
    if not section:
        errors.append(f"{DEBUG_ARTIFACTS_MD}: missing section: Common Failure Diagnosis")
        return errors

    platform_patterns = {
        "android": {
            "focus": r"mcurrentfocus|mfocusedapp|topresumed",
            "process": r"\bpidof\b|process",
            "logs": r"\blogcat\b",
        },
        "ios": {
            "bundle": r"simctl listapps|bundle id",
            "hierarchy": r"accessibility (?:hierarchy|tree)",
            "permission": r"permission (?:alert|dialog|state)",
        },
        "web": {
            "url": r"(?:actual|resolved|browser|page) (?:page )?url|\burl\b",
            "console": r"console (?:errors?|logs?)|pageerror",
            "network": r"(?:failed )?network requests?|http failures?|4\[0-9\]\{2\}|5\[0-9\]\{2\}",
        },
    }
    for platform, patterns in platform_patterns.items():
        for label, pattern in patterns.items():
            if not re.search(pattern, section):
                errors.append(f"{DEBUG_ARTIFACTS_MD}: missing {platform} debug signal: {label}")

    classifications = (
        "wrong target",
        "setup/state issue",
        "app/runtime issue",
        "selector issue",
    )
    for label in classifications:
        if label not in section:
            errors.append(f"{DEBUG_ARTIFACTS_MD}: missing failure classification: {label}")
    return errors


def validate_reference_examples() -> list[str]:
    stale_patterns = {
        "killApp command": r"\bkillApp\b",
        "runScript.file field": r"runScript:\s*\n(?:[ \t]+[A-Za-z0-9_]+:.*\n)*[ \t]+file:",
        "runScript.env field": r"runScript:\s*\n(?:[ \t]+[A-Za-z0-9_]+:.*\n)*[ \t]+env:",
        "runScript inline vars JS": r"runScript:\s*[\"']?vars\.",
        "screenshot.name field": r"screenshot:\s*\n(?:[ \t]+[A-Za-z0-9_]+:.*\n)*[ \t]+name:",
        "mockLocation latitude field": r"mockLocation:\s*\n(?:[ \t]+[A-Za-z0-9_]+:.*\n)*[ \t]+latitude:",
        "mockLocation longitude field": r"mockLocation:\s*\n(?:[ \t]+[A-Za-z0-9_]+:.*\n)*[ \t]+longitude:",
        "conditional.when field": r"conditional:\s*\n(?:[ \t]+[A-Za-z0-9_]+:.*\n)*[ \t]+when:",
        "conditional.commands field": r"conditional:\s*\n(?:[ \t]+[A-Za-z0-9_]+:.*\n)*[ \t]+commands:",
        "relative.anchor schema": r"relative:\s*\n[ \t]+anchor:",
        "relative.direction schema": r"relative:\s*\n(?:[ \t]+[A-Za-z0-9_]+:.*\n)*[ \t]+direction:",
        "relative.target schema": r"relative:\s*\n(?:[ \t]+[A-Za-z0-9_]+:.*\n)*[ \t]+target:",
        "setup hook per-file wording": r"setup\.ya?ml[^.\n]*(?:before each file|per file)",
        "teardown hook per-file wording": r"teardown\.ya?ml[^.\n]*(?:after each file|per file)",
    }
    errors: list[str] = []
    paths = [SKILL_MD, *sorted((SKILL_DIR / "references").glob("*"))]
    for path in paths:
        if path.suffix not in {".md", ".csv"}:
            continue
        text = path.read_text(encoding="utf-8")
        for label, pattern in stale_patterns.items():
            if re.search(pattern, text):
                errors.append(f"{path}: stale AI reference example: {label}")
    return errors


def yaml_fence_bodies(text: str) -> list[str]:
    return re.findall(r"```ya?ml\n(.*?)```", text, flags=re.DOTALL)


def command_names_in_yaml_example(body: str) -> set[str]:
    lines = body.splitlines()
    if "---" in [line.strip() for line in lines]:
        separator_index = next(i for i, line in enumerate(lines) if line.strip() == "---")
        lines = lines[separator_index + 1 :]
    elif not any(line.lstrip().startswith("- ") for line in lines):
        return set()

    names: set[str] = set()
    for line in lines:
        match = re.match(r"^\s*-\s+([A-Za-z][A-Za-z0-9_]*)\b", line)
        if match:
            names.add(match.group(1))
    return names


def validate_yaml_command_examples(parser_names: set[str]) -> list[str]:
    errors: list[str] = []
    paths = [SKILL_MD, *sorted((SKILL_DIR / "references").glob("*.md"))]
    for path in paths:
        text = path.read_text(encoding="utf-8")
        for idx, body in enumerate(yaml_fence_bodies(text), start=1):
            unknown = sorted(command_names_in_yaml_example(body).difference(parser_names))
            if unknown:
                errors.append(
                    f"{path}: YAML example block {idx} uses unknown command(s): "
                    + ", ".join(unknown)
                )

    for path in (COMMANDS_CSV, CLI_CSV):
        with path.open(newline="", encoding="utf-8") as fh:
            for row in csv.DictReader(fh):
                example = row.get("example", "")
                unknown = sorted(command_names_in_yaml_example(example).difference(parser_names))
                if unknown:
                    errors.append(
                        f"{path}: example for {row.get('command', '<unknown>')} "
                        f"uses unknown command(s): {', '.join(unknown)}"
                    )
    return errors


def selector_example_keys(example: str) -> set[str]:
    keys: set[str] = set()
    for match in re.finditer(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*:", example):
        prefix = example[: match.start()].rstrip()
        if not prefix or prefix[-1] in "{,;":
            keys.add(match.group(1))
    return keys


def top_level_map_keys(map_text: str) -> set[str]:
    keys: set[str] = set()
    depth = 0
    in_quote: str | None = None
    escape = False
    token_start = 0
    segments: list[str] = []
    for idx, char in enumerate(map_text):
        if in_quote:
            if escape:
                escape = False
            elif char == "\\":
                escape = True
            elif char == in_quote:
                in_quote = None
            continue
        if char in {"'", '"'}:
            in_quote = char
        elif char in "{[":
            depth += 1
        elif char in "}]":
            depth -= 1
        elif char == "," and depth == 0:
            segments.append(map_text[token_start:idx])
            token_start = idx + 1
    segments.append(map_text[token_start:])

    for segment in segments:
        match = re.match(r"\s*([A-Za-z_][A-Za-z0-9_]*)\s*:", segment)
        if match:
            keys.add(match.group(1))
    return keys


def command_example_param_keys(example: str) -> set[str]:
    match = re.search(r"-\s+[A-Za-z][A-Za-z0-9_]*\s*:\s*\{(.*)\}\s*$", example)
    if match:
        return top_level_map_keys(match.group(1))
    return set()


def command_entries_in_yaml_example(body: str) -> list[tuple[str, set[str]]]:
    lines = body.splitlines()
    if "---" in [line.strip() for line in lines]:
        separator_index = next(i for i, line in enumerate(lines) if line.strip() == "---")
        lines = lines[separator_index + 1 :]
    elif not any(line.lstrip().startswith("- ") for line in lines):
        return []

    entries: list[tuple[str, set[str]]] = []
    idx = 0
    while idx < len(lines):
        line = lines[idx]
        inline = re.match(r"^(\s*)-\s+([A-Za-z][A-Za-z0-9_]*)\s*:\s*\{(.*)\}\s*$", line)
        if inline:
            entries.append((inline.group(2), top_level_map_keys(inline.group(3))))
            idx += 1
            continue

        scalar = re.match(r"^(\s*)-\s+([A-Za-z][A-Za-z0-9_]*)\s*:\s+\S.*$", line)
        if scalar:
            entries.append((scalar.group(2), set()))
            idx += 1
            continue

        block = re.match(r"^(\s*)-\s+([A-Za-z][A-Za-z0-9_]*)\s*:\s*$", line)
        if not block:
            idx += 1
            continue

        base_indent = len(block.group(1))
        child_lines: list[str] = []
        idx += 1
        while idx < len(lines):
            next_line = lines[idx]
            next_indent = len(next_line) - len(next_line.lstrip(" "))
            if next_line.lstrip().startswith("- ") and next_indent <= base_indent:
                break
            child_lines.append(next_line)
            idx += 1

        key_matches: list[tuple[int, str]] = []
        for child in child_lines:
            match = re.match(r"^(\s+)([A-Za-z_][A-Za-z0-9_]*)\s*:", child)
            if match:
                key_matches.append((len(match.group(1)), match.group(2)))
        if not key_matches:
            entries.append((block.group(2), set()))
            continue
        top_indent = min(indent for indent, _ in key_matches)
        entries.append(
            (block.group(2), {key for indent, key in key_matches if indent == top_indent})
        )
    return entries


def allowed_command_fields(row: dict[str, str], selector_names: set[str]) -> set[str]:
    allowed = split_field_names(row["required_fields"]).union(
        split_field_names(row["common_fields"])
    )
    if row["selector_supported"].strip().lower() in {"yes", "partial"}:
        allowed.update(selector_names)
    return allowed


def validate_command_example_fields() -> list[str]:
    errors: list[str] = []
    selector_names = schema_selector_names()
    rows = command_catalog_rows()
    with COMMANDS_CSV.open(newline="", encoding="utf-8") as fh:
        for row in csv.DictReader(fh):
            example_keys = command_example_param_keys(row["example"])
            if not example_keys:
                continue
            allowed = allowed_command_fields(row, selector_names)
            unknown = sorted(example_keys.difference(allowed))
            if unknown:
                errors.append(
                    f"{COMMANDS_CSV}: example for {row['command']} has fields "
                    "not declared in required/common fields: " + ", ".join(unknown)
                )

    for path in [SKILL_MD, *sorted((SKILL_DIR / "references").glob("*.md"))]:
        text = path.read_text(encoding="utf-8")
        for idx, body in enumerate(yaml_fence_bodies(text), start=1):
            for command, example_keys in command_entries_in_yaml_example(body):
                if command not in rows or not example_keys:
                    continue
                allowed = allowed_command_fields(rows[command], selector_names)
                unknown = sorted(example_keys.difference(allowed))
                if unknown:
                    errors.append(
                        f"{path}: YAML example block {idx} command {command} has "
                        "fields not declared in commands.csv/schema: "
                        + ", ".join(unknown)
                    )
    return errors


def validate_selector_catalog() -> list[str]:
    errors: list[str] = []
    schema_names = schema_selector_names()
    example_names = schema_selector_example_names()
    with SELECTORS_CSV.open(newline="", encoding="utf-8") as fh:
        for row in csv.DictReader(fh):
            names = [row["selector"].strip()]
            aliases = row.get("aliases", "")
            names.extend(alias.strip() for alias in re.split(r"[|,]", aliases) if alias.strip())
            unknown = sorted(set(names).difference(schema_names))
            if unknown:
                errors.append(
                    f"{SELECTORS_CSV}: selector row {row['selector']} has names "
                    "not present in schema selector properties: " + ", ".join(unknown)
                )
            example_keys = selector_example_keys(row["example"])
            unknown_example_keys = sorted(example_keys.difference(example_names))
            if unknown_example_keys:
                errors.append(
                    f"{SELECTORS_CSV}: selector row {row['selector']} example has "
                    "fields not present in schema selector properties: "
                    + ", ".join(unknown_example_keys)
                )
    return errors


def validate_selector_quality() -> list[str]:
    errors: list[str] = []
    rows: dict[str, dict[str, str]] = {}
    with SELECTORS_CSV.open(newline="", encoding="utf-8") as fh:
        for row in csv.DictReader(fh):
            selector = row["selector"].strip()
            rows[selector] = row
            for column in ("when_to_use", "example", "anti_patterns"):
                if not row[column].strip():
                    errors.append(f"{SELECTORS_CSV}: selector {selector} has empty {column}")
            try:
                rank = int(row["stability_rank"])
            except ValueError:
                errors.append(f"{SELECTORS_CSV}: selector {selector} has non-numeric rank")
                continue
            if rank < 0 or rank > 9:
                errors.append(f"{SELECTORS_CSV}: selector {selector} rank must be 0..9")

    required = {"id", "accessibilityId", "text", "regex", "relative", "ocr", "image", "point"}
    missing = sorted(required.difference(rows))
    if missing:
        errors.append(f"{SELECTORS_CSV}: missing selector priority rows: " + ", ".join(missing))
        return errors

    ranks = {name: int(row["stability_rank"]) for name, row in rows.items()}
    if ranks["point"] <= ranks["ocr"]:
        errors.append(f"{SELECTORS_CSV}: point must rank below OCR fallback")
    if ranks["point"] <= ranks["image"]:
        errors.append(f"{SELECTORS_CSV}: point must rank below image fallback")
    if ranks["relative"] >= ranks["point"]:
        errors.append(f"{SELECTORS_CSV}: relative selectors must rank above point")
    for semantic in ("id", "accessibilityId"):
        if ranks[semantic] > 2:
            errors.append(f"{SELECTORS_CSV}: {semantic} should be a high-stability selector")
    if "coordinate" not in rows["point"]["anti_patterns"].lower():
        errors.append(f"{SELECTORS_CSV}: point anti-pattern should warn about coordinates")

    aliases = rows["accessibilityId"].get("aliases", "")
    for alias in ("desc", "contentDesc"):
        if alias not in aliases:
            errors.append(f"{SELECTORS_CSV}: accessibilityId aliases should include {alias}")

    discovery = (SKILL_DIR / "references" / "selector-discovery.md").read_text(
        encoding="utf-8"
    ).lower()
    for phrase in (
        "coordinates are allowed only",
        "do not immediately replace it with `point`",
        "do not start with `point`",
    ):
        if phrase not in discovery:
            errors.append(f"selector-discovery.md: missing coordinate guard: {phrase}")
    return errors


def validate_cli_catalog() -> list[str]:
    errors: list[str] = []
    source_names = cli_commands()
    with CLI_CSV.open(newline="", encoding="utf-8") as fh:
        csv_names = {row["command"].strip() for row in csv.DictReader(fh)}
    missing = sorted(source_names.difference(csv_names))
    if missing:
        errors.append("cli.csv is missing top-level CLI commands: " + ", ".join(missing))
    extra = sorted(csv_names.difference(source_names))
    if extra:
        errors.append("cli.csv contains commands not present in CLI: " + ", ".join(extra))
    return errors


def csv_cli_names() -> set[str]:
    with CLI_CSV.open(newline="", encoding="utf-8") as fh:
        return {row["command"].strip() for row in csv.DictReader(fh)}


def shell_fence_bodies(text: str) -> list[str]:
    return re.findall(r"```(?:bash|sh)\n(.*?)```", text, flags=re.DOTALL)


def cli_commands_in_shell_example(body: str) -> set[str]:
    names: set[str] = set()
    for line in body.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        match = re.match(r"lumi-tester\s+([A-Za-z][A-Za-z0-9-]*|<command>)\b", stripped)
        if not match:
            match = re.match(
                r"cargo\s+run\s+--\s+([A-Za-z][A-Za-z0-9-]*|<command>)\b",
                stripped,
            )
        if not match:
            continue
        command = match.group(1)
        if command != "<command>":
            names.add(command)
    return names


def validate_shell_cli_examples(cli_names: set[str]) -> list[str]:
    errors: list[str] = []
    paths = [SKILL_MD, *sorted((SKILL_DIR / "references").glob("*.md"))]
    for path in paths:
        text = path.read_text(encoding="utf-8")
        for idx, body in enumerate(shell_fence_bodies(text), start=1):
            unknown = sorted(cli_commands_in_shell_example(body).difference(cli_names))
            if unknown:
                errors.append(
                    f"{path}: shell example block {idx} uses unknown CLI command(s): "
                    + ", ".join(unknown)
                )
    return errors


def main() -> int:
    errors: list[str] = []
    errors.extend(
        validate_csv(
            COMMANDS_CSV,
            {
                "command",
                "aliases",
                "category",
                "purpose",
                "param_shape",
                "required_fields",
                "common_fields",
                "selector_supported",
                "platforms",
                "example",
                "notes",
            },
        )
    )
    errors.extend(
        validate_csv(
            SELECTORS_CSV,
            {
                "selector",
                "aliases",
                "platforms",
                "stability_rank",
                "when_to_use",
                "example",
                "anti_patterns",
            },
        )
    )
    errors.extend(
        validate_csv(
            CLI_CSV,
            {
                "command",
                "category",
                "purpose",
                "platforms",
                "common_options",
                "machine_readable",
                "agent_use",
                "example",
                "notes",
            },
        )
    )
    errors.extend(validate_skill_references())
    errors.extend(validate_reference_navigation())
    errors.extend(validate_agents_metadata())
    errors.extend(validate_helper_script_reference())
    errors.extend(validate_helper_script_behavior())
    errors.extend(validate_testcase_design_reference())
    errors.extend(validate_debug_artifacts_reference())
    errors.extend(validate_reference_examples())
    errors.extend(validate_command_example_fields())
    errors.extend(validate_selector_catalog())
    errors.extend(validate_selector_quality())
    errors.extend(validate_cli_catalog())

    parser_names = parser_commands()
    csv_names = csv_command_names()
    cli_names = csv_cli_names()
    schema_names = schema_command_names()
    errors.extend(validate_yaml_command_examples(parser_names))
    errors.extend(validate_shell_cli_examples(cli_names))

    missing = sorted(parser_names.difference(csv_names))
    if missing:
        errors.append(
            "commands.csv is missing parser commands/aliases: " + ", ".join(missing)
        )

    extra_csv = sorted(csv_names.difference(parser_names))
    if extra_csv:
        errors.append(
            "commands.csv contains commands/aliases not accepted by parser: "
            + ", ".join(extra_csv)
        )

    schema_missing = sorted(parser_names.difference(schema_names))
    if schema_missing:
        errors.append(
            "lumi-test.schema.json is missing parser commands/aliases: "
            + ", ".join(schema_missing)
        )

    extra_schema = sorted(schema_names.difference(parser_names))
    if extra_schema:
        errors.append(
            "lumi-test.schema.json contains commands/aliases not accepted by parser: "
            + ", ".join(extra_schema)
        )

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print("AI references are in sync.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
