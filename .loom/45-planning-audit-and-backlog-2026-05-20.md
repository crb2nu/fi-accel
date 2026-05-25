# Planning Audit And Backlog (2026-05-20)

## Summary

The 2026-05-20 service-acceleration brainstorm has been mostly implemented inside `fi-accel`. The recommended F6 + F1 path produced a scout baseline, an empirical service kill-test, a pure Go JSON-RPC route-preview helper, arguments exposure, allocation reduction, and production-shaped route-preview fixtures. The remaining unresolved item is the actual downstream `fi-mcp-gateway` rerun and conditional service integration. Sources: `.loom/brainstorm-service-acceleration-features-2026-05-20.md:81`, `docs/service-acceleration-kill-test-2026-05-20.md:89`, `docs/service-acceleration-kill-test-2026-05-20.md:172`, `go/fiaccel/transport.go:57`, `go/fiaccel/transport_route_preview_test.go:137`.

The older implementation plan and ICC backlog also lag the current code surface. README now lists ranking, text/encoding ingestion, eventlog, acceleration scout, transport, chunking, and capability reporting as promoted surfaces. The Python binding exports ranking, hashing, text/encoding, chunking, pattern matching, mail parsing, and fallback-aware capabilities. Sources: `README.md:7`, `README.md:42`, `crates/fi-accel-py/src/lib.rs:944`, `crates/fi-accel-py/src/lib.rs:970`.

May 21 update: FA-PLAN-06 now has downstream evidence. The merged `py-codebase-memory` adoption proof (`dab7b76`, including `08200dd feat(indexers): gate fi-accel adoption proof`) kept both `fi_accel.chunk_code` and `fi_accel.count_tokens` behind explicit opt-in environment modes because the accelerated code-memory path missed the wall-time gate. May 23 update: after the cached tokenizer fast path, chunk parity merge (`6fbe9f3`, including `cb2fe16`), and FA-PLAN-06B batch-token merge (`62c6ca9`, including `8a4a705`), local `PythonIndexer` plus batched exact `fi_accel.count_tokens_batch` clears the downstream default gate at 95.54 ms median versus 88.96 ms heuristic baseline with lower peak Python allocation. Promote exact token counting as the defaultable token-counting path for this fixture once downstream consumes the batch API; treat full `fi_accel.chunk_code` adoption as a separate contract decision.

## Brainstorm Implementation Status

| Prior framing | Status | Evidence | Remaining backlog |
| --- | --- | --- | --- |
| F6 - Acceleration Profiler And Adoption Scout | Mostly done | Scout surface appears in README and Python/core exports; the first kill-test was run and documented. Sources: `README.md:17`, `crates/fi-accel-py/src/lib.rs:1008`, `docs/service-acceleration-kill-test-2026-05-20.md:1`. | Keep scout useful by turning service reruns into a repeatable benchmark recipe. |
| F1 - Service Hot Path SDKs | Partial | The Go route-preview helper is a service-facing drop-in candidate with method, id, `params.name`, raw arguments, bytes, status, and validity. Sources: `go/fiaccel/transport.go:57`, `go/fiaccel/transport.go:87`. | Rerun and, if justified, land the `fi-mcp-gateway` integration. |
| F2 - MCP Transport Fast Lane | Mostly done for current scope | Transport is full in the README matrix; Go tests cover route-preview semantics and gateway-shaped payload benchmarks. Sources: `README.md:57`, `go/fiaccel/transport_route_preview_test.go:14`, `go/fiaccel/transport_route_preview_test.go:137`. | Add downstream conformance and payload-mix benchmarks before expanding policy prechecks. |
| F3 - Retrieval Ingestion Pipeline | Exact token default cleared; full chunker still separate | Chunking metadata, markdown/text chunking, hashing, ranking, text extraction, and encoding detection exist. The May 23 batch-token rerun keeps local `PythonIndexer` chunking and shows batched exact `fi_accel.count_tokens_batch` at 95.54 ms median versus 88.96 ms heuristic baseline, clearing the 25% gate while lowering peak Python allocation. Sources: `README.md`, `crates/fi-accel-py/src/lib.rs`, `docs/benchmark-ledger.md`. | Promote exact token counting for the local PythonIndexer path through `count_tokens_batch`. Revisit full `fi_accel.chunk_code` default adoption only as a separate downstream contract decision. |
| F4 - Patch And Repair Accelerator | Deferred | Fuzzy diff remains present, but this brainstorm did not drive new patch-risk APIs. Source: `README.md:12`. | Revisit after gateway and ingestion adoption gates. |
| F5 - FlexInfer Scheduling Data Plane | Deferred | No scheduling-specific helper appears in the current surface. Source: `README.md:7`. | Keep as future consumer-specific discovery, not near-term work. |
| F7 - Browser/WASM Local Acceleration | Boundary decided | WASM has local JSON/eventlog/TOML/TOON/chunk markdown/text surfaces. The May 23 bundle kill-test defers tokenizer to opt-in packaging and makes Rust tree-sitter code chunking permanently unsupported for the current WASM binding path. Sources: `README.md`, `docs/benchmark-ledger.md`. | Keep the default WASM package focused on lightweight UI helpers; reopen tokenizer only with a named consumer budget. |
| F8 - Shared Batch Interchange | Open | NumPy/Arrow interop remains in the main plan but is not visible as a promoted surface. Sources: `.loom/30-implementation-plan.md:63`, `.loom/30-implementation-plan.md:68`. | Keep behind a named consumer and benchmark gate. |

