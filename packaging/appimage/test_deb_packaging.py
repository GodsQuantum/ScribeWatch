from __future__ import annotations
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("build-deb.sh")

class DebPackagingTests(unittest.TestCase):
    def test_deb_contains_launcher_engines_and_desktop_integration(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            app=root/"ScribeWatch.AppDir"
            (app/"usr/bin").mkdir(parents=True)
            (app/"usr/share/scribewatch/engines/llm").mkdir(parents=True)
            (app/"usr/share/scribewatch/engines/speaches").mkdir(parents=True)
            (app/"usr/share/applications").mkdir(parents=True)
            (app/"usr/share/icons/hicolor/scalable/apps").mkdir(parents=True)
            (app/"AppRun").write_text("#!/bin/sh\nexec /bin/true\n")
            (app/"AppRun").chmod(0o755)
            (app/"usr/bin/scribewatch").write_text("mock-binary")
            (app/"usr/share/scribewatch/engines/llm/llama-server").write_text("llm")
            (app/"usr/share/scribewatch/engines/speaches/Cloud9-Speaches").write_text("stt")
            (app/"usr/share/scribewatch/engines/BUNDLED-RUNTIMES-SHA256.json").write_text("{}")
            (app/"usr/share/applications/io.github.GodsQuantum.ScribeWatch.desktop").write_text("[Desktop Entry]\nName=ScribeWatch\n")
            (app/"usr/share/icons/hicolor/scalable/apps/scribewatch.svg").write_text("<svg/>")
            out=root/"dist";out.mkdir()
            proc=subprocess.run(["/bin/sh",str(SCRIPT),str(app),str(out),"0.6.2","amd64"],capture_output=True,text=True,timeout=45)
            self.assertEqual(proc.returncode,0,proc.stdout+proc.stderr)
            artifact=out/"scribewatch_0.6.2_amd64.deb"
            self.assertTrue(artifact.is_file())
            meta=subprocess.check_output(["dpkg-deb","--field",str(artifact)],text=True)
            self.assertIn("Architecture: amd64",meta)
            self.assertIn("Package: scribewatch",meta)
            listing=subprocess.check_output(["dpkg-deb","--contents",str(artifact)],text=True)
            for name in ["opt/scribewatch/AppRun","opt/scribewatch/usr/share/scribewatch/engines/llm/llama-server","usr/bin/scribewatch","usr/share/applications/io.github.GodsQuantum.ScribeWatch.desktop"]:
                self.assertIn(name,listing)
            self.assertFalse("postinst" in subprocess.check_output(["dpkg-deb","--ctrl-tarfile",str(artifact)]).decode("latin1","ignore"))

    def test_missing_bundled_engine_rejects_release(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);app=root/"AppDir";app.mkdir();out=root/"dist";out.mkdir()
            result=subprocess.run(["/bin/sh",str(SCRIPT),str(app),str(out),"0.6.2","amd64"],capture_output=True,text=True,timeout=8)
            self.assertNotEqual(result.returncode,0)
            self.assertEqual(list(out.glob("*.deb")),[])

if __name__=="__main__":
    unittest.main()
