#!/usr/bin/env bash
# Coverage floors (#232): runs the tests with coverage, writes the LCOV
# files SonarQube Cloud imports (sonar-project.properties), and checks each
# group's line coverage against tool/coverage-floors.txt with
# tool/coverage_floor.py. Below a floor fails; a floor that rose is raised
# in that file, for the user to commit.
#
#   tool/coverage.sh [rust-core] [rust-gui] [dart-core] [flutter-app]
#
# No group named runs all four. A failing test stops the script; a group
# below its floor does not stop the others, so every LCOV file is written.
#
#   rust-core    cargo llvm-cov, the core crate's own tests, its src/ only
#                (with oldbrainfix-l10n, as its tests are always built)
#   rust-gui     cargo llvm-cov, the GUI crate's own tests, its src/ only;
#                needs the GUI test libraries (ALSA, a Vulkan driver)
#   dart-core    dart test --coverage, format_coverage to LCOV with paths
#                from the repository root (packages/oldbrainfix_core/lib/…)
#   flutter-app  flutter test --coverage, lib/ without the generated
#                lib/l10n/app_localizations*.dart
#
# Output: desktop/coverage/lcov.info (both Rust groups),
# packages/oldbrainfix_core/coverage/lcov.info and coverage/lcov.info.
#
# Needs cargo-llvm-cov and the llvm-tools-preview rustup component for the
# Rust groups. COVERAGE_JOBS=N limits the test and build parallelism (for a
# shared machine); unset leaves each tool's default.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

groups=("$@")
[ ${#groups[@]} -gt 0 ] || groups=(rust-core rust-gui dart-core flutter-app)

jobs=${COVERAGE_JOBS:-}
failed=()

floor() {
  python3 tool/coverage_floor.py "$1" "$2" || failed+=("$1")
}

rust() {
  local group=$1
  shift
  # The core's and the GUI's numbers come from their own tests only; the
  # floor tool keeps each crate's src/ (generated code is in target/).
  # cargo-llvm-cov leaves out files named tests.rs by default; the tests
  # live in such files (so that CodeQL can skip them) and count here as
  # they did inline, which the floors were set on.
  (cd desktop && cargo llvm-cov --locked ${jobs:+-j "$jobs"} "$@" \
    --no-default-ignore-filename-regex \
    --lcov --output-path "coverage/$group.lcov")
  floor "$group" "desktop/coverage/$group.lcov"
}

mkdir -p desktop/coverage
for group in "${groups[@]}"; do
  case $group in
    rust-core)
      rust rust-core -p oldbrainfix-core -p oldbrainfix-l10n
      ;;
    rust-gui)
      rust rust-gui -p oldbrainfix-gui
      ;;
    dart-core)
      (
        cd packages/oldbrainfix_core
        rm -rf coverage
        dart test ${jobs:+-j "$jobs"} --coverage=coverage
        dart run coverage:format_coverage --lcov \
          --in=coverage --out=coverage/lcov.info \
          --report-on=lib --base-directory=../..
      )
      floor dart-core packages/oldbrainfix_core/coverage/lcov.info
      ;;
    flutter-app)
      rm -rf coverage
      flutter test ${jobs:+--concurrency="$jobs"} --coverage
      floor flutter-app coverage/lcov.info
      ;;
    *)
      echo "unknown group: $group" >&2
      exit 2
      ;;
  esac
done

# SonarQube Cloud reads one Rust LCOV file.
if ls desktop/coverage/*.lcov > /dev/null 2>&1; then
  cat desktop/coverage/*.lcov > desktop/coverage/lcov.info
fi

if [ ${#failed[@]} -gt 0 ]; then
  echo "below its coverage floor: ${failed[*]}" >&2
  exit 1
fi
