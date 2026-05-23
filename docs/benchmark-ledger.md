# Benchmark Ledger

This ledger is the source of truth for performance evidence behind promoted
`fi-accel` claims. Release notes should link here or use conservative wording
unless a surface has a dated command, host, input shape, and result.

## Host Notes

- Host: Apple M4 local development machine unless noted otherwise.
- Date format: UTC date for command execution.
- Rust command shape: `cargo bench -p fi-accel-core --bench benchmarks <filter>`.

## ICC Utility Release Gate

Run:

```bash
cargo bench -p fi-accel-core --bench benchmarks icc_release_gate
```

| Surface | Benchmark | Input shape | Latest result | Date | Notes |
| --- | --- | --- | --- | --- | --- |
| Hashing | `icc_release_gate/sha256_batch_100k_256b` | 100K byte strings, 256 bytes each | 12.340-12.616 ms | 2026-05-21 | Backs FA-ICC-01 batch-hash claims. Compare against Python `hashlib` loop before claiming a multiplier. |
| Ranking | `icc_release_gate/hybrid_rank_1k` | 1K lexical ranks plus 1K cosine scores | 13.588-13.648 us | 2026-05-21 | Backs FA-ICC-03 hybrid-rank overhead claims. |
| Ranking | `icc_release_gate/rrf_fuse_3x1k` | Three ranked lists, 1K IDs each | 123.04-125.94 us | 2026-05-21 | Backs FA-ICC-09 RRF helper claims. |
| NDJSON | `icc_release_gate/ndjson_reader_append_1k` | 1K JSONL events appended in one chunk | 534.99-542.39 us | 2026-05-21 | Backs FA-ICC-06 streaming-reader claims. |
| Pattern matching | `icc_release_gate/literal_pattern_128_over_1k_lines` | 128 literals over 1K text lines | 1.1514-1.1599 ms | 2026-05-21 | Backs FA-ICC-07 pattern-count scaling claims. |
| Pattern matching | `icc_release_gate/regex_pattern_2_over_1k_lines` | Two regexes over 1K text lines | 71.574-72.020 us | 2026-05-21 | Tracks regex mode separately from literal matching. |
| Mail parsing | `icc_release_gate/parse_eml_multipart_attachment` | Multipart EML with text, HTML, and one attachment | 3.1875-3.3575 us | 2026-05-21 | Backs FA-ICC-08 parser overhead claims. |

## Transport Route Preview

Run from `go/fiaccel`:

```bash
go test ./... -run '^$' -bench 'Benchmark(ParseJSONRPCRoutePreviewGatewayToolsCall|ExtractJSONRPCPrefixRouteGatewayToolsCall)$' -benchmem -count=5
```

| Surface | Benchmark | Input shape | Latest result | Date | Notes |
| --- | --- | --- | --- | --- | --- |
| Go transport | `BenchmarkParseJSONRPCRoutePreviewGatewayToolsCall` | production-shaped MCP `tools/call` route payload | 5123-5436 ns/op, 1744 B/op, 22 allocs/op | 2026-05-20 | Standalone helper result from `docs/service-acceleration-kill-test-2026-05-20.md`. |
| Go transport | `BenchmarkExtractJSONRPCPrefixRouteGatewayToolsCall` | same payload, gateway prefix route fields only | 234.9-238.6 ns/op, 64 B/op, 2 allocs/op | 2026-05-22 | Extracts method and `params.name` only; does not copy id or raw arguments. |

Downstream `fi-mcp-gateway` service adoption did not clear the gate on
2026-05-20: the helper path reached approximate wall-time parity but used
slightly more bytes and allocations than the service's current route path.
The narrower prefix-route extractor clears the original standalone gate for
prefix-only routing, but downstream service adoption still needs a service-local
benchmark because argument-aware routing requires `params.arguments`.

## Code-Memory Ingestion Proof

Run from `/Users/cblevins/workspace/libs/py-codebase-memory` after installing
the local `fi-accel` Python binding:

```bash
uv pip install -e /Users/cblevins/workspace/libs/fi-accel/.worktrees/codex-code-memory-ingestion-proof/python
uv run --extra dev python <inline benchmark script>
```

Input shape: all 16 Python files under `src/codebase_memory`, 114256 bytes
total, parsed repeatedly with `PythonIndexer`.

