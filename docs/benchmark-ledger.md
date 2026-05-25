# Benchmark Ledger

This ledger is the source of truth for performance evidence behind promoted
`fi-accel` claims. Release notes should link here or use conservative wording
unless a surface has a dated command, host, input shape, and result.

## README Claim Policy

The README performance section links to this ledger and summarizes only dated
measurements that include a command, input shape, host context, and result.
Surfaces without ledger evidence should be described as available APIs, not as
speedups.

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
| Hashing | `icc_release_gate/sha256_batch_100k_256b` | 100K byte strings, 256 bytes each | 13.374-15.802 ms | 2026-05-23 | Backs FA-ICC-01 batch-hash claims. Compare against Python `hashlib` loop before claiming a multiplier. |
| Ranking | `icc_release_gate/hybrid_rank_1k` | 1K lexical ranks plus 1K cosine scores | 13.533-13.619 us | 2026-05-23 | Backs FA-ICC-03 hybrid-rank overhead claims. |
| Ranking | `icc_release_gate/rrf_fuse_3x1k` | Three ranked lists, 1K IDs each | 121.78-124.41 us | 2026-05-23 | Backs FA-ICC-09 RRF helper claims. |
| NDJSON | `icc_release_gate/ndjson_reader_append_1k` | 1K JSONL events appended in one chunk | 526.12-557.48 us | 2026-05-23 | Backs FA-ICC-06 streaming-reader claims. |
| HTML to text | `icc_release_gate/html_to_text_250_fragments` | 250 repeated email/article-style HTML fragments with entities and links | 33.547-33.932 ms | 2026-05-23 | Backs FA-ICC-02 release proof for native HTML extraction. No multiplier claim yet. |
| Encoding detection | `icc_release_gate/detect_encoding_windows1252_1k_lines` | 1K short artifact lines containing Windows-1252 smart-quote bytes | 7.1590-7.2486 us | 2026-05-23 | Backs FA-ICC-05 release proof for legacy Western text detection. |
| Pattern matching | `icc_release_gate/literal_pattern_128_over_1k_lines` | 128 literals over 1K text lines | 1.1741-1.1913 ms | 2026-05-23 | Backs FA-ICC-07 pattern-count scaling claims. |
| Pattern matching | `icc_release_gate/regex_pattern_2_over_1k_lines` | Two regexes over 1K text lines | 70.535-73.217 us | 2026-05-23 | Tracks regex mode separately from literal matching. |
| Mail parsing | `icc_release_gate/parse_eml_multipart_attachment` | Multipart EML with text, HTML, and one attachment | 3.2063-3.2880 us | 2026-05-23 | Backs FA-ICC-08 parser overhead claims. |

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
lower Python allocation matters more than wall time. Superseded for local
`PythonIndexer` exact token counting by the May 23 post-parity gate below;
full `fi_accel.chunk_code` default adoption remains separate.

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
was still blocked for default adoption at this point; the May 23 post-parity
token gate below supersedes that token-counting blocker for local
`PythonIndexer`.

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
`py-codebase-memory/src/codebase_memory` fixture is closed. At this point,
default downstream adoption still needed the token-counting path decision that
was later settled by the batched exact-token follow-up below.

### Batched Exact Token Count Follow-Up

Narrow fix chosen: add an additive Python `count_tokens_batch` API backed by a
single Rust tokenizer setup, plus `Tokenizer.count_batch`, so Python consumers
can keep their current chunking contracts while removing one PyO3 call per
chunk. The full `fi_accel.chunk_code(..., model="gpt-4")` path also reuses the
chunker-provided exact `token_count` values for comparison now that chunk
schema parity is closed on this fixture.

This is the current tokenizer API decision after the May 23 merge
(`62c6ca9`, including `8a4a705`): keep the merged batch API. The benchmark
below supersedes stale local notes that proposed rejecting the branch after a
cached scalar-only rerun, because the merged batch path preserves the local
chunk contract while reducing exact-count overhead and Python allocation.

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

Downstream integration result: on 2026-05-25, `py-codebase-memory` switched
local `PythonIndexer` chunk token assignment from per-chunk scalar exact counts
to `fi_accel.count_tokens_batch` behind the existing exact-token mode. The
benchmark script now measures that downstream path directly rather than counting
local chunks externally.

Run from this repo against `/Users/cblevins/workspace/libs/py-codebase-memory`:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable ./python \
  python scripts/bench_code_memory_token_counts.py --repeats 7
```

Input shape: all 16 Python files under `py-codebase-memory/src/codebase_memory`,
116081 bytes total, parsed repeatedly with `PythonIndexer` for the local modes.

| Surface | Mode | Latest result | Date | Notes |
| --- | --- | --- | --- | --- |
| Python code-memory ingestion | local tree-sitter + heuristic token estimate | 107.91 ms median, 1498.1 KiB median peak Python allocation, 144 chunks, 26663 estimated tokens | 2026-05-25 | Current downstream fixture after the local batch-token integration. |
| Python code-memory ingestion | local tree-sitter + downstream batched `fi_accel.count_tokens_batch` | 93.99 ms median, 1497.1 KiB median peak Python allocation, 144 chunks, 22504 exact tokens | 2026-05-25 | `PythonIndexer` now assigns exact local chunk token counts through the batched API; the planned downstream follow-up is landed for exact-token mode. |
| Python code-memory ingestion | `fi_accel.chunk_code(model="gpt-4")` with chunker token counts | 26.96 ms median, 720.1 KiB median peak Python allocation, 144 chunks, 22504 exact tokens | 2026-05-25 | Full-helper path remains a separate default-adoption decision, but it still matches local exact token totals on this fixture. |

Parity validation:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable ./python \
  python scripts/check_code_memory_chunk_parity.py
```

