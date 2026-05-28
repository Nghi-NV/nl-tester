#!/usr/bin/env python3
"""Validate AI reference CSV files against parser command names."""

from __future__ import annotations

import csv
import json
import re
import sys
from pathlib import Path


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
SCHEMA_JSON = ROOT / "lumi-tester" / "schema" / "lumi-test.schema.json"


def parser_commands() -> set[str]:
    commands: set[str] = set()
    arm_re = re.compile(r"^\s*((?:\"[^\"]+\"\s*(?:\|\s*)?)+)\s*=>")
    for line in YAML_RS.read_text(encoding="utf-8").splitlines():
        match = arm_re.match(line)
        if not match:
            continue
        commands.update(re.findall(r'"([^"]+)"', match.group(1)))
    return commands


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


def validate_skill_references() -> list[str]:
    errors: list[str] = []
    text = SKILL_MD.read_text(encoding="utf-8")
    refs = sorted(set(re.findall(r"references/[A-Za-z0-9_.-]+", text)))
    for ref in refs:
        path = SKILL_DIR / ref
        if not path.exists():
            errors.append(f"{SKILL_MD}: referenced file does not exist: {ref}")
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
    errors.extend(validate_reference_examples())

    parser_names = parser_commands()
    csv_names = csv_command_names()
    schema_names = schema_command_names()

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
