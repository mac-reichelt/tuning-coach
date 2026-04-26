#!/usr/bin/env python3
"""
Apply tech-writer patches with strict path validation.

Reads a verdict JSON file (path passed as argv[1]) shaped like:
  {"patches":[{"path":"docs/foo.md","operation":"create|update|delete","content":"..."}]}

Refuses any path that:
  - is absolute
  - contains `..` segments
  - resolves outside the repo root (via realpath)
  - is not in the docs/ tree or one of the allowed root markdown files

Prints a summary line per applied patch. Exits 0 on success.
"""
from __future__ import annotations

import json
import os
import sys
from pathlib import Path

ALLOWED_ROOT_FILES = {"README.md", "CONTRIBUTING.md", "SECURITY.md", "CHANGELOG.md"}


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: apply-tech-writer-patches.py VERDICT_JSON", file=sys.stderr)
        return 2

    repo_root = Path(os.environ.get("GITHUB_WORKSPACE", os.getcwd())).resolve()
    verdict_path = Path(sys.argv[1])
    data = json.loads(verdict_path.read_text())
    patches = data.get("patches", []) or []

    applied = 0
    for entry in patches:
        raw_path = entry.get("path", "")
        op = entry.get("operation", "")

        if not raw_path or not isinstance(raw_path, str):
            print("::warning::patch missing path; skipping")
            continue

        # Reject absolute paths and any traversal segments before resolution
        if os.path.isabs(raw_path) or ".." in Path(raw_path).parts:
            print(f"::warning::refusing patch with absolute or traversal path: {raw_path}")
            continue

        # Resolve and verify it stays inside repo root
        candidate = (repo_root / raw_path).resolve()
        try:
            candidate.relative_to(repo_root)
        except ValueError:
            print(f"::warning::refusing patch outside repo root: {raw_path}")
            continue

        rel = candidate.relative_to(repo_root)
        rel_str = rel.as_posix()
        # Allowlist: docs/** OR specific root markdown files
        in_docs = bool(rel.parts) and rel.parts[0] == "docs"
        in_root_allow = len(rel.parts) == 1 and rel_str in ALLOWED_ROOT_FILES
        if not (in_docs or in_root_allow):
            print(f"::warning::refusing patch outside allowlist: {rel_str}")
            continue

        if op == "delete":
            if candidate.is_file():
                candidate.unlink()
                print(f"  delete: {rel_str}")
                applied += 1
        elif op in ("create", "update"):
            content = entry.get("content", "")
            if not isinstance(content, str):
                print(f"::warning::patch content for {rel_str} is not a string; skipping")
                continue
            candidate.parent.mkdir(parents=True, exist_ok=True)
            candidate.write_text(content)
            print(f"  {op}: {rel_str}")
            applied += 1
        else:
            print(f"::warning::unknown operation '{op}' for {rel_str}")

    print(f"applied={applied}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
