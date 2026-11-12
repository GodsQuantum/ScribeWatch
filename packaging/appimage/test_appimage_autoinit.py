"""AppImage first-boot contract, without a GPU or model download."""
from __future__ import annotations
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

APP_RUN = Path(__file__).with_name("AppRun")

class AppImageAutoinitTests(unittest.TestCase):
    def call(self, bundled, overrides=None):
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            app=root/"bundle"
            bin_dir=app/"usr/bin"
            bin_dir.mkdir(parents=True)
            backend=bin_dir/"scribewatch"
            backend.write_text('#!/bin/sh\nprintf "DESKTOP=%s AUTOINIT=%s EMBEDDED=%s\\n" "$SCRIBEWATCH_DESKTOP_MODE" "$SCRIBEWATCH_NATIVE_AUTOINIT" "$SCRIBEWATCH_EMBEDDED_DIR"\n')
            backend.chmod(0o755)
            if bundled:
                engines=app/"usr/share/scribewatch/engines"
                engines.mkdir(parents=True)
                (engines/"BUNDLED-RUNTIMES-SHA256.json").write_text('{"files":{}}')
            env={"PATH":"/usr/bin:/bin","HOME":str(root),"APPDIR":str(app),**(overrides or {})}
            result=subprocess.run(["/bin/sh",str(APP_RUN),"--version"],env=env,capture_output=True,text=True,timeout=8,check=True)
            return result.stdout.strip()

    def test_full_bundle_bootstraps_local_models(self):
        result=self.call(True)
        self.assertIn("DESKTOP=1",result)
        self.assertIn("AUTOINIT=1",result)
        self.assertIn("/usr/share/scribewatch/engines",result)

    def test_slim_appimage_never_promises_bundled_engines(self):
        result=self.call(False)
        self.assertNotIn("AUTOINIT=1",result)
        self.assertNotIn("DESKTOP=1",result)

    def test_operator_can_disable_download_on_full_bundle(self):
        result=self.call(True, {"SCRIBEWATCH_NATIVE_AUTOINIT":"0"})
        self.assertIn("AUTOINIT=0",result)

if __name__=="__main__":
    unittest.main()
