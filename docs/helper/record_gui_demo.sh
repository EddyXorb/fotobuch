#!/usr/bin/env bash
#
# Helper to record a clean GUI demo for the README hero GIF.
#
# Usage:
#   docs/helper/record_gui_demo.sh prepare   # build a small grouped demo dataset + fresh vault
#   docs/helper/record_gui_demo.sh run       # launch the GUI on the demo vault
#
# Recording (X11) with peek:
#   1. Run `prepare`, then `run`.
#   2. Open peek, drag its frame over the fotobuch-gui window.
#   3. Hit "Record as GIF", then drag the demo group folders from your
#      file manager into the GUI window and let the solver build.
#   4. Stop peek; it saves an optimized .gif. Shrink it further with:
#        docs/helper/optimize_gif.sh recording.gif docs/examples/gui-demo.gif
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="$ROOT/docs/examples/source-images"
DEMO_SRC="$ROOT/target/demo-source"
VAULT="$ROOT/target/demo-vault"
GUI_BIN="$ROOT/target/release/fotobuch-gui"

# Group folders are timestamp-prefixed: folder names set chronological order.
declare -a DEMO_GROUPS=("2024-07-Italy" "2024-08-Hiking" "2024-09-Autumn")
PER_GROUP=6

prepare() {
  command -v "$GUI_BIN" >/dev/null 2>&1 || [ -x "$GUI_BIN" ] || {
    echo "GUI binary missing. Build it first:"
    echo "  cargo build --release --features gui --bin fotobuch-gui"
    exit 1
  }

  rm -rf "$DEMO_SRC" "$VAULT"
  mkdir -p "$DEMO_SRC" "$VAULT"

  mapfile -t IMAGES < <(find "$SRC" -maxdepth 1 -type f \
    \( -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.png' \) | sort)

  if [ "${#IMAGES[@]}" -lt "$(( ${#DEMO_GROUPS[@]} * PER_GROUP ))" ]; then
    echo "Not enough demo images in $SRC" >&2
    exit 1
  fi

  local idx=0
  for g in "${DEMO_GROUPS[@]}"; do
    mkdir -p "$DEMO_SRC/$g"
    for ((i = 0; i < PER_GROUP; i++)); do
      cp "${IMAGES[$idx]}" "$DEMO_SRC/$g/"
      idx=$((idx + 1))
    done
  done

  echo "Demo source groups ready at: $DEMO_SRC"
  ls -1 "$DEMO_SRC"
  echo
  echo "Fresh demo vault: $VAULT"
  echo "Next: docs/helper/record_gui_demo.sh run"
}

run() {
  [ -x "$GUI_BIN" ] || {
    echo "GUI not built. Run: cargo build --release --features gui --bin fotobuch-gui" >&2
    exit 1
  }
  echo "Launching GUI on demo vault. Drag these folders into the window:"
  ls -1d "$DEMO_SRC"/*/ 2>/dev/null || echo "  (run 'prepare' first)"
  FOTOBUCH_VAULT="$VAULT" "$GUI_BIN" --vault "$VAULT"
}

case "${1:-}" in
  prepare) prepare ;;
  run) run ;;
  *)
    echo "Usage: $0 {prepare|run}" >&2
    exit 1
    ;;
esac
