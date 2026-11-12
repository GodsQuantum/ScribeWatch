import hashlib
import io
import json
import tarfile
import tempfile
import unittest
import zipfile
from pathlib import Path

from portable_engines import compose

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

class PortableEngineBundlerTests(unittest.TestCase):
    def fixture(self, root):
        speech = root / "speaches.zip"
        with zipfile.ZipFile(speech, "w") as z:
            for name, content in {
                "Cloud9-Speaches/Cloud9-Speaches": b"native",
                "Cloud9-Speaches/_internal/model_aliases.json": b'{"parakeet":"repo"}',
                "Cloud9-Speaches/_internal/libssl.so.3": b"openssl-ssl",
                "Cloud9-Speaches/_internal/libcrypto.so.3": b"openssl-crypto",
                "Cloud9-Speaches/bin/libgomp.so.1": b"gcc-openmp",
                "Cloud9-Speaches/_internal/faster_whisper/assets/silero_encoder_v5.onnx": b"e",
                "Cloud9-Speaches/_internal/faster_whisper/assets/silero_decoder_v5.onnx": b"d",
            }.items():
                z.writestr(name, content)
        llm = root / "llama.tar.gz"
        with tarfile.open(llm, "w:gz") as tar:
            for name, content in {
                "llama-b12110/llama-server": b"server",
                "llama-b12110/libggml.so": b"library",
                "llama-b12110/LICENSE": b"MIT",
            }.items():
                info=tarfile.TarInfo(name)
                info.mode = 0o755 if name.endswith("llama-server") else 0o644
                info.size=len(content)
                tar.addfile(info,io.BytesIO(content))
        return speech,llm

    def test_linux_full_bundle_has_real_stt_and_llm_cpu(self):
        with tempfile.TemporaryDirectory() as work:
            root=Path(work); speech,llm=self.fixture(root)
            out=root/"engines"
            result=compose(platform="linux-amd64",speaches=speech,speaches_sha=sha(speech),llm_cpu=llm,llm_cpu_sha=sha(llm),output=out)
            self.assertTrue((out/"speaches/Cloud9-Speaches").exists())
            self.assertTrue((out/"llm/llama-server").exists())
            self.assertTrue((out/"llm/libggml.so").exists())
            self.assertTrue((out/"llm/libssl.so.3").exists())
            self.assertTrue((out/"llm/libcrypto.so.3").exists())
            self.assertEqual((out/"llm/libgomp.so.1").read_bytes(),b"gcc-openmp")
            self.assertIn("speaches/Cloud9-Speaches",result["files"])
            self.assertTrue((out/"BUNDLED-RUNTIMES-SHA256.json").exists())

    def test_macos_full_bundle_accepts_staged_speaches_app(self):
        with tempfile.TemporaryDirectory() as work:
            root=Path(work); _,llm=self.fixture(root)
            source=root/"mac"
            (source/"_internal/faster_whisper/assets").mkdir(parents=True)
            (source/"Cloud9-Speaches").write_bytes(b"mac-binary")
            (source/"_internal/model_aliases.json").write_text('{"parakeet":"repo"}')
            for name in ["silero_encoder_v5.onnx","silero_decoder_v5.onnx"]:
                (source/"_internal/faster_whisper/assets"/name).write_bytes(b"vad")
            dmg = root / "Speaches-Reloaded.dmg"
            dmg.write_bytes(b"original verified dmg release")
            out=root/"engines"
            compose(platform="macos-arm64",speaches_tree=source,
                    speaches_dmg=dmg,speaches_sha=sha(dmg),llm_cpu=llm,
                    llm_cpu_sha=sha(llm),output=out)
            self.assertTrue((out/"speaches/Cloud9-Speaches").exists())
            self.assertTrue((out/"llm/llama-server").exists())

    def test_macos_requires_sha_pinned_dmg_source(self):
        with tempfile.TemporaryDirectory() as work:
            root=Path(work);_,llm=self.fixture(root)
            tree=root/"mounted";tree.mkdir()
            with self.assertRaisesRegex(ValueError,"macOS DMG.*SHA256"):
                compose(platform="macos-arm64",speaches_tree=tree,
                        llm_cpu=llm,llm_cpu_sha=sha(llm),output=root/"engines")

    def test_sha_mismatch_rejects_prebuilt_speech(self):
        with tempfile.TemporaryDirectory() as work:
            root=Path(work);speech,llm=self.fixture(root)
            with self.assertRaisesRegex(ValueError,"SHA256"):
                compose(platform="linux-amd64",speaches=speech,speaches_sha="0"*64,llm_cpu=llm,llm_cpu_sha=sha(llm),output=root/"engines")

    def test_refuses_incomplete_acoustic_runtime(self):
        with tempfile.TemporaryDirectory() as work:
            root=Path(work);speech,llm=self.fixture(root)
            with zipfile.ZipFile(speech,"w") as z:
                z.writestr("Cloud9-Speaches/Cloud9-Speaches",b"exec")
            with self.assertRaisesRegex(ValueError,"Incomplete"):
                compose(platform="linux-amd64",speaches=speech,speaches_sha=sha(speech),llm_cpu=llm,llm_cpu_sha=sha(llm),output=root/"engines")

    def test_rejects_existing_output_to_preserve_user_files(self):
        with tempfile.TemporaryDirectory() as work:
            root=Path(work);speech,llm=self.fixture(root);out=root/"engines";out.mkdir()
            with self.assertRaises(FileExistsError):
                compose(platform="linux-amd64",speaches=speech,speaches_sha=sha(speech),llm_cpu=llm,llm_cpu_sha=sha(llm),output=out)

if __name__=="__main__":
    unittest.main()
