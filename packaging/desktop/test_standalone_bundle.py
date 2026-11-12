"""Offline verified archive assembly regression tests (no downloads)."""
from __future__ import annotations

import hashlib
from pathlib import Path
import tempfile
import unittest
import zipfile

from standalone_bundle import compose, sha256, verified_unpack


def fakezip(path: Path, files: dict[str, bytes]) -> Path:
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, contents in files.items():
            archive.writestr(name, contents)
    return path


class BundleTest(unittest.TestCase):
    def test_hash_checked_platform_archives_and_normalization(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            speaches = fakezip(root / "stt.zip", {
                "Cloud9-Speaches-dist/Cloud9-Speaches.exe": b"acoustic",
                "Cloud9-Speaches-dist/_internal/framework.dll": b"runtime",
                "Cloud9-Speaches-dist/_internal/model_aliases.json": b'{"fr-quality":"some/model"}',
                "Cloud9-Speaches-dist/_internal/faster_whisper/assets/silero_encoder_v5.onnx": b"encoder",
                "Cloud9-Speaches-dist/_internal/faster_whisper/assets/silero_decoder_v5.onnx": b"decoder",
            })
            cpu = fakezip(root / "cpu.zip", {
                "llama-b123/bin/llama-server.exe": b"cpu",
                "llama-b123/bin/ggml-base.dll": b"ggml",
            })
            vulkan = fakezip(root / "vulkan.zip", {
                "bin/llama-server.exe": b"vulkan",
                "bin/ggml-vulkan.dll": b"driver",
            })
            cuda = fakezip(root / "cuda.zip", {
                "bin/llama-server.exe": b"cuda",
                "bin/ggml-cuda.dll": b"cuda-backend",
            })
            cudart = fakezip(root / "cudart.zip", {
                "cudart/cudart64_12.dll": b"runtime",
            })
            result = compose(
                platform="windows-amd64", speaches=speaches,
                speaches_sha=sha256(speaches),
                llm_cpu=cpu, llm_cpu_sha=sha256(cpu),
                llm_vulkan=vulkan, llm_vulkan_sha=sha256(vulkan),
                llm_cuda=cuda, llm_cuda_sha=sha256(cuda),
                cuda_runtime=cudart, cuda_runtime_sha=sha256(cudart),
                output=root/"engines",
            )
            self.assertEqual((root/"engines"/"llm"/"cuda"/"cudart64_12.dll").read_bytes(), b"runtime")
            self.assertEqual((root/"engines"/"speaches"/"_internal"/"framework.dll").read_bytes(), b"runtime")
            self.assertIn("llama-cuda", result["source_archive_sha256"])
            self.assertEqual((root/"engines/speaches/_internal/model_aliases.json").read_bytes(), b'{"fr-quality":"some/model"}')
            self.assertTrue((root/"engines"/"BUNDLED-RUNTIMES-SHA256.json").exists())
            with self.assertRaises(FileExistsError):
                compose(platform="windows-amd64", speaches=speaches, speaches_sha=sha256(speaches),
                        llm_cpu=cpu, llm_cpu_sha=sha256(cpu), output=root/"engines")

    def test_rejects_real_audio_blockers_missing_from_old_rc6(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cpu = fakezip(root/"cpu.zip", {"llama-server.exe": b"cpu"})
            for missing in ("alias", "encoder", "decoder"):
                assets = {
                    "Cloud9-Speaches.exe": b"stt",
                    "_internal/model_aliases.json": b"{}",
                    "_internal/faster_whisper/assets/silero_encoder_v5.onnx": b"encoder",
                    "_internal/faster_whisper/assets/silero_decoder_v5.onnx": b"decoder",
                }
                name = {
                    "alias": "_internal/model_aliases.json",
                    "encoder": "_internal/faster_whisper/assets/silero_encoder_v5.onnx",
                    "decoder": "_internal/faster_whisper/assets/silero_decoder_v5.onnx",
                }[missing]
                del assets[name]
                archive = fakezip(root / f"missing-{missing}.zip", assets)
                with self.subTest(missing=missing), self.assertRaisesRegex(ValueError, "Incomplete Cloud9-Speaches ASR bundle"):
                    compose(platform="windows-amd64", speaches=archive,
                            speaches_sha=sha256(archive),
                            llm_cpu=cpu, llm_cpu_sha=sha256(cpu),
                            output=root/f"no-{missing}")


    def test_refuses_wrong_sha_before_unpacking(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = fakezip(root/"fake.zip", {"file": b"good"})
            with self.assertRaisesRegex(ValueError, "SHA256 mismatch"):
                verified_unpack(archive, "a"*64, root/"extracted")

    def test_refuses_path_traversal_and_symlinks(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            traversal = fakezip(root/"traversal.zip", {"../secret": b"forbidden"})
            with self.assertRaisesRegex(ValueError, "Unsafe archive"):
                verified_unpack(traversal, sha256(traversal), root/"out")
            symlink = root/"symlink.zip"
            with zipfile.ZipFile(symlink, "w") as z:
                info = zipfile.ZipInfo("engine/executable")
                info.create_system = 3
                info.external_attr = (0o120777 << 16)
                z.writestr(info, "../bad")
            with self.assertRaisesRegex(ValueError, "Unsafe archive"):
                verified_unpack(symlink, sha256(symlink), root/"out2")

    def test_cuda_requires_verified_runtime(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            stt = fakezip(root/"stt.zip", {"Cloud9-Speaches.exe": b"stt"})
            cpu = fakezip(root/"cpu.zip", {"llama-server.exe": b"cpu"})
            cuda = fakezip(root/"cuda.zip", {"llama-server.exe": b"cuda"})
            with self.assertRaisesRegex(ValueError, "CUDA engine"):
                compose(platform="windows-amd64", speaches=stt, speaches_sha=sha256(stt),
                        llm_cpu=cpu, llm_cpu_sha=sha256(cpu), llm_cuda=cuda,
                        llm_cuda_sha=sha256(cuda), output=root/"out")


if __name__ == "__main__":
    unittest.main()
