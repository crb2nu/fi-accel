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
go test ./... -run '^$' -bench 'BenchmarkParseJSONRPCRoutePreviewGatewayToolsCall' -benchmem -count=5
```

| Surface | Benchmark | Input shape | Latest result | Date | Notes |
| --- | --- | --- | --- | --- | --- |
| Go transport | `BenchmarkParseJSONRPCRoutePreviewGatewayToolsCall` | production-shaped MCP `tools/call` route payload | 5123-5436 ns/op, 1744 B/op, 22 allocs/op | 2026-05-20 | Standalone helper result from `docs/service-acceleration-kill-test-2026-05-20.md`. |

Downstream `fi-mcp-gateway` service adoption did not clear the gate on
2026-05-20: the helper path reached approximate wall-time parity but used
slightly more bytes and allocations than the service's current route path.

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
