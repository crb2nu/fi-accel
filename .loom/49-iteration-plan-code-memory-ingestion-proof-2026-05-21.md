# RALPH Iteration Plan

## Review

- Roadmap milestone: M5 close the loop with benchmarks, docs, and one downstream adopter.
- Spec section(s): FA-PLAN-06 Code-Memory Ingestion Proof; benchmark-ledger release evidence.
- Prior decisions to preserve: performance claims must be benchmark-backed; failed downstream adoption gates should become specific follow-up tickets instead of default integrations.

## Align

- Slice name: FA-PLAN-06 Code-Memory Ingestion Proof.
- Scope in: finish the existing `py-codebase-memory` adoption branch, make `fi-accel` chunking/token counting explicit opt-ins, add fallback/opt-in tests, document downstream config, record benchmark evidence.
- Scope out: changing `fi-accel` tokenizer/chunker internals, replacing non-Python indexers, fixing unrelated `py-codebase-memory` visualization tests.
- Acceptance criteria:
  - `py-codebase-memory` can exercise `fi_accel.chunk_code` and `fi_accel.count_tokens` without local guard code.
  - Default indexing behavior remains local when benchmark evidence does not justify automatic acceleration.
  - Benchmark result documents latency, peak Python allocation, chunk counts, and the specific blocker.
  - fi-accel ledger/handoff records whether adoption succeeded or failed.
- Dependencies/blockers: local `fi-accel` Python binding must be built into the `py-codebase-memory` venv for real-binding validation.

## Land

- Planned file areas:
  - Downstream: `/Users/cblevins/workspace/libs/py-codebase-memory/src/codebase_memory/indexers/`, `/Users/cblevins/workspace/libs/py-codebase-memory/tests/`, `/Users/cblevins/workspace/libs/py-codebase-memory/README.md`
  - fi-accel: `docs/benchmark-ledger.md`, `.loom/`
- Implementation steps:
  1. Preserve the existing optional `fi_accel.chunk_code` bridge and gate it behind `CODEBASE_MEMORY_PYTHON_CHUNKER_MODE=fi-accel`.
  2. Add optional `fi_accel.count_tokens` support behind `CODEBASE_MEMORY_TOKEN_ESTIMATE_MODE=auto`.
  3. Add unit tests for opt-in accelerated paths and local fallback defaults.
  4. Benchmark local versus accelerated ingestion on the downstream source tree.

## Prove

- Tests to run:
  - `uv run --extra dev python -m pytest -q tests/test_indexers.py`
  - `uv run --extra dev python -m pytest -q`
- Lint/static checks:
  - `uv run --extra dev ruff check src/codebase_memory/indexers/base.py src/codebase_memory/indexers/python_indexer.py tests/test_indexers.py`
- CI checks: downstream branch pipeline after push; fi-accel docs-only pipeline after push.

## Handoff/Harvest

- Docs to update: `docs/benchmark-ledger.md`, downstream README, FA-PLAN-06 handoff.
- Agent-context entries to add: adoption proof finding and follow-up blocker.
- Next-slice candidates: specialize repeated tokenizer/chunker calls for code-memory, or move to FA-PLAN-07 Go parity decision.
