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
    "shell",
    "system",
    "validate",
]

AGENT_COMMANDS = [
    "agent-check",
    "agent-debug",
    "agent-doctor",
    "agent-list",
    "agent-run",
    "agent-schema",
    "agent-validate",
]

PLATFORMS = ["android", "android_auto", "ios", "web", "macos", "windows"]


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
    parser.add_argument("--platform", required=True, choices=PLATFORMS)
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
    parser.add_argument("--platform", required=True, choices=PLATFORMS)
    parsed = parser.parse_args(argv)
    return ["--platform", parsed.platform, "--json"]


def parse_agent_check(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run Lumi authoring gates and optional runtime execution"
    )
    parser.add_argument("path", help="YAML file or test directory")
    parser.add_argument("--platform", choices=PLATFORMS)
    parser.add_argument("--device", help="Android serial, iOS UDID, or browser/device target")
    parser.add_argument("--output", default="./output/lumi-agent")
    parser.add_argument(
        "--summary-json",
        help="Write a compact JSON summary for AI final-answer evidence",
    )
    parser.add_argument(
        "--run",
        action="store_true",
        help="Run the test after validate/list and doctor pass",
    )
    parsed, unknown = parser.parse_known_args(argv)
    parsed.extra = unknown
    if parsed.run and not parsed.platform:
        parser.error("--run requires --platform")
    return parsed


def write_agent_check_summary(
    parsed: argparse.Namespace,
    steps: list[dict[str, object]],
    status: str,
    exit_code: int,
) -> None:
    if not parsed.summary_json:
        return
    summary_path = Path(parsed.summary_json)
    summary_path.parent.mkdir(parents=True, exist_ok=True)
    summary = {
        "status": status,
        "exitCode": exit_code,
        "path": parsed.path,
        "platform": parsed.platform,
        "device": parsed.device,
        "runRequested": parsed.run,
        "output": parsed.output if parsed.run else None,
        "steps": steps,
    }
    summary_path.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")


def run_agent_check(argv: list[str]) -> int:
    parsed = parse_agent_check(argv)
    steps: list[dict[str, object]] = []

    print("== lumi agent-check: validate --json ==", file=sys.stderr)
    code = run_lumi("validate", [parsed.path, "--json"])
    steps.append({"name": "validate", "passed": code == 0, "exitCode": code})
    if code != 0:
        print("== lumi agent-check: validate FAILED ==", file=sys.stderr)
        write_agent_check_summary(parsed, steps, "failed", code)
        return code
    print("== lumi agent-check: validate PASSED ==", file=sys.stderr)

    print("== lumi agent-check: list --json ==", file=sys.stderr)
    code = run_lumi("list", [parsed.path, "--json"])
    steps.append({"name": "list", "passed": code == 0, "exitCode": code})
    if code != 0:
        print("== lumi agent-check: list FAILED ==", file=sys.stderr)
        write_agent_check_summary(parsed, steps, "failed", code)
        return code
    print("== lumi agent-check: list PASSED ==", file=sys.stderr)

    if parsed.platform:
        print("== lumi agent-check: doctor --json ==", file=sys.stderr)
        code = run_lumi("doctor", ["--platform", parsed.platform, "--json"])
        steps.append({"name": "doctor", "passed": code == 0, "exitCode": code})
        if code != 0:
            print("== lumi agent-check: doctor FAILED ==", file=sys.stderr)
            write_agent_check_summary(parsed, steps, "failed", code)
            return code
        print("== lumi agent-check: doctor PASSED ==", file=sys.stderr)

    if parsed.run:
        print("== lumi agent-check: run with artifacts ==", file=sys.stderr)
        code = run_lumi(
            "run",
            parse_agent_run(
                [
                    parsed.path,
                    "--platform",
                    parsed.platform,
                    "--output",
                    parsed.output,
                    *(["--device", parsed.device] if parsed.device else []),
                    *parsed.extra,
                ]
            ),
        )
        steps.append({"name": "run", "passed": code == 0, "exitCode": code})
        if code != 0:
            print("== lumi agent-check: run FAILED ==", file=sys.stderr)
            write_agent_check_summary(parsed, steps, "failed", code)
            return code
        print("== lumi agent-check: run PASSED ==", file=sys.stderr)

    print("== lumi agent-check: PASS ==", file=sys.stderr)
    write_agent_check_summary(parsed, steps, "passed", 0)
    return 0


def main(argv: list[str] | None = None) -> int:
    command, extra = parse_passthrough(sys.argv[1:] if argv is None else argv)

    if command == "agent-check":
        return run_agent_check(extra)
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
    if command == "agent-schema":
        return run_lumi("schema", ["--json", *extra])

    return run_lumi(command, extra)


if __name__ == "__main__":
    raise SystemExit(main())
