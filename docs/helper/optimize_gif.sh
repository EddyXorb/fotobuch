#!/usr/bin/env bash
#
# Optimize a screen-recorded GIF (or MP4) into a small, high-quality GIF
# suitable for embedding in the README. Uses ffmpeg's two-pass palette
# workflow for good colors at a small file size.
#
# Usage:
#   docs/helper/optimize_gif.sh input.gif  output.gif  [width] [fps]
#   docs/helper/optimize_gif.sh input.mp4  output.gif  [width] [fps]
#
# Defaults: width=900px (height auto), fps=15.
#
set -euo pipefail

IN="${1:?input file required}"
OUT="${2:?output file required}"
WIDTH="${3:-1280}"
FPS="${4:-5}"

command -v ffmpeg >/dev/null || { echo "ffmpeg not found" >&2; exit 1; }

PALETTE="$(mktemp --suffix=.png)"
trap 'rm -f "$PALETTE"' EXIT

FILTERS="fps=${FPS},scale=${WIDTH}:-1:flags=lanczos"

ffmpeg -y -i "$IN" -vf "${FILTERS},palettegen=stats_mode=diff" "$PALETTE"
ffmpeg -y -i "$IN" -i "$PALETTE" \
  -lavfi "${FILTERS},paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle" \
  "$OUT"

echo "Wrote $OUT"
du -h "$OUT" | cut -f1 | xargs echo "Size:"
