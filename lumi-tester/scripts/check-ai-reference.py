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
SELECTORS_CSV = (
    ROOT
    / "lumi-tester"
    / "ai"
    / "codex-skill"
    / "lumi-tester-agent"
    / "references"
    / "selectors.csv"
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
