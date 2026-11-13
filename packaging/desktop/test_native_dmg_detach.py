import contextlib
import io
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0,str(Path(__file__).resolve().parent))
with patch.object(sys, "platform", "darwin"):
    from native_pipeline import detach_owned_test_dmg


class MacDmgDetachTests(unittest.TestCase):
    def test_retries_mac_resource_busy_before_success(self):
        with tempfile.TemporaryDirectory() as root:
            mount=Path(root)/"trio-dmg-mount-probe"
            mount.mkdir()
            busy=subprocess.CompletedProcess([],16)
            good=subprocess.CompletedProcess([],0)
            with patch("native_pipeline.subprocess.run",side_effect=[busy,busy,good]) as command, \
                 patch("native_pipeline.time.sleep") as sleeper, \
                 contextlib.redirect_stdout(io.StringIO()):
                detach_owned_test_dmg(mount)
            self.assertEqual(command.call_count,3)
            self.assertEqual(sleeper.call_count,2)
            self.assertNotIn("-force",command.call_args_list[-1].args[0])

    def test_only_forces_own_disposable_test_mount_after_all_retries(self):
        with tempfile.TemporaryDirectory() as root:
            mount=Path(root)/"trio-dmg-mount-owned"
            mount.mkdir()
            with patch("native_pipeline.subprocess.run",return_value=subprocess.CompletedProcess([],16)) as command, \
                 patch("native_pipeline.time.sleep"), \
                 patch("native_pipeline.run") as run, \
                 contextlib.redirect_stdout(io.StringIO()):
                detach_owned_test_dmg(mount)
            self.assertEqual(command.call_count,8)
            run.assert_called_once_with("hdiutil","detach","-force","-quiet",mount)

    def test_never_touches_an_unowned_mount(self):
        with tempfile.TemporaryDirectory() as root:
            mount=Path(root)/"patients"
            mount.mkdir()
            with patch("native_pipeline.subprocess.run") as command:
                with self.assertRaisesRegex(ValueError,"Refusing"):
                    detach_owned_test_dmg(mount)
            command.assert_not_called()


if __name__=="__main__":
    unittest.main()
