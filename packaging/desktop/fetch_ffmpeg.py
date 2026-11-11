#!/usr/bin/env python3
"""Download locked FFmpeg/FFprobe bundle for native OS; do not use latest URLs."""
from __future__ import annotations
import argparse
import hashlib
import os
import shutil
import tarfile
import urllib.request
import zipfile
from pathlib import Path

VERSION = "v9.0.2"
BASE = f"https://github.com/binmgr/ffmpeg/releases/download/{VERSION}"
LOCK = {
    "windows-amd64": ("ffmpeg-windows-amd64.zip", "9ffc63d330788b8eced4bc8c4ad41490a375fb2bf208c6ccd127724f5fc281fe"),
    "darwin-amd64": ("ffmpeg-darwin-amd64.tar.gz", "360184cf61f7203ab46f409bd10eb7a1fcc5753d5e7de9eed4041bd80ced9c1c"),
    "darwin-arm64": ("ffmpeg-darwin-arm64.tar.gz", "de8099093ec4de375880717a28e59d7ccfc54001bb69da3f7a3a6a5a98c0dc91"),
}


def download(variant: str, out: Path) -> None:
    filename, digest = LOCK[variant]
    out.mkdir(parents=True, exist_ok=True)
    archive = out / filename
    if not archive.is_file():
        request = urllib.request.Request(f"{BASE}/{filename}", headers={"User-Agent": "Cloud9-Desktop-Installer/1.0"})
        with urllib.request.urlopen(request, timeout=120) as response, archive.open("wb") as target:
            shutil.copyfileobj(response, target)
    got = hashlib.file_digest(archive.open("rb"), "sha256").hexdigest()
    if got != digest:
        archive.unlink(missing_ok=True)
        raise ValueError(f"FFmpeg SHA256 mismatch for {filename}: expected {digest}, got {got}")

    executable_names = ("ffmpeg.exe", "ffprobe.exe") if variant.startswith("windows") else ("ffmpeg", "ffprobe")
    if filename.endswith(".zip"):
        with zipfile.ZipFile(archive) as contents:
            for name in executable_names:
                matches = [i for i in contents.infolist() if Path(i.filename).name == name and not i.is_dir()]
                if len(matches) != 1:
                    raise ValueError(f"{name} not exactly once in {filename}")
                with contents.open(matches[0]) as source, (out / name).open("wb") as target:
                    shutil.copyfileobj(source, target)
    else:
        with tarfile.open(archive, "r:gz") as contents:
            for name in executable_names:
                matches = [i for i in contents.getmembers() if Path(i.name).name == name and i.isfile()]
                if len(matches) != 1:
                    raise ValueError(f"{name} not exactly once in {filename}")
                with contents.extractfile(matches[0]) as source, (out / name).open("wb") as target:
                    assert source is not None
                    shutil.copyfileobj(source, target)
    for name in executable_names:
        (out / name).chmod(0o755)
    print(f"FFMPEG_FFPROBE_LOCKED_OK={VERSION}/{variant} {digest}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--variant", required=True, choices=list(LOCK))
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    download(args.variant, args.output)
