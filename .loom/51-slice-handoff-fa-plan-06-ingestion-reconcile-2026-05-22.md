# RALPH Slice Handoff

## Slice Summary

- Milestone: M5 close the loop with benchmarks, docs, and one downstream adopter.
- Slice: FA-PLAN-06 Code-Memory Ingestion Proof reconciliation.
- Status: prototype adopted as evidence, default downstream adoption blocked.

## What Landed

- Added `scripts/bench_code_memory_ingestion.py`, a repeatable benchmark for a
  concrete `py-codebase-memory` ingestion-record path.
- The script compares:
  - local `PythonIndexer` + `read_text` + `hashlib.sha256` + Python ranking glue
  - `fi_accel.detect_encoding` + `fi_accel.chunk_code` + `fi_accel.sha256_batch`
    + `fi_accel.hybrid_rank`
- Updated `docs/benchmark-ledger.md` with the 2026-05-22 before/after result.

## Validation Result

Command:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable ./python \
  python scripts/bench_code_memory_ingestion.py --repeats 7
```

Result:

- Local baseline: 90.48 ms median, 1516.4 KiB median peak Python allocation,
  141 chunks.
- fi-accel prototype: 20.52 ms median, 913.5 KiB median peak Python allocation,
  143 chunks.
- Ratio: fi-accel path is 0.23x local wall time and 0.60x local peak Python
  allocation for ingestion-record construction.

## Adoption Decision

Adopt the full helper path as a documented prototype recipe, not as the default
`py-codebase-memory` ingestion path.

The positive result is real for record construction when the path avoids
per-chunk `fi_accel.count_tokens`: `detect_encoding`, `chunk_code`,
`sha256_batch`, and `hybrid_rank` compose cleanly and reduce both wall time and
Python allocation on the current downstream source tree.

The blocker is precise: chunk cardinality and ranked IDs differ from the local
indexer, and the earlier downstream opt-in path that uses per-chunk
`fi_accel.count_tokens` remains too slow for default adoption. Default adoption
needs either chunk identity/cardinality parity or an explicit downstream index
contract that accepts the new chunk shape, plus a batched/specialized token
counting path.

## Next Actions

1. Add a parity fixture that compares local and `fi_accel.chunk_code` chunk
   identity, line spans, names, and call/definition metadata on the 16-file
   `py-codebase-memory` source set.
2. Prototype batched token counting or reuse chunker-provided token counts
   without per-chunk Python calls.
3. Revisit default `py-codebase-memory` adoption only after the parity and token
   counting blockers are closed.