## ICC Backlog Status

| Ticket | Status | Evidence | Follow-up |
| --- | --- | --- | --- |
| FA-ICC-01 batch SHA-256 and stream hashing | Mostly done | `sha256_batch` and hash-chain verification are exported natively; `sha256_stream` exists in Python package code. Sources: `crates/fi-accel-py/src/lib.rs:1010`, `python/fi_accel/fallback.py:498`. | Add or refresh the 100K x 256-byte benchmark claim before release notes call it complete. |
| FA-ICC-02 HTML to text | Done, benchmark-backed | Native and fallback functions exist, and `icc_release_gate/html_to_text_250_fragments` measures 33.547-33.932 ms over 250 repeated email/article-style fragments. Sources: `crates/fi-accel-py/src/lib.rs:572`, `python/fi_accel/fallback.py:541`, `docs/benchmark-ledger.md`. | Add consumer example if ICC adopts it; no multiplier claim until compared with an ICC/Python baseline. |
| FA-ICC-03 hybrid rank | Done | Native and fallback ranking surfaces are exported and tested. Sources: `crates/fi-accel-py/src/lib.rs:987`, `python/fi_accel/fallback.py:139`, `python/tests/test_ranking.py:1`. | Consider Go/WASM parity only if a consumer asks. |
| FA-ICC-04 Gemma tokenizer | Partial | Gemma aliases require optional `gemma-tokenizer` and a local tokenizer asset; the code refuses to approximate with GPT tokenizers. Sources: `README.md:162`, `crates/fi-accel-core/src/tokenizer.rs:1`, `crates/fi-accel-core/src/tokenizer.rs:137`. | Add a fixture-backed exactness test path when a local tokenizer asset is available. |
| FA-ICC-05 encoding detection | Done, benchmark-backed | Native and fallback encoding detection exist, and `icc_release_gate/detect_encoding_windows1252_1k_lines` measures 7.1590-7.2486 us over 1K legacy Western text lines. Sources: `crates/fi-accel-py/src/lib.rs:579`, `python/fi_accel/fallback.py:557`, `docs/benchmark-ledger.md`. | No multiplier claim until compared with an ICC/Python baseline. |
| FA-ICC-06 streaming NDJSON reader | Done | `NDJSONReader` is exported natively and in fallback. Sources: `crates/fi-accel-py/src/lib.rs:999`, `python/fi_accel/fallback.py:267`. | Add skip-bad-line mode only if a consumer needs it. |
| FA-ICC-07 multi-pattern matcher | Done | Native and fallback classes exist. Sources: `crates/fi-accel-py/src/lib.rs:891`, `python/fi_accel/fallback.py:732`. | Benchmark pattern-count scaling before claiming SIMD-style wins. |
| FA-ICC-08 EML / mbox parser | Done | Native and fallback parsers exist. Sources: `crates/fi-accel-py/src/lib.rs:926`, `python/fi_accel/fallback.py:752`. | Confirm attachment memory behavior against ICC fixtures. |
| FA-ICC-09 RRF fuse | Done | Native/fallback surfaces are exported and tested. Sources: `crates/fi-accel-py/src/lib.rs:987`, `python/fi_accel/fallback.py:173`, `python/tests/test_ranking.py:1`. | None. |
| FA-ICC-10 hash-chain verifier | Done | Native and fallback verification exist, and the May 23 service-shaped audit-chain fixture covers valid, tampered-payload, and broken-link diagnostics. Sources: `crates/fi-accel-py/src/lib.rs:540`, `python/fi_accel/fallback.py:502`, `python/tests/test_icc_release_gate.py`. | None for the current fixture; add broader ICC fixtures only if the service contract changes. |
| FA-ICC-11 pure-Python fallback | Mostly done | Fallback module exists and reports `fallback: true`; tests cover fallback import. Sources: `python/fi_accel/fallback.py:1`, `python/fi_accel/fallback.py:785`, `python/tests/test_fallback.py:1`. | Expand parity coverage for every promoted native function before a release. |

