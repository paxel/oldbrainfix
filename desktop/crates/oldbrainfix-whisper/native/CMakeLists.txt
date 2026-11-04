# Whisper for the desktop's voice Notes (ADR 0007): the whisper.cpp and
# ggml sources of the submodule third_party/whisper.cpp the phone builds
# too, unchanged and built the phone's way (CPU only, built in, no OpenMP),
# with a small C bridge for Rust. Built by build.rs through the cmake
# crate; WHISPER_DIR comes from there.
cmake_minimum_required(VERSION 3.18)
project(oldbrainfix_whisper C CXX)

# Every static library lands in one place for cargo to link.
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY_RELEASE ${CMAKE_BINARY_DIR}/lib)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
set(GGML_OPENMP OFF CACHE BOOL "" FORCE)
# Only the CPU backend is built: no Metal or BLAS, also on macOS.
set(GGML_METAL OFF CACHE BOOL "" FORCE)
set(GGML_BLAS OFF CACHE BOOL "" FORCE)
# Accelerate would need the framework linked, which cargo never hears of.
set(GGML_ACCELERATE OFF CACHE BOOL "" FORCE)
set(GGML_BACKEND_DL OFF CACHE BOOL "" FORCE)
set(GGML_LLAMAFILE OFF CACHE BOOL "" FORCE)
set(GGML_CCACHE OFF CACHE BOOL "" FORCE)
set(GGML_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(GGML_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
# Not tuned to the computer it is built on, so the app runs on others;
# on x86-64 it takes the x86-64-v3 level (AVX2, FMA, F16C: every PC since
# about 2013) instead, without which Whisper is far too slow.
set(GGML_NATIVE OFF CACHE BOOL "" FORCE)
if(CMAKE_SYSTEM_PROCESSOR MATCHES "x86_64|AMD64|amd64")
    foreach(feature SSE42 AVX AVX2 BMI2 FMA F16C)
        set(GGML_${feature} ON CACHE BOOL "" FORCE)
    endforeach()
endif()
# No commit stamped into ggml, so it builds the same from any checkout.
set(GIT_EXE OFF CACHE STRING "" FORCE)
add_subdirectory(${WHISPER_DIR}/ggml ggml)

# The version whisper_version() reports, from the submodule's own
# CMakeLists.txt, as on the phone.
file(STRINGS ${WHISPER_DIR}/CMakeLists.txt WHISPER_VERSION_PARTS
    REGEX "^set\\(WHISPER_VERSION_(MAJOR|MINOR|PATCH) [0-9]+\\)")
string(REGEX REPLACE "[^0-9;]" "" WHISPER_VERSION_PARTS "${WHISPER_VERSION_PARTS}")
list(JOIN WHISPER_VERSION_PARTS "." WHISPER_VERSION)

add_library(whisper STATIC
    ${WHISPER_DIR}/src/whisper.cpp
    ${WHISPER_DIR}/src/aneforge/whisper-aneforge.cpp)
target_include_directories(whisper PUBLIC ${WHISPER_DIR}/include ${WHISPER_DIR}/src)
target_compile_definitions(whisper PRIVATE WHISPER_VERSION="${WHISPER_VERSION}")
target_compile_features(whisper PUBLIC cxx_std_11)
target_link_libraries(whisper PUBLIC ggml)

add_library(obf_whisper STATIC obf_whisper.cpp)
target_compile_features(obf_whisper PRIVATE cxx_std_17)
target_link_libraries(obf_whisper PRIVATE whisper)
