#!/usr/bin/env python3
"""
Build cover modes for my-book example.
Iterates through all cover modes, updates the YAML config, runs fotobuch build, and compiles with typst.
"""

import subprocess
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    print(
        "❌ PyYAML not installed. Install it with: pip install pyyaml", file=sys.stderr
    )
    sys.exit(1)

# All cover modes from CoverMode enum
COVER_MODES = [
    "free",
    "front",
    "front-full",
    "back",
    "back-full",
    "spread",
    "spread-full",
    "split",
    "split-full",
]

# Path to the example book directory
EXAMPLE_DIR = Path(__file__).parent.parent / "examples" / "my-fotobuch"
YAML_FILE = EXAMPLE_DIR / "my-fotobuch.yaml"


def run_command(cmd, cwd=None):
    """Run a command and return success status."""
    print(f"  Running: {' '.join(cmd)}")
    try:
        subprocess.run(cmd, cwd=cwd, check=True)
        return True
    except subprocess.CalledProcessError as e:
        print(f"  ❌ Command failed: {e}", file=sys.stderr)
        return False


def update_cover_mode_in_yaml(mode):
    """Update the cover mode in my-fotobuch.yaml."""
    try:
        with open(YAML_FILE, "r") as f:
            config = yaml.safe_load(f)

        config["config"]["book"]["cover"]["mode"] = mode

        with open(YAML_FILE, "w") as f:
            yaml.dump(config, f, default_flow_style=False, sort_keys=False)

        return True
    except Exception as e:
        print(f"  ❌ Failed to update YAML: {e}", file=sys.stderr)
        return False


def build_cover_mode(mode):
    """Build and compile a single cover mode."""
    print(f"\n📖 Processing cover mode: {mode}")

    # Update YAML with new cover mode
    if not update_cover_mode_in_yaml(mode):
        return False

    # Run fotobuch build
    if not run_command(["fotobuch", "build"], cwd=EXAMPLE_DIR):
        return False

    # Run typst compile
    if not run_command(
        ["typst", "compile", "my-fotobuch.typ", "--pages", "1", f"{mode}.svg"],
        cwd=EXAMPLE_DIR,
    ):
        return False

    print(f"  ✅ {mode} completed")
    return True


def main():
    """Main entry point."""
    print(f"🏗️  Building cover modes from {EXAMPLE_DIR}")
    print(f"Modes: {', '.join(COVER_MODES)}\n")

    failed = []

    for mode in COVER_MODES:
        if not build_cover_mode(mode):
            failed.append(mode)

    print("\n" + "=" * 50)
    if failed:
        print(f"❌ Failed modes: {', '.join(failed)}")
        return 1
    else:
        print("✅ All cover modes completed successfully!")
        return 0


if __name__ == "__main__":
    sys.exit(main())
