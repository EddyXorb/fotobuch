#!/usr/bin/env python3
"""Generate the third-party license notice file for fotobuch's binaries.

Combines three sources into a single plain-text file:

1. License texts of all dependency crates, collected by ``cargo about``
   (configured in ``about.toml``).
2. NOTICE / THIRD-PARTY files shipped at the root of dependency crates.
3. Notices for embedded assets listed in ``packaging/licenses/notices.toml``.

Usage::

    packaging/licenses/generate_third_party_licenses.py -o THIRD-PARTY-LICENSES.txt

Requires ``cargo-about`` on ``PATH``. Exits non-zero if a license cannot be
resolved or a configured notice is missing.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tomllib
from collections.abc import Iterable, Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_CONFIG = REPO_ROOT / "packaging" / "licenses" / "notices.toml"
DEFAULT_FEATURES = "gui"

# Upper-case file name prefixes of NOTICE-like files in a crate root.
NOTICE_FILE_PREFIXES = ("NOTICE", "THIRD-PARTY", "THIRD_PARTY", "LICENSE-THIRD-PARTY")

SECTION_RULE = "=" * 80
ENTRY_RULE = "-" * 80

HEADER = """\
THIRD-PARTY LICENSES

fotobuch is licensed under the GNU Affero General Public License v3.0 or later
(see LICENSE). It contains third-party software and assets that are subject to
the licenses and notices reproduced below.
"""


class GenerationError(Exception):
    """Raised when the notice file cannot be generated completely."""


@dataclass(frozen=True)
class Crate:
    """A third-party crate in the dependency graph."""

    name: str
    version: str
    root: Path

    @property
    def label(self) -> str:
        return f"{self.name} {self.version}"


@dataclass(frozen=True)
class LicenseBlock:
    """One license text together with the crates it applies to."""

    name: str
    text: str
    crates: tuple[str, ...]


@dataclass(frozen=True)
class Notice:
    """An additional notice that is reproduced verbatim."""

    title: str
    text: str


def run_cargo_about(features: str) -> dict[str, Any]:
    """Run ``cargo about`` and return its JSON output."""
    command = [
        "cargo",
        "about",
        "generate",
        "--format",
        "json",
        "--fail",
        "--locked",
        "--features",
        features,
        "--manifest-path",
        str(REPO_ROOT / "Cargo.toml"),
    ]
    result = subprocess.run(command, check=True, capture_output=True, text=True)
    return json.loads(result.stdout)


def is_third_party(package: dict[str, Any]) -> bool:
    """Workspace members have no ``source``; everything else is third-party."""
    return package.get("source") is not None


def normalize_text(text: str) -> str:
    """Unify line endings and strip surrounding blank lines."""
    return text.replace("\r\n", "\n").strip("\n")


def license_blocks(about: dict[str, Any]) -> list[LicenseBlock]:
    """Extract license texts used by third-party crates."""
    blocks: list[LicenseBlock] = []
    for entry in about["licenses"]:
        crates = tuple(
            f"{used['crate']['name']} {used['crate']['version']}"
            for used in entry["used_by"]
            if is_third_party(used["crate"])
        )
        if crates:
            blocks.append(LicenseBlock(entry["name"], normalize_text(entry["text"]), crates))
    return blocks


def third_party_crates(about: dict[str, Any]) -> list[Crate]:
    """List all third-party crates that cargo-about included."""
    return [
        Crate(
            name=package["name"],
            version=package["version"],
            root=Path(package["manifest_path"]).parent,
        )
        for package in (entry["package"] for entry in about["crates"])
        if is_third_party(package)
    ]


def find_notice_files(crate_root: Path) -> list[Path]:
    """Return NOTICE-like files located directly in ``crate_root``."""
    return sorted(
        path for path in crate_root.iterdir() if path.is_file() and path.name.upper().startswith(NOTICE_FILE_PREFIXES)
    )


def read_text(path: Path) -> str:
    return normalize_text(path.read_text(encoding="utf-8", errors="replace"))


def discovered_notices(crates: Iterable[Crate]) -> list[Notice]:
    """Collect NOTICE-like files shipped by the crates themselves."""
    return [
        Notice(f"{crate.label}: {path.name}", read_text(path))
        for crate in crates
        for path in find_notice_files(crate.root)
    ]


def configured_crate_notices(entries: Iterable[dict[str, Any]], crates: Sequence[Crate]) -> list[Notice]:
    """Resolve ``[[crate_files]]`` entries against the dependency graph."""
    notices: list[Notice] = []
    for entry in entries:
        matches = [crate for crate in crates if crate.name == entry["crate"]]
        if not matches:
            raise GenerationError(f"crate '{entry['crate']}' from notices.toml is not in the dependency graph")
        for crate in matches:
            for relative in entry["paths"]:
                path = crate.root / relative
                if not path.is_file():
                    raise GenerationError(f"{crate.label}: configured notice file '{relative}' does not exist")
                notices.append(Notice(f"{crate.label}: {relative}", read_text(path)))
    return notices


def configured_file_notices(entries: Iterable[dict[str, Any]], base_dir: Path) -> list[Notice]:
    """Resolve ``[[files]]`` entries relative to ``base_dir``."""
    notices: list[Notice] = []
    for entry in entries:
        path = base_dir / entry["path"]
        if not path.is_file():
            raise GenerationError(f"configured notice file '{entry['path']}' does not exist")
        notices.append(Notice(entry["title"], read_text(path)))
    return notices


def load_config(path: Path) -> dict[str, Any]:
    with path.open("rb") as file:
        return tomllib.load(file)


def render(blocks: Sequence[LicenseBlock], notices: Sequence[Notice]) -> str:
    """Render the complete notice file."""
    parts = [HEADER, SECTION_RULE, "PART 1: LICENSES OF RUST DEPENDENCIES", SECTION_RULE, ""]
    for block in blocks:
        parts += [ENTRY_RULE, block.name, "Used by: " + ", ".join(block.crates), ENTRY_RULE, "", block.text, ""]
    parts += [SECTION_RULE, "PART 2: ADDITIONAL NOTICES (EMBEDDED ASSETS AND NOTICE FILES)", SECTION_RULE, ""]
    for notice in notices:
        parts += [ENTRY_RULE, notice.title, ENTRY_RULE, "", notice.text, ""]
    return "\n".join(parts)


def generate(about: dict[str, Any], config: dict[str, Any], repo_root: Path) -> str:
    """Build the notice file from cargo-about output and the notice config."""
    crates = third_party_crates(about)
    notices = (
        discovered_notices(crates)
        + configured_crate_notices(config.get("crate_files", []), crates)
        + configured_file_notices(config.get("files", []), repo_root)
    )
    return render(license_blocks(about), notices)


def parse_args(argv: Sequence[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("-o", "--output", type=Path, required=True, help="file to write")
    parser.add_argument("--features", default=DEFAULT_FEATURES, help="cargo features of the release build")
    parser.add_argument("--config", type=Path, default=DEFAULT_CONFIG, help="notice configuration (TOML)")
    return parser.parse_args(argv)


def main(argv: Sequence[str]) -> int:
    args = parse_args(argv)
    try:
        content = generate(run_cargo_about(args.features), load_config(args.config), REPO_ROOT)
    except subprocess.CalledProcessError as error:
        print(error.stderr, file=sys.stderr)
        return 1
    except GenerationError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    args.output.write_text(content + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
