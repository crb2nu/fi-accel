# Decisions

Record decisions as they are made, with date, rationale, and sources.

## Template

### YYYY-MM-DD: Decision title

- Decision:
- Rationale:
- Alternatives considered:
- Consequences:
- Sources:
  - [S1] …

### 2026-03-08: Prioritize workspace-fit API parity before novel new domains

- Decision:
  - Treat cross-language parity, MCP transport helpers, and chunking/indexing usefulness as the next planning priority for `fi-accel`.
- Rationale:
  - The surrounding workspace is dominated by MCP servers, orchestration, semantic indexing, and resilient patch flows. The repo already contains partial support for these domains, but the bindings and docs drift enough that the library is not yet a dependable shared substrate.
- Alternatives considered:
  - Focus next on deeper Rust-only numerical features or TOON expansion.
- Consequences:
  - Near-term work will skew toward reusable integration primitives and API discipline rather than only micro-optimizations.
- Sources:
  - [S1] `/Users/cblevins/workspace/services/loom-core/README.md:63-71`
  - [S2] `/Users/cblevins/workspace/services/mcp-orchestra/README.md:40-49`
  - [S3] `/Users/cblevins/workspace/services/diff-surgeon/README.md:17-25`
  - [S4] `/Users/cblevins/workspace/libs/py-codebase-memory/README.md:16-24`
  - [S5] `crates/fi-accel-py/src/lib.rs:450-552`
  - [S6] `go/fiaccel/tokenizer.go:3-27`

### 2026-03-08: Keep Arrow and NumPy interop optional until benchmark-backed

- Decision:
  - Plan NumPy- and Arrow-based interop as an optional track, not a mandatory foundation for all targets.
- Rationale:
  - The official ecosystems support strong zero-copy and vector-friendly patterns, but they also add packaging and compatibility complexity. The workspace value is plausible, but not yet proven enough to justify a mandatory dependency path.
- Alternatives considered:
  - Make Arrow or NumPy interop a core requirement of the next release.
- Consequences:
  - Feature flags and benchmarks become part of the acceptance criteria for any interop work.
- Sources:
  - [S1] <https://docs.rs/numpy/latest/numpy/> (accessed 2026-03-08 via Tavily extract)
  - [S2] <https://arrow.apache.org/docs/format/Columnar.html> (accessed 2026-03-08 via Tavily extract)
  - [S3] `python/pyproject.toml:1-39`
  - [S4] `Cargo.toml:17-52`

### 2026-03-14: Treat runtime introspection parity as the next M1 delivery slice

- Decision:
  - Close the next M1 increment by standardizing binding-level runtime introspection around version and capability helpers before expanding into new module families.
- Rationale:
  - The capability matrix and transport/chunking work were already documented, but the runtime API story still drifted by binding. Finishing version/capabilities parity improves adoption confidence with a small additive change and keeps the README/examples honest.
- Alternatives considered:
  - Start M2 transport expansion immediately.
  - Skip runtime version helpers and rely only on package metadata.
- Consequences:
  - Python and Go gain explicit runtime version helpers, and milestone progress can be tracked against a clearer parity baseline.
- Sources:
  - [S1] `.loom/20-product-spec.md`
  - [S2] `.loom/30-implementation-plan.md`
  - [S3] `README.md`
  - [S4] `python/fi_accel/__init__.py`

### 2026-05-20: Optimize Go route preview before service promotion

- Decision:
  - Reduce `ParseJSONRPCRoutePreview` allocations in `fi-accel` before merging downstream `fi-mcp-gateway` integration.
- Rationale:
  - The gateway integration probe improved wall time but increased allocations and did not reliably clear the 20% kill-test threshold. A small additive helper optimization improves the shared library without committing a service to a marginal integration.
- Alternatives considered:
  - Merge the gateway integration immediately based on the lower wall-time range.
  - Replace the helper with an FFI-backed parser in this slice.
- Consequences:
  - The helper remains pure Go and usable with `CGO_ENABLED=0`; downstream adoption remains gated on a fresh service benchmark.
- Sources:
  - [S1] `.loom/38-iteration-plan-route-preview-arguments-2026-05-20.md`
  - [S2] `docs/service-acceleration-kill-test-2026-05-20.md`
  - [S3] `go/fiaccel/transport.go`

