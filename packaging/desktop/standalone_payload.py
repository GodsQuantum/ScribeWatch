#!/usr/bin/env python3
"""Compress verified ScribeWatch full Windows stage under stock NSIS's 2 GiB ceiling.

Windows 11 ships with bsdtar, which can extract ZIP from the embedded
payload. The resulting setup remains a single EXE and has no external
downloads, additional services, or third-party archive tool dependency.
"""
from __future__ import annotations

import json
from pathlib import Path
import zipfile

MAX_NSIS_INPUT_BYTES = 1_950_000_000


def make_payload(stage: Path, output: Path) -> tuple[int, int]:
    if not stage.is_dir():
        raise ValueError("Installer stage directory is missing")
    if output.exists():
        raise FileExistsError("Refusing to overwrite pre-existing installer payload")
    manifest = stage / "manifest-sha256.json"
    if not manifest.is_file():
        raise ValueError("Installer stage must be SHA-256 verified before archiving")
    metadata = json.loads(manifest.read_text(encoding="utf-8"))
    files = sorted(path for path in stage.rglob("*") if path.is_file())
    if len(metadata.get("files", {})) != len(files) - 1:
        raise ValueError("Installer stage manifest and physical file list differ")
    if any(file.is_symlink() for file in files):
        raise ValueError("Installer payload must not include symlinks")
    output.parent.mkdir(parents=True, exist_ok=True)
    try:
        with zipfile.ZipFile(
            output,
            mode="x",
            compression=zipfile.ZIP_DEFLATED,
            compresslevel=2,
            allowZip64=True,
        ) as archive:
            for file in files:
                archive.write(file, file.relative_to(stage).as_posix())
                if output.stat().st_size > MAX_NSIS_INPUT_BYTES:
                    raise ValueError("Installer payload exceeds stock NSIS 2 GiB file limit")
        if output.stat().st_size > MAX_NSIS_INPUT_BYTES:
            raise ValueError("Installer payload exceeds stock NSIS 2 GiB file limit")
        with zipfile.ZipFile(output) as archive:
            names = archive.namelist()
            if len(names) != len(files) or len(names) != len(set(names)):
                raise ValueError("Installer payload has missing or duplicate files")
            if set(names) != {p.relative_to(stage).as_posix() for p in files}:
                raise ValueError("Installer payload file names mismatch")
        return len(files), output.stat().st_size
    except BaseException:
        output.unlink(missing_ok=True)
        raise


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--stage", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    count, size = make_payload(args.stage, args.output)
    print("NSIS_ONE_EXE_PAYLOAD_READY", count, "files", size, "bytes")
