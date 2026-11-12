#!/usr/bin/env python3
"""Stage verified Speaches Reloaded and llama.cpp engines for Linux/macOS desktop.

Never downloads or executes an archive: CI fetches pinned releases and confirms
the exact SHA-256 before assembly. The Windows assembler remains untouched.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import stat
import tarfile
from pathlib import Path, PurePosixPath
from tempfile import TemporaryDirectory
from zipfile import ZipFile

from standalone_bundle import sha256, verified_unpack, verify_speaches_native_assets

SUPPORTED = {"linux-amd64", "linux-arm64", "macos-amd64", "macos-arm64"}
MAX_UNPACKED = 12 * 1024**3


def checked_sha(digest: str) -> str:
    digest = digest.removeprefix("sha256:").lower().strip()
    if len(digest) != 64 or any(c not in "0123456789abcdef" for c in digest):
        raise ValueError("Expected verified source SHA256")
    return digest


def extract_llama(archive: Path, expected_sha: str, output: Path) -> None:
    expected = checked_sha(expected_sha)
    if sha256(archive) != expected:
        raise ValueError("Archive SHA256 mismatch: " + archive.name)
    output.mkdir(parents=True, exist_ok=False)
    total = 0
    seen: set[str] = set()
    with tarfile.open(archive, mode="r:gz") as data:
        members = data.getmembers()
        if len(members) > 120_000:
            raise ValueError("Too many llama.cpp members")
        for member in members:
            logical = PurePosixPath(member.name)
            if logical.is_absolute() or ".." in logical.parts or not logical.parts or logical.parts[0] != "llama-b12110":
                raise ValueError("Unsafe llama.cpp archive path: " + member.name)
            if member.issym() or member.islnk():
                link = PurePosixPath(member.linkname)
                if link.is_absolute() or ".." in link.parts:
                    raise ValueError("Unsafe llama.cpp symlink target")
            elif not (member.isfile() or member.isdir()):
                raise ValueError("Unsafe llama.cpp archive member")
            total += max(0, member.size)
            if total > MAX_UNPACKED:
                raise ValueError("Oversized llama.cpp archive")
        for member in members:
            if not (member.isfile() or member.issym() or member.islnk()):
                continue
            name = PurePosixPath(member.name).name
            # Upstream bundles also contain CLI binaries unrelated to the server.
            if name != "llama-server" and not (
                name.startswith("lib") or name.endswith(".metal") or name == "LICENSE"
            ):
                continue
            if name in seen:
                raise ValueError("Duplicate bundled llama.cpp runtime file")
            source = data.extractfile(member)
            if source is None:
                raise ValueError("Missing archive content: " + member.name)
            target = output / name
            with source, target.open("xb") as destination:
                shutil.copyfileobj(source, destination, length=1024 * 1024)
            target.chmod(0o755 if name == "llama-server" else 0o644)
            seen.add(name)
    if "llama-server" not in seen:
        raise ValueError("Missing llama.cpp server binary")


def assemble_speaches(
    root: Path, *, archive: Path | None, digest: str,
    tree: Path | None, output: Path,
) -> None:
    if (archive is None) == (tree is None):
        raise ValueError("Exactly one Speaches Reloaded archive or verified macOS tree is required")
    if archive is not None:
        with TemporaryDirectory(prefix="scribewatch-reloaded-") as temp:
            extracted = Path(temp) / "unpacked"
            verified_unpack(archive, digest, extracted)
            matches = [
                p for p in extracted.rglob("Cloud9-Speaches")
                if p.is_file() and p.parent.name == "Cloud9-Speaches"
            ]
            if len(matches) != 1:
                raise ValueError("Expected one historical Speaches Reloaded portable executable")
            source_dir = matches[0].parent
            # zipfile ignores the Unix executable bit while extracting: restore
            # precisely the executable flags encoded in the pinned release.
            with ZipFile(archive) as zipped:
                member_modes = {
                    PurePosixPath(info.filename).as_posix(): info.external_attr >> 16
                    for info in zipped.infolist()
                }
            for file in source_dir.rglob("*"):
                if not file.is_file():
                    continue
                rel = file.relative_to(extracted).as_posix()
                if member_modes.get(rel, 0) & 0o111:
                    file.chmod(file.stat().st_mode | 0o111)
            shutil.move(str(source_dir), output)
    else:
        if not tree.is_dir():
            raise ValueError("Mounted Speaches Reloaded macOS app tree is absent")
        shutil.copytree(tree, output, symlinks=False)
    binary = output / "Cloud9-Speaches"
    if not binary.is_file():
        raise ValueError("Missing Speaches Reloaded desktop binary")
    binary.chmod(binary.stat().st_mode | 0o111)
    verify_speaches_native_assets(output)


def compose(
    *, platform: str, output: Path, llm_cpu: Path, llm_cpu_sha: str,
    speaches: Path | None = None, speaches_sha: str = "",
    speaches_tree: Path | None = None,
    speaches_dmg: Path | None = None,
    llm_vulkan: Path | None = None, llm_vulkan_sha: str = "",
) -> dict:
    if platform not in SUPPORTED:
        raise ValueError("Unsupported native desktop platform")
    if output.exists():
        raise FileExistsError("Refusing to overwrite existing engine staging directory")
    if platform.startswith("macos") and speaches is not None:
        raise ValueError("macOS release Speaches Reloaded ships as a DMG, not a ZIP")
    if platform.startswith("linux") and speaches_tree is not None:
        raise ValueError("Linux release Speaches Reloaded requires verified ZIP")
    if platform.startswith("macos"):
        if speaches_dmg is None or not speaches_sha:
            raise ValueError("macOS DMG requires pinned SHA256")
        if sha256(speaches_dmg) != checked_sha(speaches_sha):
            raise ValueError("macOS DMG SHA256 mismatch")
    elif speaches_dmg is not None:
        raise ValueError("Linux cannot use macOS DMG")
    if llm_vulkan is not None and platform != "linux-amd64" and platform != "linux-arm64":
        raise ValueError("Vulkan bundling is Linux-only")
    source_shas = {"speaches-reloaded": checked_sha(speaches_sha)}
    source_shas["llama-cpu"] = checked_sha(llm_cpu_sha)
    if llm_vulkan:
        source_shas["llama-vulkan"] = checked_sha(llm_vulkan_sha)
    with TemporaryDirectory(prefix="scribewatch-portable-engine-") as temp:
        temp_root = Path(temp) / "engines"
        temp_root.mkdir()
        assemble_speaches(
            temp_root,
            archive=speaches,
            digest=speaches_sha,
            tree=speaches_tree,
            output=temp_root / "speaches",
        )
        cpu = temp_root / "llm"
        if llm_vulkan:
            cpu.mkdir()
            extract_llama(llm_cpu, llm_cpu_sha, cpu / "cpu")
            extract_llama(llm_vulkan, llm_vulkan_sha, cpu / "vulkan")
            # Generic fallback for installations with a missing GPU driver.
            shutil.copytree(cpu / "cpu", cpu / "fallback")
            for item in (cpu / "fallback").iterdir():
                if item.is_file():
                    shutil.move(str(item), str(cpu / item.name))
            (cpu / "fallback").rmdir()
        else:
            extract_llama(llm_cpu, llm_cpu_sha, cpu)
        if platform.startswith("linux"):
            # The pinned Speaches Reloaded Linux distribution already ships
            # OpenSSL 3. Reuse those exact runtime libraries for llama.cpp,
            # whose official Ubuntu build otherwise fails on hosts with
            # OpenSSL 4 (e.g. Ubuntu 26.10). No host package or version pin.
            for library in ("libssl.so.3", "libcrypto.so.3"):
                source = temp_root / "speaches" / "_internal" / library
                if not source.is_file():
                    raise ValueError("Speaches Reloaded missing portable OpenSSL: " + library)
                shutil.copy2(source, cpu / library)
        notice = temp_root / "THIRD-PARTY-NATIVE-RUNTIMES.txt"
        notice.write_text(
            "Speaches Reloaded: https://github.com/GodsQuantum/Speaches-Reloaded\n"
            "Historical portable binary filename Cloud9-Speaches is retained for ABI compatibility.\n"
            "Speaches bundled license/notices apply. llama.cpp: https://github.com/ggml-org/llama.cpp\n"
            "See licenses/llama.cpp-LICENSE.txt and BUNDLED-RUNTIMES-SHA256.json.\n",
            encoding="utf-8",
        )
        licenses = temp_root / "licenses"
        licenses.mkdir()
        shutil.copy2(Path(__file__).parent / "third_party" / "llama.cpp-LICENSE.txt", licenses / "llama.cpp-LICENSE.txt")
        files = {
            file.relative_to(temp_root).as_posix(): sha256(file)
            for file in sorted(temp_root.rglob("*"))
            if file.is_file()
        }
        manifest = {"platform": platform, "source_archive_sha256": source_shas, "files": files}
        (temp_root / "BUNDLED-RUNTIMES-SHA256.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        shutil.move(str(temp_root), str(output))
    return manifest


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--platform", choices=sorted(SUPPORTED), required=True)
    ap.add_argument("--speaches", type=Path)
    ap.add_argument("--speaches-sha", default="")
    ap.add_argument("--speaches-tree", type=Path)
    ap.add_argument("--speaches-dmg", type=Path)
    ap.add_argument("--llm-cpu", type=Path, required=True)
    ap.add_argument("--llm-cpu-sha", required=True)
    ap.add_argument("--llm-vulkan", type=Path)
    ap.add_argument("--llm-vulkan-sha", default="")
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    result = compose(
        platform=args.platform, speaches=args.speaches, speaches_sha=args.speaches_sha,
        speaches_tree=args.speaches_tree, speaches_dmg=args.speaches_dmg, llm_cpu=args.llm_cpu, llm_cpu_sha=args.llm_cpu_sha,
        llm_vulkan=args.llm_vulkan, llm_vulkan_sha=args.llm_vulkan_sha, output=args.output,
    )
    print("SCRIBEWATCH_PORTABLE_ENGINES_READY", result["platform"], len(result["files"]))


if __name__ == "__main__":
    main()