### 2026-05-22: Keep code-memory acceleration opt-in after failed wall-time gate

- Decision:
  - Treat FA-PLAN-06 as complete but not default-promoted. `py-codebase-memory` keeps both `fi_accel.chunk_code` and `fi_accel.count_tokens` behind explicit opt-in modes until the downstream wall-time and chunk-contract gates pass.
- Rationale:
  - The merged May 21 adoption proof (`dab7b76`, including `08200dd feat(indexers): gate fi-accel adoption proof`) showed that the accelerated code-memory paths missed the wall-time gate. The chunker path reduced peak Python allocation but changed chunk cardinality, while per-chunk exact token counting dominated wall time.
- Alternatives considered:
  - Promote the downstream integration because it is available and allocation-friendly.
  - Reopen a broad ingestion-pipeline rewrite.
- Consequences:
  - The next `fi-accel` slice is deliberately narrow: reuse tokenizer state for repeated small/medium `count_tokens` calls, then rerun the same `py-codebase-memory` benchmark before revisiting defaults.
- Sources:
  - [S1] `/Users/cblevins/workspace/libs/py-codebase-memory` commit `dab7b76`
  - [S2] `.loom/45-planning-audit-and-backlog-2026-05-20.md`

### 2026-05-22: Bound FA-PLAN-07 to service-friendly Go helpers

- Decision:
  - Promote the existing Go `Version()` helper and pure-Go Markdown text/code-fence helpers as supported additive APIs.
  - Report the Markdown helper surface with a dedicated `markdown_helpers` capability so callers can distinguish it from FFI-backed `chunker_markdown`.
- Rationale:
  - These helpers are small, deterministic, tested with and without CGo, and useful for agent/service workflows that need to clean model Markdown output without linking the native library.
  - They do not require committing to full Go parity for Rust/Python modules.
- Alternatives considered:
  - Remove the untracked files as a dead-end.
  - Expand the Go binding to TOML, TOON, ranking, and every remaining Python/Rust surface.
- Consequences:
  - FA-PLAN-07 closes with a narrow Go boundary: service-friendly pure-Go helpers are acceptable, while broad parity expansion still needs a named consumer and kill-test.
- Sources:
  - [S1] `go/fiaccel/markdown.go`
  - [S2] `go/fiaccel/version.go`
  - [S3] `.loom/30-implementation-plan.md`

### 2026-05-23: Default exact code-memory token counts through batching

- Decision:
  - Add `count_tokens_batch` and `Tokenizer.count_batch` as additive Python APIs.
  - Treat local `PythonIndexer` chunking plus exact `fi_accel` token counts as defaultable for the current `py-codebase-memory/src/codebase_memory` fixture once downstream consumes the batched API.
- Rationale:
  - The repeated-small-chunk blocker was Python/PyO3 call overhead, not token exactness itself. The batched API counts all 141 local chunks through one Python extension call and one tokenizer setup.
  - The May 23 benchmark shows local heuristic counting at 88.96 ms median and local chunking with batched exact counts at 95.54 ms median. The exact path is within roughly 6.6 ms median while producing exact GPT-4 counts.
  - The already-closed chunk parity fixture still passes, and `fi_accel.chunk_code(model="gpt-4")` reaches 26.70 ms median with 141 chunks and matching exact token totals.
- Alternatives considered:
  - Keep optimizing scalar `count_tokens` only.
  - Make `py-codebase-memory` default to the full Rust chunker in this library slice.
- Consequences:
  - Existing scalar APIs remain stable.
  - Downstream default adoption should be a separate `py-codebase-memory` change that calls `count_tokens_batch` for local chunks or explicitly opts into chunker-provided exact counts.
- Sources:
  - [S1] `docs/benchmark-ledger.md`
  - [S2] `scripts/bench_code_memory_token_counts.py`
  - [S3] `crates/fi-accel-py/src/lib.rs`

### 2026-05-23: Treat the benchmark ledger as the only README performance source

- Decision:
  - README performance summaries should link to `docs/benchmark-ledger.md` and include only dated measurements that have command, host context, input shape, and result evidence.
