"""Unit tests for generate_third_party_licenses.py.

Run with: python3 -m unittest discover -s packaging/licenses
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path
from typing import Any

import generate_third_party_licenses as gen


def package(name: str, root: Path, *, third_party: bool = True) -> dict[str, Any]:
    """Build a minimal cargo-about package entry."""
    return {
        "name": name,
        "version": "1.0.0",
        "source": "registry+https://github.com/rust-lang/crates.io-index" if third_party else None,
        "manifest_path": str(root / "Cargo.toml"),
    }


def about_output(*packages: dict[str, Any], license_text: str = "MIT text") -> dict[str, Any]:
    """Build a minimal cargo-about JSON document with one license used by all packages."""
    return {
        "licenses": [
            {"name": "MIT License", "text": license_text, "used_by": [{"crate": p} for p in packages]},
        ],
        "crates": [{"package": p} for p in packages],
    }


class TempDirTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp = Path(self._tmp.name)

    def tearDown(self) -> None:
        self._tmp.cleanup()

    def make_crate(self, name: str, files: dict[str, str] | None = None) -> Path:
        root = self.tmp / name
        root.mkdir()
        for relative, content in (files or {}).items():
            path = root / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        return root


class LicenseBlocksTest(TempDirTestCase):
    def test_workspace_crates_are_excluded(self) -> None:
        about = about_output(
            package("dep", self.tmp),
            package("fotobuch", self.tmp, third_party=False),
        )

        blocks = gen.license_blocks(about)

        self.assertEqual(blocks[0].crates, ("dep 1.0.0",))

    def test_license_used_only_by_workspace_is_dropped(self) -> None:
        about = about_output(package("fotobuch", self.tmp, third_party=False))

        self.assertEqual(gen.license_blocks(about), [])

    def test_text_is_normalized(self) -> None:
        about = about_output(package("dep", self.tmp), license_text="\r\nline 1\r\nline 2\r\n")

        self.assertEqual(gen.license_blocks(about)[0].text, "line 1\nline 2")


class NoticeDiscoveryTest(TempDirTestCase):
    def test_finds_notice_like_files_only(self) -> None:
        root = self.make_crate(
            "dep",
            {"NOTICE": "n", "LICENSE-THIRD-PARTY": "t", "notices.md": "m", "LICENSE-MIT": "x", "src/NOTICE": "nested"},
        )

        names = [path.name for path in gen.find_notice_files(root)]

        self.assertEqual(names, ["LICENSE-THIRD-PARTY", "NOTICE", "notices.md"])

    def test_discovered_notice_is_titled_with_crate_and_file(self) -> None:
        root = self.make_crate("dep", {"NOTICE": "notice text\n"})
        crate = gen.Crate("dep", "1.0.0", root)

        self.assertEqual(gen.discovered_notices([crate]), [gen.Notice("dep 1.0.0: NOTICE", "notice text")])


class ConfiguredNoticesTest(TempDirTestCase):
    def test_crate_file_is_read(self) -> None:
        root = self.make_crate("fonts", {"fonts/OFL.txt": "ofl"})
        crates = [gen.Crate("fonts", "2.0.0", root)]

        notices = gen.configured_crate_notices([{"crate": "fonts", "paths": ["fonts/OFL.txt"]}], crates)

        self.assertEqual(notices, [gen.Notice("fonts 2.0.0: fonts/OFL.txt", "ofl")])

    def test_missing_crate_fails(self) -> None:
        with self.assertRaisesRegex(gen.GenerationError, "not in the dependency graph"):
            gen.configured_crate_notices([{"crate": "gone", "paths": ["x"]}], [])

    def test_missing_crate_file_fails(self) -> None:
        crates = [gen.Crate("fonts", "2.0.0", self.make_crate("fonts"))]

        with self.assertRaisesRegex(gen.GenerationError, "does not exist"):
            gen.configured_crate_notices([{"crate": "fonts", "paths": ["missing.txt"]}], crates)

    def test_repo_file_is_read(self) -> None:
        (self.tmp / "LICENSE.txt").write_text("asset license", encoding="utf-8")

        notices = gen.configured_file_notices([{"title": "Asset", "path": "LICENSE.txt"}], self.tmp)

        self.assertEqual(notices, [gen.Notice("Asset", "asset license")])

    def test_missing_repo_file_fails(self) -> None:
        with self.assertRaisesRegex(gen.GenerationError, "does not exist"):
            gen.configured_file_notices([{"title": "Asset", "path": "missing.txt"}], self.tmp)


class GenerateTest(TempDirTestCase):
    def test_output_contains_licenses_and_all_notice_kinds(self) -> None:
        root = self.make_crate("dep", {"NOTICE": "dep notice", "assets/LICENSE.txt": "asset of dep"})
        (self.tmp / "own.txt").write_text("own asset", encoding="utf-8")
        config = {
            "crate_files": [{"crate": "dep", "paths": ["assets/LICENSE.txt"]}],
            "files": [{"title": "Own asset", "path": "own.txt"}],
        }

        output = gen.generate(about_output(package("dep", root)), config, self.tmp)

        self.assertTrue(output.startswith("THIRD-PARTY LICENSES"))
        for expected in ("MIT text", "Used by: dep 1.0.0", "dep notice", "asset of dep", "Own asset", "own asset"):
            self.assertIn(expected, output)
        self.assertLess(output.index("PART 1"), output.index("PART 2"))


if __name__ == "__main__":
    unittest.main()
