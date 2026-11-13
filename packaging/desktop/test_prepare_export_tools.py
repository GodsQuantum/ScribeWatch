"""Supply-chain and extraction checks for portable document exporters."""
from __future__ import annotations
import io
import hashlib
import tarfile
import tempfile
from pathlib import Path
import unittest
import zipfile

from prepare_export_tools import archive_file, verified_input, assemble


class ExportArchiveTests(unittest.TestCase):
    def test_exact_signed_release_executable_is_selected(self):
        with tempfile.TemporaryDirectory() as tmp:
            archive = Path(tmp) / "official.zip"
            with zipfile.ZipFile(archive, "w") as z:
                z.writestr("pandoc-3.14/bin/pandoc.exe", b"verified-pinned-binary")
                z.writestr("pandoc-3.14/man/pandoc.txt", b"documentation")
            self.assertEqual(archive_file(archive, "pandoc.exe"), b"verified-pinned-binary")

    def test_duplicate_or_traversal_archive_members_are_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            archive = Path(tmp) / "unsafe.zip"
            with zipfile.ZipFile(archive, "w") as z:
                z.writestr("../pandoc", b"malicious")
            with self.assertRaisesRegex(ValueError, "Unsafe archive member"):
                archive_file(archive, "pandoc")
            with zipfile.ZipFile(archive, "w") as z:
                z.writestr("pandoc-3.14/bin/pandoc", b"first")
                z.writestr("pandoc-3.14/tools/pandoc", b"second")
            with self.assertRaisesRegex(ValueError, "Expected exactly one"):
                archive_file(archive, "pandoc")

    def test_tar_symlink_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            archive = Path(tmp) / "unsafe.tar.xz"
            with tarfile.open(archive, mode="w:xz") as tar:
                info = tarfile.TarInfo("typst-v0.15.1/typst")
                info.type = tarfile.SYMTYPE
                info.linkname = "/etc/passwd"
                tar.addfile(info)
            with self.assertRaisesRegex(ValueError, "Invalid native binary"):
                archive_file(archive, "typst")

    def test_wrong_or_modified_sha_is_never_accepted(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp)
            (cache / "download.zip").write_bytes(b"tampered archive")
            with self.assertRaisesRegex(ValueError, "SHA256 mismatch"):
                verified_input(cache, "download.zip", "https://invalid.example.test/x",
                               hashlib.sha256(b"trusted archive").hexdigest())

    def test_existing_installation_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            output = root / "tools"
            output.mkdir()
            (output / "my-settings").write_text("preserve")
            with self.assertRaises(FileExistsError):
                assemble("linux-amd64", output, root / "cache")
            self.assertEqual((output / "my-settings").read_text(), "preserve")


if __name__ == "__main__":
    unittest.main()