- Rationale:
  - Legacy multiplier claims for token counting, embeddings, fuzzy patching, JSON, SSE, TOML, and TOON were too broad for the current evidence. The ledger already contains narrower, reproducible measurements for ICC utilities, Go transport, and code-memory token counting.
- Alternatives considered:
  - Leave the old table but rely on the top-of-file ledger warning.
  - Delete the performance section entirely until every promoted surface has fresh benchmarks.
- Consequences:
  - Unmeasured surfaces remain documented as APIs without speed claims. New release claims must update the ledger before the README or release notes repeat them.
- Sources:
  - [S1] `README.md`
  - [S2] `docs/benchmark-ledger.md`
  - [S3] `.loom/45-planning-audit-and-backlog-2026-05-20.md`

### 2026-05-24: Keep merged batch token API after reconciliation

- Decision:
  - Keep the May 23 batch-token API merge (`62c6ca9`, including `8a4a705`) instead of trimming `count_tokens_batch` or `Tokenizer.count_batch`.
- Rationale:
  - The stale local checkout contained notes that rejected the batch API after a cached scalar-only rerun, but those notes predate and contradict the current `main` history.
  - The merged benchmark evidence is the stronger source of truth for the API surface: local `PythonIndexer` plus batched exact counts preserves 141 chunks, records 22117 exact tokens, and measures 95.54 ms median versus 88.96 ms for the heuristic baseline while lowering median peak Python allocation from 1489.2 KiB to 1236.2 KiB.
  - The API is additive and directly matches the downstream integration shape: keep local chunking, replace per-chunk scalar token calls with one batched exact-count call.
- Alternatives considered:
  - Remove the merged batch API and rely on cached scalar `count_tokens`.
  - Keep only `Tokenizer.count_batch` and remove the module-level wrapper.
- Consequences:
  - Downstream `py-codebase-memory` should switch local chunk token counting to `count_tokens_batch` in its own integration slice.
  - Future scalar-only benchmark notes must be treated as historical unless they are rerun against current `main` and explain why the batch path no longer applies.
- Sources:
  - [S1] `docs/benchmark-ledger.md`
  - [S2] `scripts/bench_code_memory_token_counts.py`
  - [S3] commit `8a4a705`

### 2026-05-25: Downstream PythonIndexer consumes batch token counts

- Decision:
  - Treat the downstream local `PythonIndexer` exact-token integration as landed: `py-codebase-memory` now assigns local chunk token counts with `fi_accel.count_tokens_batch` instead of scalar `count_tokens` calls.
  - Keep full `fi_accel.chunk_code` default adoption as a separate downstream contract decision.
- Rationale:
  - The downstream rerun measures the integration directly: 144 local chunks, 22504 exact tokens, and 93.99 ms median for downstream batched exact counts versus 107.91 ms median for heuristic counts on the current 16-file fixture.
  - Chunk schema parity still passes with 144 local chunks, 144 `fi_accel` chunks, and `first_divergence: null`.
  - The full chunker path remains faster at 26.96 ms median, but it changes which implementation owns chunk extraction and should be adopted separately from token-counting exactness.
- Alternatives considered:
  - Leave `py-codebase-memory` on scalar exact counting and keep the May 23 result as only a prototype.
  - Flip `py-codebase-memory` directly to full `fi_accel.chunk_code`.
- Consequences:
  - The May 23 downstream follow-up is no longer planned work for local `PythonIndexer` exact token counts.
  - Future code-memory token gate reruns should use the downstream exact mode, not an external post-parse batch count.
- Sources:
  - [S1] `/Users/cblevins/workspace/libs/py-codebase-memory/src/codebase_memory/indexers/base.py`
  - [S2] `/Users/cblevins/workspace/libs/py-codebase-memory/src/codebase_memory/indexers/python_indexer.py`
  - [S3] `docs/benchmark-ledger.md`

### 2026-05-24: Treat hash-chain service fixture as landed

- Decision:
  - Keep the May 23 hash-chain fixture merge (`020b1c8`, including `fbbaa22`) as the current FA-PLAN-04 state.
