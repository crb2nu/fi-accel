# RALPH Slice Handoff

## Slice Summary

- Milestone: M5 close the loop with benchmarks, docs, and one downstream adopter.
- Slice: FA-PLAN-06B Repeated Token Counting Fast Path.
- Status: implemented and benchmarked; downstream default adoption is now unblocked for local chunking plus exact batched token counts.

## What Landed

- Added `fi_accel.count_tokens_batch(texts, model="gpt-4")`.
- Added `fi_accel.Tokenizer(...).count_batch(texts)`.
- Kept existing scalar `count_tokens` and `Tokenizer.count` APIs unchanged.
- Added `scripts/bench_code_memory_token_counts.py` for the three-mode `py-codebase-memory` token-count benchmark shape:
  - local `PythonIndexer` plus heuristic token estimates
  - local `PythonIndexer` plus batched exact `fi_accel` token counts
  - `fi_accel.chunk_code(model="gpt-4")` using chunker-provided exact token counts

## Validation Result

Command:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable ./python \
  python scripts/bench_code_memory_token_counts.py --repeats 7
```

Result:

- Local heuristic: 88.96 ms median, 1489.2 KiB median peak Python allocation, 141 chunks.
- Local plus batched exact token counts: 95.54 ms median, 1236.2 KiB median peak Python allocation, 141 chunks.
- `fi_accel.chunk_code(model="gpt-4")`: 26.70 ms median, 706.3 KiB median peak Python allocation, 141 chunks.

Parity command:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable ./python \
  python scripts/check_code_memory_chunk_parity.py
```

Result: PASS with 141 local chunks, 141 `fi_accel` chunks, and `first_divergence: null`.

## Adoption Decision

Local `PythonIndexer` chunking plus exact `fi_accel` token counts is now defaultable for the current fixture if downstream uses `count_tokens_batch` instead of per-chunk scalar calls.

Full `fi_accel.chunk_code(model="gpt-4")` also clears the fixture gates, but it should be promoted through an explicit downstream `py-codebase-memory` integration slice so the index contract change is visible.

## Next Actions

1. Update `py-codebase-memory` to batch exact token counting for local chunks.
2. Optionally evaluate making `CODEBASE_MEMORY_PYTHON_CHUNKER_MODE=fi-accel` the default after a downstream smoke test over a broader repo set.
