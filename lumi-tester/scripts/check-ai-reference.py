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
    errors.extend(validate_selector_catalog())
    errors.extend(validate_cli_catalog())

    parser_names = parser_commands()
    csv_names = csv_command_names()
    schema_names = schema_command_names()
    errors.extend(validate_yaml_command_examples(parser_names))

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
