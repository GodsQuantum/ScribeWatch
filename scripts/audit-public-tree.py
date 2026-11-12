#!/usr/bin/env python3
"""Fail CI before publishing private clinical media, model weights or machine paths.

Checks the Git INDEX (staged commit) rather than ignored/untracked work files.
No patient names, account details or private IP addresses are embedded here.
"""
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BLOCKED_EXTENSIONS = {
    ".m4a", ".mp3", ".wav", ".flac", ".opus", ".mp4", ".mov",
    ".gguf", ".sqlite", ".sqlite3", ".db", ".pem", ".key",
}
BLOCKED_FILENAMES = {".env", "id_rsa", "id_ed25519"}
BLOCKED_PATH_PARTS = {"benchmark-prive", ".secrets", "user-data"}
TEXT_EXTENSIONS = {
    ".md", ".txt", ".rs", ".ts", ".js", ".svelte", ".sh", ".py", ".json",
    ".yml", ".yaml", ".toml", ".css", ".html", ".nsi",
}
PRIVATE_CONTENT = [
    re.compile(rb"/home/[a-z0-9_-]+/(?:Documents|Downloads|T[\xc3\xa9]l[\xc3\xa9]chargements)/", re.I),
    re.compile(rb"[A-Z]:[\\/]Users[\\/][^\\/\s]{1,80}[\\/](?:Documents|AppData)", re.I),
    re.compile(rb"(?<![\d.])192\.168\.(?:\d{1,3}\.){1}\d{1,3}(?![\d.])"),
    re.compile(rb"(?:sk-[A-Za-z0-9]{25,}|ghp_[A-Za-z0-9]{25,}|github_pat_[A-Za-z0-9_]{40,})"),
]
MAX_PUBLIC_ARTIFACT_BYTES = 3 * 1024 * 1024


def check_tree() -> list[str]:
    tracked = subprocess.check_output(
        ["git", "ls-files", "--cached", "-z"], cwd=ROOT
    ).split(b"\0")
    issues: list[str] = []
    for item in tracked:
        if not item:
            continue
        relative = Path(item.decode("utf-8"))
        path = ROOT / relative
        if not path.is_file():
            continue  # deletion staged
        parts = {part.lower() for part in relative.parts}
        if parts.intersection(BLOCKED_PATH_PARTS):
            issues.append(f"private directory in Git index: {relative}")
        if relative.suffix.lower() in BLOCKED_EXTENSIONS or relative.name.lower() in BLOCKED_FILENAMES:
            issues.append(f"private binary/secret in Git index: {relative}")
        if path.stat().st_size > MAX_PUBLIC_ARTIFACT_BYTES:
            issues.append(f"unexpected large file in Git index: {relative}")
        if path.suffix.lower() in TEXT_EXTENSIONS and relative.name != Path(__file__).name:
            raw = path.read_bytes()
            for pattern in PRIVATE_CONTENT:
                if pattern.search(raw):
                    issues.append(f"potential private data in tracked text: {relative}")
                    break
    return issues


if __name__ == "__main__":
    p = Path(__file__)
    errors = check_tree()
    if errors:
        print("\n".join(errors), file=sys.stderr)
        sys.exit(1)
    print("PUBLIC_GIT_TREE_PRIVACY_AUDIT_PASS")
