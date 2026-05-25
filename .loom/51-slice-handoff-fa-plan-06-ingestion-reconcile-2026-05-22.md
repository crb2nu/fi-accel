# RALPH Slice Handoff

## Slice Summary

- Milestone: M5 close the loop with benchmarks, docs, and one downstream adopter.
- Slice: FA-PLAN-06 Code-Memory Ingestion Proof reconciliation.
- Status: prototype adopted as evidence; exact token default unblocked by the May 23 rerun, full helper default still separate.

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

The original blocker was precise: chunk cardinality and ranked IDs differed
from the local indexer, and the earlier downstream opt-in path that used
per-chunk `fi_accel.count_tokens` was too slow for default adoption. The May 23
rerun changes the token-counting conclusion: after the chunk parity merge,
parity passes on the 16-file fixture and local `PythonIndexer` plus cached
`fi_accel.count_tokens` measures 93.65 ms median versus 85.53 ms heuristic
baseline, about 9.5% overhead. Promote exact token counting for the local
PythonIndexer path; treat full `fi_accel.chunk_code` adoption as a separate
contract decision.

## Next Actions

1. Land the downstream default flip for exact token counting when
   `fi_accel.count_tokens` is available, preserving local `PythonIndexer`
   chunking.
2. Decide full `fi_accel.chunk_code` default adoption from the broader
   downstream chunk contract, not from the now-closed token gate.
