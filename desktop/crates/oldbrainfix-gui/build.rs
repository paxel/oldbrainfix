//! Generates the string tables from the phone app's ARB files, so the
//! desktop and the phone share every translation. The few strings only
//! the desktop needs lie over them from `desktop/l10n`.

use std::path::Path;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let phone = manifest.join("../../../lib/l10n");
    let desktop = manifest.join("../../l10n");
    println!("cargo:rerun-if-changed={}", phone.display());
    println!("cargo:rerun-if-changed={}", desktop.display());
    let written = oldbrainfix_l10n::generate_dirs(&[&phone, &desktop], "en").and_then(|source| {
        let out = std::env::var("OUT_DIR").map_err(|e| format!("OUT_DIR: {e}"))?;
        std::fs::write(Path::new(&out).join("l10n.rs"), source).map_err(|e| format!("l10n.rs: {e}"))
    });
    if let Err(e) = written {
        println!("cargo:warning=l10n: {e}");
        std::process::exit(1);
    }
}
