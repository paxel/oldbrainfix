#!/usr/bin/env sh
# Ask end to end against a real local model (obf-1024, #331): starts
# Ollama in a container with a small tool-calling model, runs the ignored
# tests of desktop/crates/oldbrainfix-gui/tests/ask_e2e.rs against it (the
# desktop's question, the phone's question through the paired host, and
# the Ask panel drawn into PNG files), and stops it again. No CI job runs
# this; see docs/agents/ask-e2e.md.
#
#   tool/ask-e2e.sh run [filter]   build, start, test, stop (the default)
#   tool/ask-e2e.sh start          start Ollama and pull the model
#   tool/ask-e2e.sh test [filter]  run the tests against a started Ollama
#   tool/ask-e2e.sh stop           stop and remove the container
#
# The build runs before the model starts, so the two never need memory
# at once. Settings, all optional:
#   ASK_E2E_MODEL       the model (default qwen2.5:3b)
#   ASK_E2E_PORT        the port on 127.0.0.1 (default 11435, beside an
#                       Ollama of one's own on 11434)
#   ASK_E2E_CACHE       where the models are kept, outside the repo
#                       (default ~/.cache/oldbrainfix/ask-e2e)
#   ASK_E2E_SHOTS       where the panel's PNG files go, outside the repo
#                       (default the system's temp folder)
#   ASK_E2E_LIMIT_SECS  how long an answer may take (default 120)
#   ASK_E2E_HARD_SECS   how long a test waits at all (default 900)
#   ASK_E2E_JOBS        Rust build jobs (default 1)
#   CONTAINER           docker or podman (default: docker, else podman)
# The desktop's gui tests link ALSA and draw through wgpu: a software
# Vulkan such as lavapipe draws the PNG files, as for tool/guide-frames.sh.
set -eu
cd "$(dirname "$0")/.."

# Ollama 0.40.13 (2026-10-29), its multi-arch index by digest.
IMAGE="docker.io/ollama/ollama@sha256:b2f0122c476bc9fe7ce679d8bb4bd555351e25e2554eefb156f61cb0a89cf429"
NAME="oldbrainfix-ask-e2e"
MODEL="${ASK_E2E_MODEL:-qwen2.5:3b}"
PORT="${ASK_E2E_PORT:-11435}"
CACHE="${ASK_E2E_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/oldbrainfix/ask-e2e}"
JOBS="${ASK_E2E_JOBS:-1}"
BASE="http://127.0.0.1:$PORT"
if [ -z "${CONTAINER:-}" ]; then
  if command -v docker >/dev/null 2>&1; then CONTAINER=docker; else CONTAINER=podman; fi
fi

start() {
  mkdir -p "$CACHE"
  if ! "$CONTAINER" container inspect "$NAME" >/dev/null 2>&1; then
    # Only this computer reaches it; :Z lets SELinux allow the cache.
    nice -n 19 "$CONTAINER" run -d --rm --name "$NAME" \
      -p "127.0.0.1:$PORT:11434" -v "$CACHE:/root/.ollama:Z" "$IMAGE" >/dev/null
  fi
  i=0
  until curl -sf "$BASE/api/version" >/dev/null; do
    i=$((i + 1))
    if [ "$i" -gt 60 ]; then
      echo "Ollama did not start on $BASE" >&2
      "$CONTAINER" logs "$NAME" >&2 || true
      exit 1
    fi
    sleep 1
  done
  echo "Ollama $(curl -sf "$BASE/api/version") on $BASE, models in $CACHE"
  echo "pulling $MODEL…"
  curl -sSf "$BASE/api/pull" -d "{\"model\":\"$MODEL\",\"stream\":false}"
  echo
  # Loads the model once, so that no test pays for the load; the first
  # question still reads the system prompt and the tools afresh.
  t0=$(date +%s)
  curl -sSf "$BASE/api/generate" -d "{\"model\":\"$MODEL\",\"prompt\":\"\"}" >/dev/null
  echo "$MODEL loaded in $(($(date +%s) - t0))s"
}

stop() {
  "$CONTAINER" rm -f "$NAME" >/dev/null 2>&1 || true
}

cargo_test() {
  (cd desktop && nice -n 19 env CARGO_BUILD_JOBS="$JOBS" \
    cargo test -j "$JOBS" -p oldbrainfix-gui --test ask_e2e --locked "$@")
}

test_() {
  export ASK_E2E_URL="$BASE/v1" ASK_E2E_MODEL="$MODEL"
  status=0
  # One question at a time: the model answers one request at a time.
  cargo_test -- --ignored --test-threads=1 --nocapture "$@" || status=$?
  echo "--- Ollama after the run"
  curl -sS "$BASE/api/ps" || true
  echo
  "$CONTAINER" logs "$NAME" 2>&1 | grep -iE "truncat|num_ctx|n_ctx|context" | tail -20 || true
  return "$status"
}

case "${1:-run}" in
  start) start ;;
  stop) stop ;;
  test) shift; test_ "$@" ;;
  run)
    [ $# -gt 0 ] && shift
    echo "building the tests…"
    cargo_test --no-run
    trap stop EXIT INT TERM
    start
    test_ "$@"
    ;;
  *)
    echo "usage: tool/ask-e2e.sh [run|start|test|stop] [test filter]" >&2
    exit 2
    ;;
esac
