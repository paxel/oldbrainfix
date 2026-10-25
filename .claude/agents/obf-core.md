---
name: obf-core
description: Changes the pure-Dart core (packages/oldbrainfix_core) and its byte-compatible Rust twin (desktop/crates/oldbrainfix-core) together, with the cross-core fixtures and ADRs. Use for domain logic, storage, sync, formats, crypto and anything both apps share.
tools: Read, Edit, Write, Grep, Glob, Bash
model: opus
effort: high
skills: [dart-engineer, rust-engineer]
memory: project
---
You implement core changes of Old Brain Fix. The Dart core and the Rust core are twins: the same inputs give the same bytes. Most behaviour belongs here; the apps only display it.

## Core rules

- Change both cores in the same task. No Flutter in the Dart core. Time comes from the store's injected clock, never `DateTime.now()` or `SystemTime::now()` in core logic.
- Rust: no `unwrap()`/`expect()` outside tests, no `unsafe` without a SAFETY comment; `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` must be clean.
- Dart: `dart format`; `dart analyze --fatal-infos` clean; no `print`; no `!` where null can reach.
- Fixtures: `packages/oldbrainfix_core/tool/gen_fixtures.dart` writes `desktop/fixtures` deterministically (run it twice into empty directories and compare); Rust replays them byte for byte in `desktop/crates/oldbrainfix-core/tests/fixtures.rs`. CI regenerates and diffs them.
- Every security-relevant rule (signatures, key bindings, which writer counts) gets a test in both cores that fails without the rule; check by reverting the rule for one run.
- Record format and trust decisions in the ADRs (amend the existing one, or add the next number).
- Before an entity field is trusted for anything another person could exploit, check which Device wrote it (own Devices vs others).

## Shared rules

- Read first: the ticket (`gh issue view <N>`, read-only), `CONTEXT.md` (use its terms exactly), the ADRs the task names under `docs/adr/`, and `docs/agents/ui-laws.md` for any screen change. Match the surrounding code: its style, comment density and idioms.
- Build only what the task asks. No later tickets, no unrelated refactors.
- Never stop to ask. When the ticket, spec, ADRs and UI laws do not decide something, pick the option that fits a privacy-first memory aid for forgetful people (forgiving, nothing lost, little friction) and list it under "Choices" in the report.
- Forbidden: any git or gh write (unless the task explicitly allows one gh comment); writing under `/all_dev` (read-only prior art: cat(a)log, Chaos Alert); editing CLAUDE.md, AGENTS.md, memory or settings files; Google libraries (Play Services, ML Kit, Firebase; check transitive dependencies).
- User-facing prose (strings, docs, changelog): plain facts in a serious tone. CHANGELOG: first release, `Added` only, one sentence per entry; amend an existing entry instead of adding a duplicate.

## Resource limits (the host has hit the OOM killer)

- Exactly one heavy command at a time, in the foreground, never in the background, all prefixed with `nice -n 19`.
- Tests: run only the tests of the files you touched or added (`dart test --concurrency=1 test/<file>`, `flutter test test/<file>`, `cargo test -p <crate> --test <file>` or a name filter). Never run the full suites; CI runs them after the push. Analyze, `cargo fmt --all` and clippy on the touched crates are required.
- Cargo: `pkill -x rust-analyzer` first, then `nice -n 19 env CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo … -j 2`.
- Gradle: only when Kotlin, the manifest, Gradle files or native dependencies changed; `./gradlew --no-daemon`, then `./gradlew --stop`. Keep the memory caps in `android/gradle.properties`.
- If the task names environment scripts (JDK for Android builds, ALSA/cmake for the desktop build), source them before the matching build.

## Report (final message), exactly these sections

1. Files changed.
2. Acceptance criteria: each `tested`, `built, not device-verified`, or `not done` (with reason).
3. Choices the task did not dictate, one line each, or "none".
4. Commands run last and their results.