Result: PASS with 144 local chunks, 144 `fi_accel` chunks, and
`first_divergence: null`.

### Scalar Token Gate Rerun (Historical)

A local scalar-only rerun on 2026-05-23 also showed that cached scalar
`fi_accel.count_tokens` could clear the 25% wall-time gate after chunk parity
closed, but its API conclusion is superseded by the merged batched exact-token
follow-up above.

Command shape:

```bash
uv run --project /Users/cblevins/workspace/libs/py-codebase-memory \
  --with-editable /Users/cblevins/workspace/libs/fi-accel/python \
  python scripts/bench_code_memory_token_gate.py --repeats 7
```

| Surface | Mode | Latest result | Date | Notes |
| --- | --- | --- | --- | --- |
| Python code-memory ingestion | local tree-sitter + heuristic token estimate | 85.53 ms median, 83.38-96.05 ms range, 1489.4 KiB median peak Python allocation, 141 chunks | 2026-05-23 | Scalar-only rerun baseline. |
| Python code-memory ingestion | local tree-sitter + cached `fi_accel.count_tokens` | 93.65 ms median, 93.08-140.52 ms range, 1488.8 KiB median peak Python allocation, 141 chunks | 2026-05-23 | Cleared the wall-time gate, but did not reduce Python allocation the way the batched path later did. |
| Python code-memory ingestion | `fi_accel.chunk_code` + cached token counts | 29.09 ms median, 28.76-29.82 ms range, 617.4 KiB median peak Python allocation, 141 chunks | 2026-05-23 | Historical comparison point before the merged batched-count benchmark. |

Decision: keep this as historical evidence only. The current API guidance is to
use `count_tokens_batch` for downstream local-chunk exact token counting; the
`py-codebase-memory` exact-token mode now does this for local `PythonIndexer`
chunks.

## WASM Tokenizer And Code-Chunking Kill-Test

Run from a temporary copy of this repo on 2026-05-23:

```bash
wasm-pack build crates/fi-accel-wasm --target web --release --out-dir <out>
wasm-pack build crates/fi-accel-wasm --target bundler --release --out-dir <out>
wasm-pack build crates/fi-accel-wasm --target web --release --out-dir <out> --features tokenizer
wasm-pack build crates/fi-accel-wasm --target bundler --release --out-dir <out> --features tokenizer
wasm-pack build crates/fi-accel-wasm --target web --release --out-dir <out> --features chunker-code
```

Prototype shape: temporary WASM crate features forwarded `tokenizer` to
`fi-accel-core/tokenizer` and `chunker-code` to `fi-accel-core/chunker`. The
tokenizer prototype exported `count_tokens`, `truncate_to_tokens`, and
`split_by_tokens`; the code-chunking prototype exported `chunk_code`.

Host/runtime: Apple M4 local development machine, `rustc 1.92.0`, `cargo
1.92.0`, `wasm-pack 0.13.1`, Node `v26.0.0`. Size rows use the `web` and
`bundler` release outputs after wasm-pack's wasm-opt pass. Init rows use the
`web` target, dynamic ESM import with a cache-busting URL, and
`initSync({ module: bytes })` over 31 repeats.

| Surface | Variant | Bundle/build result | Init result | Date | Notes |
| --- | --- | --- | --- | --- | --- |
| WASM current boundary | baseline, tokenizer/code chunking unsupported | `fi_accel_wasm_bg.wasm` 2080517 bytes raw, 771710 bytes gzip; web JS 42906 bytes; bundler JS total 39015 bytes | 0.405 ms median `initSync`; 0.752 ms median import plus init | 2026-05-23 | Existing package with markdown/text chunking, transport, eventlog, TOML/TOON, JSON, diff, log, and embedding helpers. |
| WASM tokenizer prototype | add `tiktoken-rs` tokenizer exports | `fi_accel_wasm_bg.wasm` 9366845 bytes raw, 4097317 bytes gzip; web JS 45348 bytes; bundler JS total 41508 bytes | 1.944 ms median `initSync`; 2.698 ms median import plus init | 2026-05-23 | Adds 7286328 raw bytes and 3325607 gzip bytes versus baseline. Smoke check: `count_tokens("hello world", "gpt-4") == 2`. |
| WASM code-chunking prototype | add Rust tree-sitter chunker exports | build failed for `wasm32-unknown-unknown` | not measured | 2026-05-23 | Current tree-sitter crates invoke C compilation for the browser target and fail on missing libc headers such as `stdlib.h` and `stdio.h`. |
| WASM tokenizer plus code-chunking prototype | add both prototype features | build failed for `wasm32-unknown-unknown` | not measured | 2026-05-23 | Fails for the same tree-sitter C build reason before package-size/init measurements are available. |

Decision: keep the current WASM unsupported boundary for tokenizer and code
chunking. Tokenizer support is technically viable but deferred because it turns
the `.wasm` from about 2.0 MB to 8.9 MB raw and from about 0.74 MiB to 3.91 MiB
gzip without a named browser consumer that needs exact tokenization at startup.
If a consumer appears, implement it as an explicit opt-in package or feature,
not in the default `@flexinfer/fi-accel` WASM bundle. Rust-core tree-sitter
code chunking is permanently unsupported in `fi-accel-wasm` for the current
Rust chunker path; any future browser code parser should be a separate
parser-WASM design, not a direct promotion of the current Rust tree-sitter
chunker path.
