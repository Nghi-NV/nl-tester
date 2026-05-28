#!/usr/bin/env python3
"""Small helper for running Lumi Tester agent commands from any workspace."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path


def find_repo_root(start: Path) -> Path | None:
    current = start.resolve()
    for candidate in [current, *current.parents]:
        if (candidate / "lumi-tester" / "Cargo.toml").exists():
            return candidate
    return None


def build_command(args: argparse.Namespace) -> tuple[list[str], Path | None]:
    repo = find_repo_root(Path.cwd())
    if repo is not None:
        manifest = repo / "lumi-tester" / "Cargo.toml"
        return [
            "cargo",
            "run",
            "--manifest-path",
            str(manifest),
            "--",
            args.command,
            *args.extra,
        ], None

    binary = shutil.which("lumi-tester")
    if binary:
        return [binary, args.command, *args.extra], None

    raise SystemExit(
        "Could not find repo-local lumi-tester/Cargo.toml or installed lumi-tester binary"
    )


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Lumi Tester commands for agents")
    parser.add_argument(
        "command",
        choices=[
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
        ],
        help="Lumi Tester command to run",
    )
    parser.add_argument("extra", nargs=argparse.REMAINDER)
    parsed = parser.parse_args()

    command, cwd = build_command(parsed)
    proc = subprocess.run(command, cwd=cwd, text=True, capture_output=True)

    if proc.stdout:
        print(proc.stdout, end="")
    if proc.stderr:
        print(proc.stderr, end="", file=sys.stderr)

    if parsed.command in {"devices", "doctor", "list", "schema", "validate"} and proc.stdout.strip():
        try:
            json.loads(proc.stdout)
        except json.JSONDecodeError:
            pass

    return proc.returncode


if __name__ == "__main__":
    raise SystemExit(main())
