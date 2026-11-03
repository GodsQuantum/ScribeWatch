from __future__ import annotations

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
NSIS_ACTION = "negrutiu/nsis-install@f3339c88dba6fd08910d5275a943f8f746d94876"


class InstallerDependenciesTest(unittest.TestCase):
    def test_large_standalone_uses_single_embedded_zip_and_windows_tar(self) -> None:
        installer = (ROOT / "packaging/desktop/installer.nsi").read_text()
        pipeline = (ROOT / "packaging/desktop/native_pipeline.py").read_text()
        self.assertIn("!ifdef ARCHIVE_PAYLOAD", installer)
        self.assertIn('File /oname=_ScribeWatch_payload.zip', installer)
        self.assertIn(r'"$SYSDIR\tar.exe"', installer)
        self.assertIn('nsExec::ExecToLog', installer)
        self.assertIn("Pop $0", installer)
        self.assertIn("StrCmp $0", installer)
        self.assertIn('File /r "', installer)
        self.assertIn('standalone_payload.py', pipeline)
        self.assertIn('"/DARCHIVE_PAYLOAD="+str(payload)', pipeline)

    def test_nsis_uses_pinned_action_not_chocolatey(self) -> None:
        pipeline = (ROOT / "packaging/desktop/native_pipeline.py").read_text()
        workflow = (ROOT / ".github/workflows/desktop-installer.yml").read_text()
        self.assertNotIn('run("choco"', pipeline)
        self.assertIn(NSIS_ACTION, workflow)


if __name__ == "__main__":
    unittest.main()
