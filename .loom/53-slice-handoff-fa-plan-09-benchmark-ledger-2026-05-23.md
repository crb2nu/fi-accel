# RALPH Slice Handoff

## Slice Summary

- Milestone: M5 close the loop with benchmarks, docs, and one downstream adopter.
- Slice: FA-PLAN-09 Benchmark Ledger Claim Discipline.
- Status: complete.

## What Landed

- Key changes:
  - Removed broad legacy speed multipliers from README feature bullets and the Python package docstring.
  - Replaced the README performance table with ledger-backed highlights only.
  - Added a benchmark-ledger claim policy and marked FA-PLAN-09 complete in planning docs.
- Key files:
  - `README.md`
  - `python/fi_accel/__init__.py`
  - `docs/benchmark-ledger.md`
  - `.loom/30-implementation-plan.md`
  - `.loom/40-decisions.md`
  - `.loom/45-planning-audit-and-backlog-2026-05-20.md`
- Validation results:
  - `rg -n "10-50x|50-100x|10-100x|Speedup|SSE parse|TOML parse|TOON parse" README.md python/fi_accel/__init__.py docs/benchmark-ledger.md .loom/30-implementation-plan.md .loom/45-planning-audit-and-backlog-2026-05-20.md`
  - `python -m py_compile python/fi_accel/__init__.py`
  - Documentation-only slice; full runtime tests were not run.

## What Is Still Open

- Remaining acceptance criteria: none for FA-PLAN-09.
- Known issues:
  - Some code-level crate doc comments still use generic "high-performance" wording, but they do not publish numeric claims.
  - Fresh benchmark runs are still needed before promoting speed claims for tokenizer, embeddings, diff, SSE, TOML, TOON, or WASM parity.
- Dependencies:
  - New performance claims should update `docs/benchmark-ledger.md` before README or release notes repeat them.

## Next Actions

1. Land the downstream exact-token default flip in `py-codebase-memory`.
2. Decide FA-PLAN-08 WASM tokenizer and code-chunking bundle cost.
3. Add fresh ledger entries before promoting unmeasured surfaces.

## Context Links

- Agent-context session: `3dc38e3789601cbf`
- Task IDs: none.
- Relevant docs/specs:
  - `.loom/20-product-spec.md`
  - `.loom/30-implementation-plan.md`
  - `.loom/45-planning-audit-and-backlog-2026-05-20.md`
  - `docs/benchmark-ledger.md`