## New Backlog

Priority key: P1 = next unblocker, P2 = high-value hardening, P3 = strategic follow-up.

### FA-PLAN-01 - Gateway Production Payload Rerun (P1) - Done, Gate Failed

Run `fi-mcp-gateway` route benchmarks using the production-shaped payload from `go/fiaccel/transport_route_preview_test.go:12` and the latest allocation-reduced helper.

Acceptance:
- Done: command and results were appended to `docs/service-acceleration-kill-test-2026-05-20.md`.
- Done: result compares wall time, B/op, and allocs/op against the current production-shaped gateway route path.
- Decision: stop the direct service integration path for now. The `fi-accel` candidate reached approximate wall-time parity but used slightly more bytes and allocations.

### FA-PLAN-02 - Conditional Gateway Integration (P1) - Skipped

If FA-PLAN-01 clears the documented gate, land the smallest `fi-mcp-gateway` integration using `ParseJSONRPCRoutePreview`.

Acceptance:
- Skipped because FA-PLAN-01 did not clear the benchmark gate.
- Reopen only if `fi-accel` gains a more specialized route extractor that materially reduces decode/copy work.

### FA-PLAN-03 - Plan/Spec Reconciliation (P1) - Done

Update the main product spec and implementation plan so completed surfaces are not described as future work.

Acceptance:
- Done: `.loom/20-product-spec.md` reflects current promoted surfaces and remaining gaps.
- Done: `.loom/30-implementation-plan.md` references this audit, the post-route brainstorm, and the failed gateway gate.
- README matrix and plan agree on major support gaps.

### FA-PLAN-04 - ICC Utility Release Gate (P1)

Turn the implemented ICC utility backlog into a release checklist with tests, benchmarks, and consumer examples.

Acceptance:
- Each FA-ICC item has one native test or fallback parity test, and known exceptions are documented.
- Benchmark-backed claims exist for hashing, ranking, HTML-to-text extraction, encoding detection, NDJSON reader, pattern matching, and mail parsing before release notes mention speed.
- ICC can import the fallback module without local guard code.
- Remaining boundary: Gemma exactness still needs a local tokenizer asset fixture, and fallback parity breadth remains a release checklist item.

### FA-PLAN-05 - Gemma Fixture Harness (P1)

Add an opt-in Gemma tokenizer fixture test that runs only when `FI_ACCEL_GEMMA_TOKENIZER_PATH` points at a local Hugging Face `tokenizer.json`.

Acceptance:
- Test confirms exact token counts on a 100-string fixture.
- Test is skipped, not failed, when the asset is absent.
- README documents the skip behavior and asset path.

### FA-PLAN-06 - Code-Memory Ingestion Proof (P2) - Done, Initial Gate Failed

Use `fi-accel` chunk/text/hash/ranking helpers in one code-memory or indexing path and record the before/after numbers.

Acceptance:
- Done: the downstream `py-codebase-memory` proof merged as `dab7b76`.
- Done: `py-codebase-memory` exposes `CODEBASE_MEMORY_PYTHON_CHUNKER_MODE=fi-accel` and `CODEBASE_MEMORY_TOKEN_ESTIMATE_MODE=auto`, but keeps defaults as local chunking plus heuristic token estimates.
- Done: result documents latency, allocation behavior, and schema compatibility tradeoffs.
- Initial decision: do not promote downstream chunking/token counting as defaults. The code-memory path missed the wall-time gate; `fi_accel.chunk_code` reduced Python allocation but changed chunk cardinality, and per-chunk exact token counting dominated wall time.
- May 23 update: the initial token-counting blocker is closed by cached `fi_accel.count_tokens`; local `PythonIndexer` plus exact counts clears the 25% wall-time gate at about 9.5% overhead. Promote exact token counting as the default token path for this fixture. Full `fi_accel.chunk_code` default adoption remains a separate contract decision.

### FA-PLAN-06B - Repeated Token Counting Fast Path (P2) - Done, Gate Passed

Remove the narrow `fi-accel` blocker exposed by FA-PLAN-06: repeated `count_tokens` calls over small/medium chunks should reuse tokenizer state behind the existing API.

