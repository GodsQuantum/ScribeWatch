#!/usr/bin/env python3
"""Deterministic, offline assembly of third-party native engines into one installer.

All inputs must be downloaded separately from pinned GitHub releases/runs;
every archive's SHA256 is verified before it is read. Never download inside
this build step or silently mix runtimes from different platforms.
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import stat
from tempfile import TemporaryDirectory
import zipfile

MAX_UNPACKED = 12 * 1024**3
MAX_FILES = 90000


def sha256(path: Path) -> str:
    with path.open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def verified_unpack(archive: Path, expected: str, dest: Path) -> int:
    expected = expected.strip().removeprefix("sha256:").lower()
    if len(expected) != 64 or not all(c in "0123456789abcdef" for c in expected):
        raise ValueError("Expected SHA256 must be 64 lowercase hexadecimal characters")
    real = sha256(archive)
    if real != expected:
        raise ValueError("Archive SHA256 mismatch: " + archive.name)
    dest.mkdir(parents=True, exist_ok=False)
    total = 0
    with zipfile.ZipFile(archive) as package:
        entries = package.infolist()
        if len(entries) > MAX_FILES:
            raise ValueError("Too many zip members")
        for entry in entries:
            raw = entry.filename.replace("\\", "/")
            logical = PurePosixPath(raw)
            file_type = stat.S_IFMT(entry.external_attr >> 16)
            if (
                raw.startswith("/")
                or ".." in logical.parts
                or any(":" in part or not part for part in logical.parts)
                or file_type == stat.S_IFLNK
                or entry.file_size > MAX_UNPACKED
            ):
                raise ValueError("Unsafe archive member")
            total += entry.file_size
            if total > MAX_UNPACKED:
                raise ValueError("Unpacked asset exceeds 12 GiB budget")
        for entry in entries:
            logical = PurePosixPath(entry.filename.replace("\\", "/"))
            if entry.is_dir():
                (dest / logical).mkdir(parents=True, exist_ok=True)
                continue
            target = dest / logical
            target.parent.mkdir(parents=True, exist_ok=True)
            # Never trust ZipFile.extractall with an untrusted asset.
            with package.open(entry) as src, target.open("xb") as dst:
                shutil.copyfileobj(src, dst, length=1024 * 1024)
    return total


def normalize_binary(source: Path, target: Path, executable: str) -> None:
    matches = [p for p in source.rglob(executable) if p.is_file()]
    if len(matches) != 1:
        raise ValueError("Expected one " + executable + ", found " + str(len(matches)))
    shutil.copytree(matches[0].parent, target)


def verify_speaches_native_assets(root: Path) -> None:
    """Reject a portable ASR that starts but fails its first real audio request.

    Both VAD assets and the ID map are required at runtime by bundled
    faster_whisper and speaches.model_aliases, respectively.
    """
    expected = (
        "_internal/model_aliases.json",
        "_internal/faster_whisper/assets/silero_encoder_v5.onnx",
        "_internal/faster_whisper/assets/silero_decoder_v5.onnx",
    )
    for relative in expected:
        asset = root / relative
        if not asset.is_file() or not asset.stat().st_size:
            raise ValueError(f"Incomplete Cloud9-Speaches ASR bundle: {relative}")
    aliases = json.loads((root / expected[0]).read_text(encoding="utf-8"))
    if not isinstance(aliases, dict) or any(
        not isinstance(key, str) or not isinstance(value, str)
        for key, value in aliases.items()
    ):
        raise ValueError("Cloud9-Speaches alias map must be a string-to-string JSON object")


def compose(
    *,
    platform: str,
    speaches: Path,
    speaches_sha: str,
    llm_cpu: Path,
    llm_cpu_sha: str,
    output: Path,
    llm_vulkan: Path | None = None,
    llm_vulkan_sha: str = "",
    llm_cuda: Path | None = None,
    llm_cuda_sha: str = "",
    cuda_runtime: Path | None = None,
    cuda_runtime_sha: str = "",
) -> dict:
    if platform != "windows-amd64":
        raise ValueError("Current verified full standalone target is Windows x64")
    if output.exists():
        raise FileExistsError("Refusing to overwrite existing engine staging directory")
    if bool(llm_cuda) != bool(cuda_runtime):
        raise ValueError("CUDA engine and NVIDIA DLL runtime must both be supplied")
    supplied = [
        ("cloud9-speaches", speaches, speaches_sha),
        ("llama-cpu", llm_cpu, llm_cpu_sha),
    ]
    for label, filename, digest in [
        ("llama-vulkan", llm_vulkan, llm_vulkan_sha),
        ("llama-cuda", llm_cuda, llm_cuda_sha),
        ("cuda-runtime", cuda_runtime, cuda_runtime_sha),
    ]:
        if filename is not None:
            supplied.append((label, filename, digest))
    with TemporaryDirectory(prefix="scribewatch-engines-build-", dir=output.parent) as temp:
        scratch = Path(temp)
        verified: dict[str, str] = {}
        for label, filename, digest in supplied:
            verified_unpack(filename, digest, scratch / label)
            verified[label] = sha256(filename)
        engines = scratch / "engines"
        engines.mkdir()
        normalize_binary(scratch / "cloud9-speaches", engines / "speaches", "Cloud9-Speaches.exe")
        verify_speaches_native_assets(engines / "speaches")
        root = engines / "llm"
        root.mkdir()
        normalize_binary(scratch / "llama-cpu", root / "cpu", "llama-server.exe")
        if llm_vulkan is not None:
            normalize_binary(scratch / "llama-vulkan", root / "vulkan", "llama-server.exe")
        if llm_cuda is not None:
            normalize_binary(scratch / "llama-cuda", root / "cuda", "llama-server.exe")
            assert cuda_runtime is not None
            runtimes = list((scratch / "cuda-runtime").rglob("*.dll"))
            if not runtimes:
                raise ValueError("CUDA runtime asset does not contain Windows DLLs")
            for dll in runtimes:
                target = root / "cuda" / dll.name
                if target.exists():
                    if sha256(target) != sha256(dll):
                        raise ValueError("Conflicting NVIDIA runtime DLL: " + dll.name)
                    continue
                shutil.copy2(dll, target)
        # Preserve the actual authors' upstream license texts in the redistributable.
        license_source = Path(__file__).parent / "third_party" / "llama.cpp-LICENSE.txt"
        if not license_source.is_file():
            raise RuntimeError("Missing verified llama.cpp MIT license notice")
        license_dest = engines / "licenses"
        license_dest.mkdir()
        shutil.copy2(license_source, license_dest / "llama.cpp-LICENSE.txt")
        (engines / "THIRD-PARTY-NATIVE-RUNTIMES.txt").write_text(
            "Speaches Reloaded: source https://github.com/GodsQuantum/Speaches-Reloaded; "
            "the distributed speaches/bin/LICENSE and bin/THIRD_PARTY_NOTICES.txt apply.\n"
            "llama.cpp: source https://github.com/ggml-org/llama.cpp; "
            "see licenses/llama.cpp-LICENSE.txt.\n"
            "Exact release archive hashes and runtime files are listed in "
            "BUNDLED-RUNTIMES-SHA256.json.\n"
            "On first normal desktop launch, Parakeet Ultra and Qwen3.5-4B model weights download automatically into the local cache.\n",
            encoding="utf-8",
        )
        all_files = sorted(p for p in engines.rglob("*") if p.is_file())
        if len(all_files) < 3:
            raise ValueError("Bundled engines unexpectedly empty")
        manifest = {
            "platform": platform,
            "source_archive_sha256": verified,
            "files": {p.relative_to(engines).as_posix(): sha256(p) for p in all_files},
        }
        (engines / "BUNDLED-RUNTIMES-SHA256.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        shutil.move(str(engines), str(output))
    return manifest


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", default="windows-amd64")
    parser.add_argument("--speaches", type=Path, required=True)
    parser.add_argument("--speaches-sha", required=True)
    parser.add_argument("--llm-cpu", type=Path, required=True)
    parser.add_argument("--llm-cpu-sha", required=True)
    parser.add_argument("--llm-vulkan", type=Path)
    parser.add_argument("--llm-vulkan-sha", default="")
    parser.add_argument("--llm-cuda", type=Path)
    parser.add_argument("--llm-cuda-sha", default="")
    parser.add_argument("--cuda-runtime", type=Path)
    parser.add_argument("--cuda-runtime-sha", default="")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    result = compose(
        platform=args.platform, speaches=args.speaches, speaches_sha=args.speaches_sha,
        llm_cpu=args.llm_cpu, llm_cpu_sha=args.llm_cpu_sha,
        llm_vulkan=args.llm_vulkan, llm_vulkan_sha=args.llm_vulkan_sha,
        llm_cuda=args.llm_cuda, llm_cuda_sha=args.llm_cuda_sha,
        cuda_runtime=args.cuda_runtime, cuda_runtime_sha=args.cuda_runtime_sha,
        output=args.output,
    )
    print("STANDALONE_ENGINES_ASSEMBLED", len(result["files"]), "files", args.output)


if __name__ == "__main__":
    main()
