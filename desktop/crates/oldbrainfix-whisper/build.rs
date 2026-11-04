//! Builds the whisper.cpp and ggml sources of the submodule
//! `third_party/whisper.cpp`, which the phone builds too, unchanged,
//! with the C bridge in `native/`, and links them in. Always optimised:
//! an unoptimised ggml is far too slow to transcribe anything. Needs
//! CMake and a C++ compiler.

use std::path::Path;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let whisper = manifest.join("../../../third_party/whisper.cpp");
    let native = manifest.join("native");
    println!("cargo:rerun-if-changed={}", native.display());
    // The parts built, not the submodule's root: its `.git` changes
    // without the sources changing.
    for part in ["CMakeLists.txt", "include", "src", "ggml"] {
        println!("cargo:rerun-if-changed={}", whisper.join(part).display());
    }
    let dst = cmake::Config::new(&native)
        .define("WHISPER_DIR", &whisper)
        .profile("Release")
        .build_target("obf_whisper")
        .build();
    let lib = dst.join("build").join("lib");
    println!("cargo:rustc-link-search=native={}", lib.display());
    for name in ["obf_whisper", "whisper", "ggml", "ggml-cpu", "ggml-base"] {
        println!("cargo:rustc-link-lib=static={name}");
    }
    let target = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    match target.as_str() {
        "macos" | "ios" => println!("cargo:rustc-link-lib=dylib=c++"),
        "windows" => {}
        _ => println!("cargo:rustc-link-lib=dylib=stdc++"),
    }
}
