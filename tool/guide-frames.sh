#!/usr/bin/env sh
# Writes the user guide's frames (#134): every guide screen of the phone
# and the desktop, drawn from the screenshot generators' seeded data, in
# English and German, light and dark, under site/assets/frames/. The frames
# are committed; CI renders them again and fails when one no longer
# matches its screen. Run this after a screen changes and commit the
# frames with the change.
#
#   tool/guide-frames.sh           write every frame
#   tool/guide-frames.sh --check   compare only, as CI does
#
# The desktop frames are drawn by Mesa's software Vulkan, lavapipe, as on
# the CI runner (package mesa-vulkan-drivers on Debian, Ubuntu and Fedora):
# egui_kittest prefers a CPU adapter over a GPU, and Vulkan only keeps
# Mesa's GL one (llvmpipe) out. FRAMES_JOBS caps the Rust build (default 1).
set -eu
jobs="${FRAMES_JOBS:-1}"
cd "$(dirname "$0")/.."
export WGPU_BACKEND="${WGPU_BACKEND:-vulkan}"
if [ -z "${VK_DRIVER_FILES:-}${VK_ICD_FILENAMES:-}" ] &&
  ! ls /usr/share/vulkan/icd.d/lvp_icd*.json >/dev/null 2>&1; then
  echo "warning: no lavapipe found; a GPU draws the desktop frames and CI may differ" >&2
fi
if [ "${1:-}" = "--check" ]; then
  unset UPDATE_FRAMES
else
  export UPDATE_FRAMES=1
fi
echo "phone frames (en, de; light, dark)…"
nice -n 19 flutter test test/screenshots/screenshots_test.dart --run-skipped \
  --concurrency=1 --plain-name 'guide frames'
echo "desktop frames (en, de; light, dark)…"
(cd desktop && nice -n 19 env CARGO_BUILD_JOBS="$jobs" \
  cargo test -j "$jobs" -p oldbrainfix-gui --test screenshots --locked \
  -- --ignored guide_frames)
echo "done: site/assets/frames"