- Rationale:
  - The fixture covers the ICC-shaped audit-chain verifier cases that stale local planning notes still listed as open: valid chain, tampered payload, and broken `prev_hash` diagnostics across native and fallback paths.
  - The fallback diagnostic shape now matches native mismatch details by including `expected_prev_hash` and `actual_prev_hash`.
- Alternatives considered:
  - Carry forward stale notes that still describe the service-shaped hash-chain fixture as missing.
- Consequences:
  - FA-PLAN-04 release hardening can move on to remaining gates such as Gemma exactness and claim-backed benchmarks; the hash-chain service fixture should not be re-planned.
- Sources:
  - [S1] `python/tests/test_icc_release_gate.py`
  - [S2] `python/fi_accel/fallback.py`
  - [S3] commit `fbbaa22`

### 2026-05-23: Keep WASM tokenizer deferred and code chunking unsupported

- Decision:
  - Do not add tokenizer support to the default `fi-accel-wasm` bundle now; defer it until there is a named browser/JS consumer and ship it only as an explicit opt-in package or feature.
  - Treat Rust-core tree-sitter code chunking as permanently unsupported in `fi-accel-wasm` for the current Rust chunker path.
- Rationale:
  - The current WASM package already initializes quickly and keeps tokenizer/code chunking out of the browser bundle: 2080517 raw `.wasm` bytes, 771710 gzip bytes, and 0.405 ms median Node `initSync` on the May 23 web-target measurement.
  - A real tokenizer prototype builds and works, but grows the `.wasm` to 9366845 raw bytes and 4097317 gzip bytes while raising median `initSync` to 1.944 ms. The runtime cost is acceptable, but the roughly 3.33 MB gzip download increase is too large for the default package without a consumer that needs exact browser tokenization.
  - A code-chunking prototype using the current Rust tree-sitter path does not build for `wasm32-unknown-unknown`; tree-sitter and grammar crates invoke C compilation and fail on missing libc headers such as `stdlib.h` and `stdio.h`.
- Alternatives considered:
  - Implement tokenizer in the default package because the init-time delta is only about 1.54 ms median.
  - Add tree-sitter code chunking behind a feature flag despite the current browser-target build failure.
  - Mark tokenizer permanently unsupported.
- Consequences:
  - README and capability reporting remain honest: WASM tokenizer and code chunking stay unsupported in the default surface.
  - Future tokenizer work must start with an opt-in packaging plan and consumer budget instead of adding `tiktoken-rs` to every JS/WASM user.
  - Future browser code parsing should use a separate parser-WASM design or package; it should not be treated as simple parity work for the existing Rust core chunker.
- Sources:
  - [S1] `docs/benchmark-ledger.md`
  - [S2] `crates/fi-accel-wasm/Cargo.toml`
  - [S3] `crates/fi-accel-wasm/src/lib.rs`

### 2026-05-23: Close first missing ICC utility benchmark proof

- Decision:
  - Add benchmark-ledger proof for FA-ICC-02 HTML-to-text extraction and FA-ICC-05 encoding detection before treating those utilities as release-ready ICC surfaces.
- Rationale:
  - After the ledger cleanup, the ICC release-gate table already had measurements for batch hashing, ranking/RRF, NDJSON, pattern matching, and mail parsing. The first implemented backlog utilities without benchmark rows were HTML extraction and charset detection.
  - The new Criterion cases are narrow and consumer-shaped: 250 repeated email/article HTML fragments with entities and links, and 1K short artifact lines containing Windows-1252 smart-quote bytes.
- Alternatives considered:
  - Add broader performance claims for all text utilities at once.
  - Skip benchmarks because native/fallback parity tests already cover behavior.
- Consequences:
  - Release notes may describe FA-ICC-02 and FA-ICC-05 as benchmark-backed utilities, but should avoid multiplier claims until a Python or ICC baseline comparison exists.
  - The remaining FA-PLAN-04 release boundary is Gemma exactness behind a local tokenizer asset and complete fallback parity breadth for promoted native functions.
- Sources:
  - [S1] `crates/fi-accel-core/benches/benchmarks.rs`
  - [S2] `docs/benchmark-ledger.md`
  - [S3] `.loom/45-planning-audit-and-backlog-2026-05-20.md`
