# RALPH Slice Handoff

## Slice Summary

- Milestone: M5 release proof and ICC utility hardening.
- Slice: FA-PLAN-04 ICC Utility Release Gate.
- Status: complete.

## What Landed

- Key changes:
  - Added `fi_accel.fallback.sha256_stream` so ICC can import the fallback hashing surface directly.
  - Added ICC release-gate parity tests for hashing, text/encoding, NDJSON reader behavior, ranking/RRF, pattern matching, mail parsing, and Gemma opt-in behavior.
  - Fixed fallback `NDJSONReader` line-number accounting so trailing newline segments do not drift from the Rust reader.
  - Added Criterion benchmarks for hashing, ranking, NDJSON reader, pattern matching, and mail parsing under the `icc_release_gate` group.
  - Added `docs/benchmark-ledger.md` and linked README performance wording to the ledger.
  - Updated CI to run `python -m pytest -q python/tests` after installing the built wheel.
  - May 23 follow-up: added an ICC-shaped hash-chain service fixture and aligned fallback mismatch diagnostics with the native verifier.
- Key files:
  - `python/fi_accel/fallback.py`
  - `python/tests/test_icc_release_gate.py`
  - `crates/fi-accel-core/benches/benchmarks.rs`
  - `docs/benchmark-ledger.md`
  - `.gitlab-ci.yml`
  - `.loom/32-backlog-icc-integration-2026-05-16.md`
  - `README.md`
- Validation results:
  - `cargo fmt --check`
  - `cargo check --all-targets --all-features`
  - `cargo test -p fi-accel-core --all-features`
  - `cd python && python -m pytest -q tests`
  - temporary venv native-extension run: `maturin develop --quiet --extras dev && python -m pytest -q tests`
  - `cargo bench -p fi-accel-core --bench benchmarks icc_release_gate`
  - `cd go/fiaccel && CGO_ENABLED=0 go test ./...`
  - May 23 follow-up: `uv run --project python --extra dev python -m pytest -q python/tests/test_icc_release_gate.py -k hash_chain`

## What Is Still Open

- Remaining acceptance criteria: none for this release-gate slice.
- Known issues:
  - Hashing speedup claims still need a Python `hashlib` loop comparator before release notes claim a multiplier.
  - Gemma exactness still depends on the existing opt-in local tokenizer asset fixture from FA-PLAN-05.
- Dependencies:
  - Future benchmark entries should preserve host/date/input shape before promotion.

## Next Actions

1. Use `docs/benchmark-ledger.md` as the source of truth for release notes and future README performance claims.
2. Continue with FA-PLAN-06 Code-Memory Ingestion Proof as the next adoption-focused slice.
3. Add Python comparator benchmarks only when preparing explicit speedup claims for ICC release notes.

## Context Links

- Agent-context session: `aa4185770ec38b4c`
- Task IDs: none.
- Relevant docs/specs:
  - `.loom/47-iteration-plan-icc-release-gate-2026-05-20.md`
  - `.loom/32-backlog-icc-integration-2026-05-16.md`
  - `docs/benchmark-ledger.md`
