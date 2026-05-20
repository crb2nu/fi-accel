# RALPH Iteration Plan

## Review

- Roadmap milestone: M5 from `.loom/30-implementation-plan.md` - close the loop with benchmarks, docs, and one downstream adopter.
- Spec section(s): `.loom/20-product-spec.md` goals around making `fi-accel` a reusable acceleration substrate and reducing adoption risk across service consumers.
- Prior decisions to preserve: Additive APIs first; benchmark-backed promotion; keep service semantics outside `fi-accel`.

## Align

- Slice name: Service acceleration scout baseline
- Scope in:
  - Add a Rust core helper that summarizes NDJSON operation observations into ranked hot-path candidates.
  - Expose the helper through Python bindings and fallback.
  - Document the capability and add focused tests.
- Scope out:
  - No service integration yet.
  - No background profiler, tracing SDK, or benchmark harness.
  - No Go/WASM binding in this slice.
- Acceptance criteria:
  - Rust can aggregate observations by service/operation with totals, p95, max, input bytes, fallback rate, and candidate score.
  - Python exposes the same summary function with fallback parity.
  - README documents the new capability.
  - Targeted Rust and Python tests pass.
- Dependencies/blockers:
  - Downstream service adoption remains blocked on running the kill-test from `.loom/brainstorm-service-acceleration-features-2026-05-20.md`.

## Land

- Planned file areas:
  - `crates/fi-accel-core/src/scout.rs`
  - `crates/fi-accel-core/src/lib.rs`
  - `crates/fi-accel-py/src/lib.rs`
  - `python/fi_accel/`
  - `python/tests/`
  - `README.md`
- Implementation steps:
  1. Add core `AccelScoutConfig` and `AccelOperationSummary` types plus NDJSON summarization.
  2. Add Python native/fallback exports and tests.
  3. Update README and handoff notes.

## Prove

- Tests to run:
  - `cargo test -p fi-accel-core scout`
  - `python -m pytest python/tests/test_scout.py`
- Lint/static checks:
  - `cargo fmt --check`
  - `cargo check -p fi-accel-py`
- CI checks:
  - Not available locally in this turn; branch should run normal GitLab pipeline after push.

## Handoff/Harvest

- Docs to update:
  - README capability list and Python example.
  - Slice handoff in `.loom/33-slice-handoff-service-accel-scout-2026-05-20.md`.
- Agent-context entries to add:
  - Decision: scout helper is aggregation-only, not an always-on profiler.
  - Finding: first adopter still needs empirical kill-test.
- Next-slice candidates:
  - Run the kill-test against `fi-mcp-gateway` or `loom-core`.
  - Add Go binding if an MCP service proves the path.
  - Add a benchmark fixture with captured service observations.