| Surface | Mode | Latest result | Date | Notes |
| --- | --- | --- | --- | --- |
| Python code-memory ingestion | local tree-sitter + heuristic token estimate | 86.42 ms median, 1462.8 KiB median peak Python allocation, 141 chunks | 2026-05-21 | Baseline downstream path. |
| Python code-memory ingestion | local tree-sitter + `fi_accel.count_tokens` | 4152.80 ms median, 1462.3 KiB median peak Python allocation, 141 chunks | 2026-05-21 | Does not clear adoption gate; per-chunk BPE counting dominates wall time. |
| Python code-memory ingestion | `fi_accel.chunk_code` + `fi_accel.count_tokens` | 4199.62 ms median, 609.7 KiB median peak Python allocation, 143 chunks | 2026-05-21 | Reduces peak Python allocation but fails the wall-time gate by a large margin. |

Decision: keep the `py-codebase-memory` integration opt-in for both Python
chunk extraction and token counting. Do not promote `fi-accel` as the default
code-memory ingestion path until the tokenizer/chunker call path is specialized
for repeated small/medium source chunks or benchmarked against a consumer where
lower Python allocation matters more than wall time.

### Cached Tokenizer Follow-Up

Narrow fix chosen: repeated-small-file tokenizer fast path. `py-codebase-memory`
already calls `fi_accel.count_tokens` per local chunk when exact token mode is
enabled, so caching tiktoken encodings behind that existing API attacks the May
21 blocker without a downstream API change, batch integration work, or changing
the chunk identity contract.

Run from this repo against the current `py-codebase-memory` checkout:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable ./python \
  python <inline May 21 three-mode ingestion benchmark>  # repeats=7
```

Input shape: all 16 Python files under `py-codebase-memory/src/codebase_memory`,
114240 bytes total, parsed repeatedly with `PythonIndexer`.

| Surface | Mode | Latest result | Date | Notes |
| --- | --- | --- | --- | --- |
| Python code-memory ingestion | local tree-sitter + heuristic token estimate | 83.91 ms median, 1165.2 KiB median peak Python allocation, 141 chunks | 2026-05-22 | Same local chunking/token heuristic baseline shape as the May 21 proof. |
| Python code-memory ingestion | local tree-sitter + cached `fi_accel.count_tokens` | 103.38 ms median, 1165.1 KiB median peak Python allocation, 141 chunks | 2026-05-22 | Replaces the previous 4152.80 ms exact-count path; the wall-time blocker is reduced to about 19.5 ms over heuristic counting. |
| Python code-memory ingestion | `fi_accel.chunk_code` + cached token counts | 31.15 ms median, 158.3 KiB median peak Python allocation, 143 chunks | 2026-05-22 | Replaces the previous 4199.62 ms chunker path and is now faster than local, but chunk cardinality still differs from the downstream contract. |

Decision: revisit default adoption for exact token counting with local
`PythonIndexer` chunking. The wall-time gap closed enough that the remaining
tradeoff is exact-count value versus roughly 20 ms median overhead on this input
set. Keep full `fi_accel.chunk_code` default adoption blocked on chunk
identity/cardinality parity, despite the faster wall time and lower Python
allocation.

### Full Helper Ingestion-Record Prototype

Run from this repo against the current `py-codebase-memory` checkout:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable ./python \
  python scripts/bench_code_memory_ingestion.py --repeats 7
```

Input shape: all 16 Python files under `py-codebase-memory/src/codebase_memory`,
114240 bytes total. Baseline uses the local `PythonIndexer`, `read_text`,
`hashlib.sha256`, and Python ranking glue. Prototype uses `fi_accel.detect_encoding`,
`fi_accel.chunk_code`, `fi_accel.sha256_batch`, and `fi_accel.hybrid_rank` to build
rank-ready ingestion records.

| Surface | Mode | Latest result | Date | Notes |
| --- | --- | --- | --- | --- |
| Python code-memory ingestion-record build | local tree-sitter + stdlib hash/ranking glue | 90.48 ms median, 1516.4 KiB median peak Python allocation, 141 chunks | 2026-05-22 | Baseline record preparation path for the same source tree. |
| Python code-memory ingestion-record build | `fi-accel` decode/chunk/hash/rank helpers | 20.52 ms median, 913.5 KiB median peak Python allocation, 143 chunks | 2026-05-22 | Clears the prototype wall-time/allocation gate for record construction, but chunk cardinality and ranked IDs differ from local. |

