import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location(
    "prepare_release", Path(__file__).with_name("prepare-release.py")
)
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def test_merge_bumps_patch_and_skips_already_published_versions(self):
        self.assertEqual(release.next_version("0.1.0", "Merge feature", set()), "0.1.1")
        self.assertEqual(
            release.next_version("0.1.0", "Merge feature", {"0.1.1", "0.1.2"}), "0.1.3"
        )

    def test_retry_reuses_release_version_before_or_after_upload(self):
        for published in (set(), {"0.1.1"}):
            self.assertEqual(
                release.next_version("0.1.1", "chore(release): v0.1.1", published), "0.1.1"
            )

    def test_changelog_preserves_notes_and_previous_releases(self):
        self.assertEqual(
            release.release_changelog("## Unreleased\n\n- New\n\n## 0.1.0\n", "0.1.1", "2026-10-22"),
            "## Unreleased\n\n## 0.1.1 - 2026-10-22\n\n- New\n\n## 0.1.0\n",
        )

    def test_missing_changelog_section_stops_release(self):
        with self.assertRaises(ValueError):
            release.release_changelog("# Changelog\n", "0.1.1", "2026-10-22")


if __name__ == "__main__":
    unittest.main()
