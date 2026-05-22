# RALPH Slice Handoff

## Slice Summary

- Milestone: M5 close the loop with benchmarks, docs, and one downstream adopter.
- Slice: FA-PLAN-06 Code-Memory Ingestion Proof.
- Status: partial.

## What Landed

- Key changes:
  - Continued the downstream `py-codebase-memory` branch `codex/pycodebase-memory-fi-accel-adoption`.
  - Kept Python chunk extraction local by default and made `fi_accel.chunk_code` opt-in with `CODEBASE_MEMORY_PYTHON_CHUNKER_MODE=fi-accel`.
  - Added optional `fi_accel.count_tokens` token estimation behind `CODEBASE_MEMORY_TOKEN_ESTIMATE_MODE=auto`; the default remains the local heuristic.
  - Added tests proving accelerated opt-in behavior, default local chunking, accelerated token counting, heuristic fallback, and size-gated token counting.
  - Documented the downstream environment knobs in `py-codebase-memory/README.md`.
  - Added code-memory ingestion proof results to `docs/benchmark-ledger.md`.
- Key files:
  - `/Users/cblevins/workspace/libs/py-codebase-memory/src/codebase_memory/indexers/python_indexer.py`
  - `/Users/cblevins/workspace/libs/py-codebase-memory/src/codebase_memory/indexers/base.py`
  - `/Users/cblevins/workspace/libs/py-codebase-memory/tests/test_indexers.py`
  - `/Users/cblevins/workspace/libs/py-codebase-memory/README.md`
  - `docs/benchmark-ledger.md`
- Validation results:
  - Passed: `uv run --extra dev python -m pytest -q tests/test_indexers.py`
  - Passed: `uv run --extra dev ruff check src/codebase_memory/indexers/base.py src/codebase_memory/indexers/python_indexer.py tests/test_indexers.py`
  - Failed unrelated broader check: `uv run --extra dev python -m pytest -q` fails in `tests/test_visualization.py::test_generate_dependency_graph` because the existing visualization mock now reaches the real `diagram_gen.renderers.svg.SVGRenderer` path with an empty graph.
  - Benchmark: local heuristic path 86.42 ms median and 1462.8 KiB median peak Python allocation; local plus `fi_accel.count_tokens` 4152.80 ms median; `fi_accel.chunk_code` plus token counting 4199.62 ms median and 609.7 KiB median peak Python allocation.

## What Is Still Open

- Remaining acceptance criteria:
  - The adoption proof did not justify making `fi-accel` the default ingestion path.
  - A 2026-05-22 reconciliation pass found a faster full-helper ingestion-record
    prototype, but it is still blocked on chunk identity/cardinality parity before
    default downstream adoption. See
    `.loom/51-slice-handoff-fa-plan-06-ingestion-reconcile-2026-05-22.md`.
  - A future `fi-accel` slice needs to reduce repeated tokenizer/chunker overhead before this can become a promoted code-memory acceleration claim.
- Known issues:
  - `fi_accel.count_tokens` dominates wall time when called for each downstream chunk.
  - `fi_accel.chunk_code` returns a similar chunk shape but is still roughly 48x slower than local tree-sitter on the current downstream source set, while reducing median peak Python allocation by about 58%.
  - Full downstream pytest has the pre-existing visualization mock failure noted above.
- Dependencies:
  - Downstream branch pipeline and MR still need to be completed for the opt-in integration.

## Next Actions

1. Commit and push the downstream `py-codebase-memory` opt-in integration branch.
2. Commit and push this fi-accel docs/ledger branch.
3. Open follow-up work for specialized repeated token counting or a chunker fast path before revisiting default code-memory adoption.

## Context Links

- Agent-context session: `8ae2c32a4689a973`
- Task IDs: none.
- Relevant docs/specs:
  - `.loom/49-iteration-plan-code-memory-ingestion-proof-2026-05-21.md`
  - `docs/benchmark-ledger.md`
  - `/Users/cblevins/workspace/libs/py-codebase-memory/README.md`
