"""Ensure tag releases deliver every native desktop artifact, not just slim builds."""
from pathlib import Path
import unittest

ROOT=Path(__file__).resolve().parents[2]
RELEASE=(ROOT/'.github/workflows/release.yml').read_text(encoding='utf-8')
PORTABLE=(ROOT/'.github/workflows/native-full-portable.yml').read_text(encoding='utf-8')

class ReleaseNativeArtifactContract(unittest.TestCase):
    def test_full_native_workflow_is_reusable_by_tag_release(self):
        self.assertIn('workflow_call:',PORTABLE)
        self.assertIn('  full-native:',RELEASE)
        self.assertIn('uses: ./.github/workflows/native-full-portable.yml',RELEASE)
        self.assertIn('needs: [publish, appimage, windows-standalone, full-native]',RELEASE)

    def test_release_requires_and_checks_both_linux_architectures_and_macos(self):
        self.assertIn('pattern: scribewatch-full-linux-*',RELEASE)
        self.assertIn('pattern: scribewatch-full-macos-*',RELEASE)
        self.assertIn('dist/full/*-standalone.dmg',RELEASE)
        self.assertIn('dist/full/*.deb',RELEASE)
        self.assertIn('dist/full/*-standalone.AppImage',RELEASE)
        self.assertIn('sha256sum -c',RELEASE)
        # A failed native installer must not move the Docker :latest tag.
        self.assertIn('needs: [image, full-native, windows-standalone, appimage]',RELEASE)

if __name__=='__main__':
    unittest.main()
