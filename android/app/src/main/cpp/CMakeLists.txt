# Whisper for voice Notes (ADR 0007): whisper.cpp with its ggml, CPU
# only, linked with the app's JNI bridge into one library,
# liboldbrainfix_whisper.so. No network code, no GPU, no Google library.
#
# whisper.cpp is the git submodule third_party/whisper.cpp, pinned to an
# upstream release (MIT, see its LICENSE; third_party/README.md names the
# tag); only its CPU parts are built, unchanged. A submodule rather than
# fetched at build time, so the APK builds offline and the sources cannot
# change under the pinned commit.
cmake_minimum_required(VERSION 3.18)
project(oldbrainfix_whisper C CXX)

# ggml is heavy to compile: at most two compiles and one link at a time,
# whatever ninja is told, so the build never takes the machine down.
set_property(GLOBAL PROPERTY JOB_POOLS compile=2 link=1)
set(CMAKE_JOB_POOL_COMPILE compile)
set(CMAKE_JOB_POOL_LINK link)

# Optimised in debug builds too: unoptimised ggml is far too slow to
# transcribe anything on a phone. Comes after the build type's flags.
add_compile_options(-O2)

set(WHISPER_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../../../../../third_party/whisper.cpp)
if(NOT EXISTS ${WHISPER_DIR}/include/whisper.h)
    message(FATAL_ERROR "whisper.cpp is missing: git submodule update --init")
endif()
# The version whisper_version() reports, from the submodule's
# own CMakeLists.txt, so an update of the pin changes it with it.
file(STRINGS ${WHISPER_DIR}/CMakeLists.txt WHISPER_VERSION_PARTS
    REGEX "^set\\(WHISPER_VERSION_(MAJOR|MINOR|PATCH) [0-9]+\\)")
string(REGEX REPLACE "[^0-9;]" "" WHISPER_VERSION_PARTS "${WHISPER_VERSION_PARTS}")
list(JOIN WHISPER_VERSION_PARTS "." WHISPER_VERSION)

# Static parts linked into one shared library.
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# ggml with the CPU backend only, built in, without OpenMP and for the
# ABI's baseline (armv8-a on arm64): it runs on every phone of the ABI.
set(BUILD_SHARED_LIBS OFF CACHE BOOL "" FORCE)
set(GGML_NATIVE OFF CACHE BOOL "" FORCE)
set(GGML_OPENMP OFF CACHE BOOL "" FORCE)
set(GGML_BACKEND_DL OFF CACHE BOOL "" FORCE)
set(GGML_LLAMAFILE OFF CACHE BOOL "" FORCE)
set(GGML_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(GGML_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
# No commit stamped into ggml, so the library builds the same from any
# checkout, with or without git.
set(GIT_EXE OFF CACHE STRING "" FORCE)
add_subdirectory(${WHISPER_DIR}/ggml ggml)

add_library(whisper STATIC
    ${WHISPER_DIR}/src/whisper.cpp
    ${WHISPER_DIR}/src/aneforge/whisper-aneforge.cpp)
target_include_directories(whisper PUBLIC ${WHISPER_DIR}/include ${WHISPER_DIR}/src)
target_compile_definitions(whisper PRIVATE WHISPER_VERSION="${WHISPER_VERSION}")
target_compile_features(whisper PUBLIC cxx_std_11)
target_link_libraries(whisper PUBLIC ggml)

add_library(oldbrainfix_whisper SHARED whisper_jni.cpp)
target_compile_features(oldbrainfix_whisper PRIVATE cxx_std_17)
target_link_libraries(oldbrainfix_whisper PRIVATE whisper log)
# Only the JNI functions leave the library.
target_link_options(oldbrainfix_whisper PRIVATE -Wl,--exclude-libs,ALL -Wl,--gc-sections)
