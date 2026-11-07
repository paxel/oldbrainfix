# Third-party sources

Sources the apps build from that Old Brain Fix does not write. Each is a git submodule pinned to an upstream release tag ([ADR 0045](../docs/adr/0045-third-party-sources-are-pinned-submodules.md)), built unchanged. The submodule's commit is the pin; the Pinned column names the tag it is, and `licenses/common.txt` repeats it for the About screen. `python3 tool/whisper_pin.py` fails CI and the release when they part, and warns when upstream has a newer release.

| Submodule     | Upstream                                | Pinned   | Licence                                               | Built by                                                                                                                                                           |
|---------------|-----------------------------------------|----------|-------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `whisper.cpp` | https://github.com/ggml-org/whisper.cpp | `v1.9.5` | MIT, in [licenses/common.txt](../licenses/common.txt) | the phone (`android/app/src/main/cpp/CMakeLists.txt`), the desktop (`desktop/crates/oldbrainfix-whisper/build.rs`), the Flatpak, the iPhone (`tool/ios-native.sh`) |

Only whisper.cpp's CPU parts are compiled: `include/`, `src/` and `ggml/` with its CPU backend. The lockfiles of its examples and bindings are never built and are left out of the OSV scan.
