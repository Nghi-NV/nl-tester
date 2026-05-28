#!/usr/bin/env python3
"""Refresh docs/index.html embedded markdown from source docs."""

from __future__ import annotations

import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DOCS_DIR = ROOT / "lumi-tester" / "docs"
INDEX_HTML = DOCS_DIR / "index.html"

DOC_SOURCES = {
    "commands": DOCS_DIR / "api" / "commands.md",
    "flows": DOCS_DIR / "flows" / "test_execution_flow.md",
    "writing_tests": DOCS_DIR / "writing_tests.md",
    "ai_authoring": DOCS_DIR / "ai-authoring.md",
}

PAGE_NAMES = {
    "commands": "Commands Reference",
    "flows": "Test Flows",
    "writing_tests": "Writing Tests",
    "ai_authoring": "AI Authoring",
}


def main() -> int:
    docs = {
        key: path.read_text(encoding="utf-8")
        for key, path in DOC_SOURCES.items()
    }
    html = INDEX_HTML.read_text(encoding="utf-8")
    replacement = "const docs = " + json.dumps(docs, ensure_ascii=True) + ";"
    updated, count = re.subn(
        r"const docs = \{.*?\};\n\s*const pageNames",
        lambda _match: replacement + "\n        const pageNames",
        html,
        count=1,
        flags=re.DOTALL,
    )
    if count != 1:
        raise SystemExit(f"Could not find embedded docs block in {INDEX_HTML}")
    page_names_json = json.dumps(PAGE_NAMES, ensure_ascii=True, indent=4)
    page_names_replacement = (
        "const pageNames = " + page_names_json.replace("\n", "\n        ") + ";"
    )
    updated, count = re.subn(
        r"const pageNames = \{.*?\};\n\s*function loadPage",
        lambda _match: page_names_replacement + "\n\n        function loadPage",
        updated,
        count=1,
        flags=re.DOTALL,
    )
    if count != 1:
        raise SystemExit(f"Could not find pageNames block in {INDEX_HTML}")
    INDEX_HTML.write_text(updated, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
