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
