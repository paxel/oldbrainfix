#!/usr/bin/env sh
# Builds the iPhone app's native libraries (#239) into
# ios/Native/<sdk>/, where the Runner's project links them from:
#
#   liboldbrainfix_ocr.a  RustO for Photo Texts, the crate android/ocr
#                         as a static library (its C functions are the
#                         iPhone's; the JNI ones are Android's)
#   libobf_whisper.a, libwhisper.a, libggml*.a
#                         whisper.cpp for Transcripts, from the submodule
#                         third_party/whisper.cpp with the desktop's C
#                         bridge, desktop/crates/oldbrainfix-whisper/native,
#                         CPU only as on Android and the desktop
#
# for a device (iphoneos, aarch64-apple-ios) and the simulator
# (iphonesimulator, aarch64-apple-ios-sim), both arm64 only. Run it on a
# Mac before `flutter build ios` or `xcodebuild`; CI does (ios.yml,
# release.yml). Needs Xcode, cmake, and Rust with both iOS targets
# (`rustup target add aarch64-apple-ios aarch64-apple-ios-sim`).
#
#   tool/ios-native.sh               both SDKs
#   tool/ios-native.sh iphoneos      only the device's
#
# IOS_NATIVE_JOBS caps the C++ build (default: the Mac's cores).
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
whisper="$root/third_party/whisper.cpp"
if [ ! -f "$whisper/include/whisper.h" ]; then
    echo "whisper.cpp is missing: git submodule update --init" >&2
    exit 1
fi
# The app's oldest iOS, as in ios/Runner.xcodeproj.
export IPHONEOS_DEPLOYMENT_TARGET=26.0
jobs=${IOS_NATIVE_JOBS:-$(sysctl -n hw.ncpu)}

build() {
    sdk=$1
    triple=$2
    out="$root/ios/Native/$sdk"
    mkdir -p "$out"

    # A static library for this build only: Android's cargo-ndk build
    # keeps the crate's own cdylib.
    cargo rustc --locked --release --lib \
        --manifest-path "$root/android/ocr/Cargo.toml" \
        --target "$triple" --crate-type staticlib
    cp "$root/android/ocr/target/$triple/release/liboldbrainfix_ocr.a" "$out/"

    cmake_dir="$root/build/ios-whisper/$sdk"
    cmake -S "$root/desktop/crates/oldbrainfix-whisper/native" -B "$cmake_dir" \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_SYSTEM_NAME=iOS \
        -DCMAKE_SYSTEM_PROCESSOR=aarch64 \
        -DCMAKE_OSX_SYSROOT="$sdk" \
        -DCMAKE_OSX_ARCHITECTURES=arm64 \
        -DCMAKE_OSX_DEPLOYMENT_TARGET="$IPHONEOS_DEPLOYMENT_TARGET" \
        -DCMAKE_TRY_COMPILE_TARGET_TYPE=STATIC_LIBRARY \
        -DWHISPER_DIR="$whisper"
    cmake --build "$cmake_dir" --target obf_whisper --config Release -j "$jobs"
    for name in obf_whisper whisper ggml ggml-cpu ggml-base; do
        cp "$cmake_dir/lib/lib$name.a" "$out/"
    done
}

sdks=${1:-"iphoneos iphonesimulator"}
for sdk in $sdks; do
    case $sdk in
        iphoneos) build iphoneos aarch64-apple-ios ;;
        iphonesimulator) build iphonesimulator aarch64-apple-ios-sim ;;
        *)
            echo "unknown SDK $sdk: iphoneos or iphonesimulator" >&2
            exit 1
            ;;
    esac
done