Decision: promote this as a prototype recipe only. The full helper path is about
4.4x faster for ingestion-record construction and uses about 40% less peak
Python allocation, but it should not become the default `py-codebase-memory`
path until chunk identity/cardinality parity is either fixed or accepted by the
downstream index contract. The earlier per-chunk `fi_accel.count_tokens` path
remains blocked for default adoption.

### Chunk Schema Parity Fixture

Run from this repo against the current `py-codebase-memory` checkout:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable ./python \
  python scripts/check_code_memory_chunk_parity.py
```

Input shape: all 16 Python files under `py-codebase-memory/src/codebase_memory`,
compared between local `PythonIndexer` chunking and `fi_accel.chunk_code`.

| Surface | Compared fields | Latest result | Date | Notes |
| --- | --- | --- | --- | --- |
| Python code-memory chunk schema parity | chunk count, id, type, file path, line/column spans, name, parent name/type, calls, definitions | PASS: 141 local chunks, 141 `fi_accel` chunks, `first_divergence: null` | 2026-05-23 | Aligns code chunk IDs with the downstream zero-based hash input while keeping public line spans one-based; Python files with no semantic class/function chunks now match local `PythonIndexer` cardinality. |

Decision: the chunk identity/cardinality blocker for the 16-file
`py-codebase-memory/src/codebase_memory` fixture is closed. Default downstream
adoption still needs the token-counting path decision from the cached tokenizer
follow-up.

### Batched Exact Token Count Follow-Up

Narrow fix chosen: add an additive Python `count_tokens_batch` API backed by a
single Rust tokenizer setup, plus `Tokenizer.count_batch`, so Python consumers
can keep their current chunking contracts while removing one PyO3 call per
chunk. The full `fi_accel.chunk_code(..., model="gpt-4")` path also reuses the
chunker-provided exact `token_count` values for comparison now that chunk
schema parity is closed on this fixture.

Run from this repo against the current `py-codebase-memory` checkout:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable ./python \
  python scripts/bench_code_memory_token_counts.py --repeats 7
```

Input shape: all 16 Python files under `py-codebase-memory/src/codebase_memory`,
114240 bytes total, parsed repeatedly with `PythonIndexer` for the local modes.

| Surface | Mode | Latest result | Date | Notes |
| --- | --- | --- | --- | --- |
| Python code-memory ingestion | local tree-sitter + heuristic token estimate | 88.96 ms median, 1489.2 KiB median peak Python allocation, 141 chunks, 26232 estimated tokens | 2026-05-23 | Same local chunking/token heuristic fixture shape as the prior FA-PLAN-06 proofs. |
| Python code-memory ingestion | local tree-sitter + batched `fi_accel.count_tokens_batch` | 95.54 ms median, 1236.2 KiB median peak Python allocation, 141 chunks, 22117 exact tokens | 2026-05-23 | Removes per-chunk Python/PyO3 token-count calls; exact counting adds about 6.6 ms median over the heuristic path on this input set. |
| Python code-memory ingestion | `fi_accel.chunk_code(model="gpt-4")` with chunker token counts | 26.70 ms median, 706.3 KiB median peak Python allocation, 141 chunks, 22117 exact tokens | 2026-05-23 | Reuses chunker-provided exact token counts after the chunk parity fixture closed; validates the fast full-helper path separately from local-contract adoption. |

Validation:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable ./python \
  python scripts/check_code_memory_chunk_parity.py
```

Result: PASS with 141 local chunks, 141 `fi_accel` chunks, and
`first_divergence: null`.

Decision: local `PythonIndexer` chunking plus exact `fi_accel` token counts is
now defaultable for this fixture when downstream switches from per-chunk
`count_tokens` calls to `count_tokens_batch`. Full `fi_accel.chunk_code` with
chunker-provided exact counts also clears the local wall-time/allocation and
chunk-parity gates on this fixture, but the downstream default flip should still
land as an explicit `py-codebase-memory` integration change.
