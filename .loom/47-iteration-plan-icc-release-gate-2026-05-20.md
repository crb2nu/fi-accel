# RALPH Iteration Plan

## Review

- Roadmap milestone: M5 release proof and ICC utility hardening.
- Spec section(s): `.loom/32-backlog-icc-integration-2026-05-16.md` FA-ICC-01 through FA-ICC-11; `.loom/45-planning-audit-and-backlog-2026-05-20.md` FA-PLAN-04.
- Prior decisions to preserve: Gemma exactness stays opt-in and asset-backed; broad ICC utilities should be release-gated by parity tests and benchmark evidence before performance claims are promoted.

## Align

- Slice name: FA-PLAN-04 ICC Utility Release Gate.
- Scope in: fallback import completeness for ICC helpers, native/fallback parity tests for implemented ICC utilities, benchmark coverage for promoted ICC speed surfaces, compact benchmark ledger, CI execution of the Python pytest suite.
- Scope out: new utility APIs, downstream ICC integration, exact Gemma asset validation beyond the existing FA-PLAN-05 fixture, release publication.
- Acceptance criteria:
  - Each implemented FA-ICC utility has native or fallback parity coverage, with Gemma exactness documented as the opt-in exception.
  - Benchmark coverage exists for hashing, ranking, NDJSON reader, pattern matching, and mail parsing.
  - ICC can import the fallback module helpers directly without local guard code.
  - Python parity tests run in CI after the wheel install.
- Dependencies/blockers: Benchmark numbers depend on local host characteristics and must be recorded with command, date, host, and input shape.

## Land

- Planned file areas: `python/fi_accel/fallback.py`, `python/tests/`, `crates/fi-accel-core/benches/`, `.gitlab-ci.yml`, `docs/`, `.loom/`.
- Implementation steps:
  1. Fill the fallback surface gap for streaming SHA-256.
  2. Add ICC release-gate parity tests covering the implemented utility families.
  3. Add Criterion benchmark cases and a ledger entry template for promoted ICC utility claims.
  4. Wire Python tests into CI.

## Prove

- Tests to run:
  - `cargo fmt --check`
  - `cargo test -p fi-accel-core --all-features`
  - `cd python && maturin develop --quiet && python -m pytest -q tests`
  - `cargo bench -p fi-accel-core --bench benchmarks icc_release_gate`
- Lint/static checks: `cargo check --all-targets --all-features`
- CI checks: Rust lint/build/test and Python pytest suite should run on the branch pipeline.

## Handoff/Harvest

- Docs to update: `docs/benchmark-ledger.md`, FA-PLAN-04 handoff.
- Agent-context entries to add: release-gate finding and follow-up benchmark/adoption notes.
- Next-slice candidates: FA-PLAN-06 code-memory ingestion proof; FA-PLAN-09 ongoing benchmark ledger maintenance.
