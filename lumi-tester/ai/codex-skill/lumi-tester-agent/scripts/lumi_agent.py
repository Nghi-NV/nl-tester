#!/usr/bin/env python3
"""Small helper for running Lumi Tester agent commands from any workspace."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path


LUMI_COMMANDS = [
    "ai",
    "devices",
    "doctor",
    "inspect",
    "list",
    "record",
    "report",
    "run",
    "schema",
    "validate",
]

AGENT_COMMANDS = [
    "agent-debug",
    "agent-doctor",
    "agent-list",
    "agent-run",
    "agent-validate",
]


def find_repo_root(start: Path) -> Path | None:
    current = start.resolve()
    for candidate in [current, *current.parents]:
        if (candidate / "lumi-tester" / "Cargo.toml").exists():
            return candidate
    return None


def build_command(command: str, extra: list[str]) -> tuple[list[str], Path | None]:
    repo = find_repo_root(Path.cwd())
    if repo is not None:
        manifest = repo / "lumi-tester" / "Cargo.toml"
        return [
            "cargo",
            "run",
            "--manifest-path",
            str(manifest),
            "--",
            command,
            *extra,
        ], None

    binary = shutil.which("lumi-tester")
    if binary:
        return [binary, command, *extra], None

    raise SystemExit(
        "Could not find repo-local lumi-tester/Cargo.toml or installed lumi-tester binary"
    )


def run_lumi(command: str, extra: list[str]) -> int:
    invocation, cwd = build_command(command, extra)
    proc = subprocess.run(invocation, cwd=cwd, text=True, capture_output=True)

    if proc.stdout:
        print(proc.stdout, end="")
    if proc.stderr:
        print(proc.stderr, end="", file=sys.stderr)

    if command in {"devices", "doctor", "list", "schema", "validate"} and proc.stdout.strip():
        try:
            json.loads(proc.stdout)
        except json.JSONDecodeError:
            pass

    return proc.returncode


def parse_passthrough(argv: list[str]) -> tuple[str, list[str]]:
    parser = argparse.ArgumentParser(description="Run Lumi Tester commands for agents")
    parser.add_argument(
        "command",
        choices=[*LUMI_COMMANDS, *AGENT_COMMANDS],
        help="Lumi Tester command to run",
    )
    parser.add_argument("extra", nargs=argparse.REMAINDER)
    parsed = parser.parse_args(argv)
    return parsed.command, parsed.extra


def parse_agent_run(argv: list[str], *, require_command_index: bool = False) -> list[str]:
    parser = argparse.ArgumentParser(
        description="Run a Lumi test with debug-friendly artifacts enabled"
    )
    parser.add_argument("path", help="YAML file or test directory")
    parser.add_argument("--platform", required=True, choices=["android", "ios", "web"])
    parser.add_argument("--device", help="Android serial, iOS UDID, or browser/device target")
    parser.add_argument("--output", default="./output/lumi-agent")
    parser.add_argument("--command-index", type=int, required=require_command_index)
    parsed, unknown = parser.parse_known_args(argv)

    extra = [
        parsed.path,
        "--platform",
        parsed.platform,
        "--report",
        "--snapshot",
        "--events-jsonl",
        "--output",
        parsed.output,
    ]
    if parsed.device:
        extra.extend(["--device", parsed.device])
    if parsed.command_index is not None:
        extra.extend(["--command-index", str(parsed.command_index)])
    extra.extend(unknown)
    return extra


def parse_agent_path_json(argv: list[str], command: str) -> list[str]:
    parser = argparse.ArgumentParser(description=f"Run Lumi {command} with JSON output")
    parser.add_argument("path", help="YAML file or test directory")
    parsed = parser.parse_args(argv)
    return [parsed.path, "--json"]


def parse_agent_doctor(argv: list[str]) -> list[str]:
    parser = argparse.ArgumentParser(description="Run Lumi doctor with JSON output")
    parser.add_argument("--platform", required=True, choices=["android", "ios", "web"])
    parsed = parser.parse_args(argv)
    return ["--platform", parsed.platform, "--json"]


def main(argv: list[str] | None = None) -> int:
    command, extra = parse_passthrough(sys.argv[1:] if argv is None else argv)

    if command == "agent-run":
        return run_lumi("run", parse_agent_run(extra))
    if command == "agent-debug":
        return run_lumi("run", parse_agent_run(extra, require_command_index=True))
    if command == "agent-validate":
        return run_lumi("validate", parse_agent_path_json(extra, "validate"))
    if command == "agent-list":
        return run_lumi("list", parse_agent_path_json(extra, "list"))
    if command == "agent-doctor":
        return run_lumi("doctor", parse_agent_doctor(extra))

    return run_lumi(command, extra)


if __name__ == "__main__":
    raise SystemExit(main())
