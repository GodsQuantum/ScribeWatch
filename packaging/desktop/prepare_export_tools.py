#!/usr/bin/env python3
"""Reproducibly bundle Pandoc + Typst + Noto CJK for self-contained exports.

These tools are downloaded only at build time; installed apps never fetch or
install host packages. Archive paths and SHA-256 are independently validated.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import tarfile
import tempfile
import urllib.request
import zipfile

PANDOC_VERSION = "3.14"
TYPST_VERSION = "v0.15.1"
FONT_COMMIT = "f8d157532fbfaeda587e826d4cd5b21a49186f7c"
FONT_NAME = "NotoSansCJK-Regular.ttc"
FONT_SHA = "b76b0433203017ca80401b2ee0dd69350349871c4b19d504c34dbdd80541690a"
LICENSES = {
    "pandoc-COPYING.md": (
        "https://raw.githubusercontent.com/jgm/pandoc/3.14/COPYING.md",
        "3ebfbf9235b049d384cdba5a66a1a3434bb544391e720b76faa8e1b3ee61a4e5",
    ),
    "NotoSansCJK-LICENSE": (
        f"https://raw.githubusercontent.com/notofonts/noto-cjk/{FONT_COMMIT}/Sans/LICENSE",
        "6a73f9541c2de74158c0e7cf6b0a58ef774f5a780bf191f2d7ec9cc53efe2bf2",
    ),
}
FONT_URL = (
    f"https://raw.githubusercontent.com/notofonts/noto-cjk/{FONT_COMMIT}"
    f"/Sans/OTC/{FONT_NAME}"
)
RELEASES = {
    "windows-amd64": (
        ("pandoc-3.14-windows-x86_64.zip",
         "0144b96e8c7b023fd382ec5ef5bdc475e873f1019d6fa72502492df2c9a94a3b"),
        ("typst-x86_64-pc-windows-msvc.zip",
         "19ce3551153c2fe7ee9fa2f95208310c8f4d3209fedb699e0333faf8913f6736"),
    ),
    "macos-amd64": (
        ("pandoc-3.14-x86_64-macOS.zip",
         "891708579061e7b1c1a6ce0df30ebb3cb2b6d0ca045ed3c8554ab830c1e0c270"),
        ("typst-x86_64-apple-darwin.tar.xz",
         "7f9fdd9584866245de9a79e0add8f9236fae6f40a8a45e2c4771ccc14db4e0fa"),
    ),
    "macos-arm64": (
        ("pandoc-3.14-arm64-macOS.zip",
         "58d6593884fec9d4424b641b92503c316d9c36decb2891ff8d73ae74dd3b6304"),
        ("typst-aarch64-apple-darwin.tar.xz",
         "48f62ed034aa3a7978309579ac6ca00045e2ef0da73114e8af27cfd8e74dc05a"),
    ),
    "linux-amd64": (
        ("pandoc-3.14-linux-amd64.tar.gz",
         "c5f4720cdefb3f7bdc5d07202ef579feef0d1370773c873804dd8fdaa8c4f1b8"),
        ("typst-x86_64-unknown-linux-musl.tar.xz",
         "a6d077d0a95eed5a2eba715b2dae06be954f624ccbf85758a03f389ded33118c"),
    ),
    "linux-arm64": (
        ("pandoc-3.14-linux-arm64.tar.gz",
         "c692f3d78a415741bbbac4cdbbbe26fa7edca1f43d2301b982fbf4009cc8cf7f"),
        ("typst-aarch64-unknown-linux-musl.tar.xz",
         "5aa8d74a3d906e60ea12a66ac2f37f8eef1b14cbad7182a745e393a10c23dcee"),
    ),
}


def digest(path: Path) -> str:
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def verified_input(cache: Path, name: str, url: str, expected: str) -> Path:
    cache.mkdir(parents=True, exist_ok=True)
    target = cache / name
    if not target.is_file():
        request = urllib.request.Request(url, headers={"User-Agent": "ScribeWatch-build/1"})
        temporary = cache / f"{name}.part"
        try:
            with urllib.request.urlopen(request, timeout=90) as source, temporary.open("wb") as out:
                shutil.copyfileobj(source, out, 1024 * 1024)
            if digest(temporary) != expected:
                raise ValueError(f"SHA256 mismatch: {name}")
            temporary.replace(target)
        finally:
            temporary.unlink(missing_ok=True)
    if digest(target) != expected:
        raise ValueError(f"SHA256 mismatch: {name}")
    return target


def archive_file(archive: Path, basename: str) -> bytes:
    def is_safe(member_name: str) -> bool:
        path = PurePosixPath(member_name)
        return not path.is_absolute() and ".." not in path.parts
    if archive.name.lower().endswith(".zip"):
        with zipfile.ZipFile(archive) as z:
            candidates = []
            for info in z.infolist():
                if PurePosixPath(info.filename).name != basename:
                    continue
                if not is_safe(info.filename) or info.is_dir():
                    raise ValueError(f"Unsafe archive member: {info.filename}")
                mode = info.external_attr >> 16
                if mode & 0o170000 == 0o120000:
                    raise ValueError("Symlink not permitted in export tool archive")
                if info.file_size > 300_000_000:
                    raise ValueError("Excessive native binary size")
                candidates.append(info)
            if len(candidates) != 1:
                raise ValueError(f"Expected exactly one {basename} in {archive.name}")
            return z.read(candidates[0])
    with tarfile.open(archive, mode="r:*") as tar:
        candidates = [m for m in tar.getmembers() if PurePosixPath(m.name).name == basename]
        if len(candidates) != 1 or not candidates[0].isfile() \
                or not is_safe(candidates[0].name) or candidates[0].size > 300_000_000:
            raise ValueError(f"Invalid native binary {basename} in {archive.name}")
        content = tar.extractfile(candidates[0])
        if content is None:
            raise ValueError("Missing archive content")
        with content:
            return content.read()


def assemble(variant: str, output: Path, cache: Path) -> dict:
    if variant not in RELEASES:
        raise ValueError(f"Unknown export tool target {variant}")
    if output.exists():
        raise FileExistsError(f"Refusing to overwrite existing exported binaries: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = Path(tempfile.mkdtemp(prefix="scribewatch-export-tools-", dir=output.parent))
    try:
        sources = {}
        for name, (archive_name, archive_sha) in zip(("pandoc", "typst"), RELEASES[variant]):
            repository, release = (
                ("jgm/pandoc", PANDOC_VERSION) if name == "pandoc"
                else ("typst/typst", TYPST_VERSION)
            )
            url = f"https://github.com/{repository}/releases/download/{release}/{archive_name}"
            archive = verified_input(cache, archive_name, url, archive_sha)
            binary_name = name + (".exe" if variant.startswith("windows") else "")
            data = archive_file(archive, binary_name)
            binary = temporary / binary_name
            binary.write_bytes(data)
            binary.chmod(0o755)
            sources[archive_name] = archive_sha
        fonts = temporary / "fonts"
        fonts.mkdir()
        shutil.copy2(verified_input(cache, FONT_NAME, FONT_URL, FONT_SHA), fonts / FONT_NAME)
        sources[FONT_NAME] = FONT_SHA
        licenses = temporary / "licenses"
        licenses.mkdir()
        for license_name, (url, expected) in LICENSES.items():
            shutil.copy2(verified_input(cache, license_name, url, expected), licenses / license_name)
            sources[license_name] = expected
        # The official Typst archive includes the Apache license and NOTICE.
        typst_archive = cache / RELEASES[variant][1][0]
        (licenses / "typst-LICENSE").write_bytes(archive_file(typst_archive, "LICENSE"))
        (licenses / "typst-NOTICE").write_bytes(archive_file(typst_archive, "NOTICE"))
        files = {
            p.relative_to(temporary).as_posix(): digest(p)
            for p in sorted(temporary.rglob("*")) if p.is_file()
        }
        metadata = {"variant": variant, "sources_sha256": sources, "files_sha256": files,
                    "versions": {"pandoc": PANDOC_VERSION, "typst": TYPST_VERSION}}
        (temporary / "EXPORT-TOOLS-SHA256.json").write_text(
            json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        temporary.rename(output)
        return metadata
    finally:
        if temporary.exists():
            shutil.rmtree(temporary)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--variant", choices=sorted(RELEASES), required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--cache", type=Path, default=None)
    args = parser.parse_args()
    cache = args.cache or Path(tempfile.mkdtemp(prefix="scribewatch-export-download-"))
    try:
        result = assemble(args.variant, args.output, cache)
        print("SCRIBEWATCH_NATIVE_EXPORT_TOOLS_VERIFIED", result["variant"],
              len(result["files_sha256"]), flush=True)
    finally:
        if args.cache is None:
            shutil.rmtree(cache)


if __name__ == "__main__":
    main()
