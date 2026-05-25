# RALPH Slice Handoff

## Slice Summary

- Milestone: M5 close the loop with benchmarks, docs, and one downstream adopter.
- Slice: FA-PLAN-06B branch reconciliation after `origin/main` merged batched exact token counting.
- Status: reconciled with current `main`; merged batch API kept.

## Branch Compared

- Local branch: `codex/fa-plan-06-token-count-overhead`
- Remote branch: `origin/codex/fa-plan-06-token-count-overhead`
- Commit: `8a4a705 feat(tokenizer): add batched token counts`

## Conclusion

Keep the merged public `count_tokens_batch` and `Tokenizer.count_batch` APIs.
The stale local checkout had a scalar-only conclusion, but `origin/main` now
contains the stronger batch-token merge and benchmark evidence.

The scalar token-gate benchmark shape is retained as historical evidence in
`scripts/bench_code_memory_token_gate.py`, while current guidance points
downstream integration to the merged `scripts/bench_code_memory_token_counts.py`
and the batch API.

## Evidence

- Batched exact counting result on `main`: local `PythonIndexer` plus
  `fi_accel.count_tokens_batch` measured 95.54 ms median versus 88.96 ms for
  the heuristic baseline on the 16-file `py-codebase-memory` fixture, while
  reducing median peak Python allocation from 1489.2 KiB to 1236.2 KiB.
- Historical scalar exact counting result: local `PythonIndexer` plus cached
  `fi_accel.count_tokens` measured 93.65 ms median versus 85.53 ms for the
  heuristic baseline. This supports the exact-token decision, but no longer
  overrides the merged batch API.
- Chunk parity result on `main`: 141 local chunks, 141 `fi_accel` chunks, and
  `first_divergence: null`.
- Branch delta now present on `main`: additive batch APIs in
  Rust/Python/fallback/docs/tests plus `scripts/bench_code_memory_token_counts.py`.

## Files Updated

- `.loom/30-implementation-plan.md`
- `.loom/40-decisions.md`
- `.loom/45-planning-audit-and-backlog-2026-05-20.md`
- `docs/benchmark-ledger.md`
- `scripts/bench_code_memory_token_gate.py`

## Next Actions

1. Downstream `py-codebase-memory` can promote exact token counting through the
   `fi_accel.count_tokens_batch` path.
2. Treat scalar-only token gate notes as historical unless they are rerun
   against current `main` and explain why the batch path no longer applies.
3. Treat full `fi_accel.chunk_code` default adoption as a separate downstream
   contract decision.