Acceptance:
- Implement without changing the Python or Rust public API; `fi_accel.count_tokens(text, model="gpt-4")` and `Tokenizer(model).count(text)` keep their current behavior.
- Cache/reuse tiktoken encoding state per model or encoding in the Rust core, with safe fallback/error behavior for unknown models and existing Gemma asset rules unchanged.
- Add targeted Rust tests or Python smoke coverage showing repeated calls return the same counts and do not rebuild model state where observable.
- Rerun the May 21 `py-codebase-memory` benchmark shape before promotion:
  - repo: `/Users/cblevins/workspace/libs/py-codebase-memory`
  - input: all Python files under `src/codebase_memory`
  - repeats: at least 7
  - modes: local tree-sitter + heuristic token estimate; local tree-sitter + `fi_accel.count_tokens`; `fi_accel.chunk_code` + `fi_accel.count_tokens`
  - report: median wall time, min/max wall time, peak Python allocation, file count, chunk count, and whether chunk cardinality matches local
- Done: May 23 rerun reports local heuristic at 85.53 ms median and local `PythonIndexer` plus cached `fi_accel.count_tokens` at 93.65 ms median, about 9.5% overhead. Parity also passes on the 16-file fixture with 141 local chunks and 141 `fi_accel` chunks.
- Gate: passed for exact token counting. Promote local `PythonIndexer` plus cached `fi_accel.count_tokens` as the downstream default token-counting path for this fixture. Full `fi_accel.chunk_code` remains a separate default-adoption decision even though this fixture now has chunk-count parity.
- Follow-up branch decision: reject `codex/fa-plan-06-token-count-overhead`'s public `count_tokens_batch` / `Tokenizer.count_batch` API. Keep `scripts/bench_code_memory_token_gate.py` as the repeatable benchmark artifact using existing `count_tokens`.

### FA-PLAN-06A - Specialized Route Extractor Kill-Test (P2)

Only revisit gateway routing if the candidate changes materially from the current Go preview helper.

Acceptance:
- Prototype avoids at least one downstream `encoding/json` object allocation path or avoids decoding `params.arguments` into a map when prefix routing is enough.
- Benchmark uses the same production-shaped payload and compares against `BenchmarkRouteMessageGatewayToolsCall`.
- Integration remains blocked unless it clears the original wall-time or allocation gate.

### FA-PLAN-07 - Go Parity Decision (P2)

Decide whether Go should expose ranking, hashing, text/encoding, and pattern/mail helpers, or intentionally remain transport/chunking focused.

Acceptance:
- Decision added to `.loom/40-decisions.md`.
- If accepted, split into one small ticket per surface.
- If rejected, README and capability docs explain the boundary.

### FA-PLAN-08 - WASM Tokenizer And Code-Chunking Kill-Test (P2) - Done

Measure bundle-size and init-time impact for adding tokenizer and/or code chunking to WASM.

Acceptance:
- Done: `docs/benchmark-ledger.md` records size, gzip size, Node `v26.0.0` init latency, and the browser-target build result for current, tokenizer, code-chunking, and combined prototypes.
- Done: tokenizer is deferred to a future opt-in package/feature with a named consumer because it adds about 3.33 MB gzip to the `.wasm` payload; Rust-core tree-sitter code chunking is permanently unsupported in `fi-accel-wasm` for the current Rust chunker path because the `wasm32-unknown-unknown` build fails on tree-sitter C/libc headers.

### FA-PLAN-09 - Benchmark Ledger (P2) - Done

Create a compact benchmark ledger for promoted surfaces so future README claims have one source of truth.

Acceptance:
- Done: ledger records command, host, date, input shape, and result tables for current promoted benchmark evidence.
- Done: README performance claims link to the ledger and summarize only dated, ledger-backed measurements.
- Done: Python package prose no longer advertises unbacked speed multipliers.

### FA-PLAN-10 - Patch Repair Revisit (P3)

Revisit the patch accelerator framing after gateway and ingestion adoption gates close.

Acceptance:
- Decide whether `diff-surgeon` has a concrete helper path worth promoting.
- If yes, write a separate kill-test and implementation slice.

## MCP / Index State

- Loom context templates were refreshed with `python /Users/cblevins/.codex/skills/plan-loom-core/scripts/init_loom_context.py --root .`.
- Workspace snapshot was refreshed with `python /Users/cblevins/.codex/skills/plan-loom-core/scripts/workspace_snapshot.py --root .`.
- Codebase-memory stats for `repo_id=fi-accel` returned `total_chunks: 388` with Rust, Go, and Python chunks available.
