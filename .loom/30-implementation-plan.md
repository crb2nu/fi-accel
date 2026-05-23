# Implementation Plan

## Scope

Plan the next delivery slices that would make `fi-accel` materially more useful to the surrounding workspace. This is a planning artifact only; no production code changes are included in this turn.

## Milestones

- M1. Establish the product surface and parity baseline
- M2. Ship MCP transport helpers
- M3. Improve chunking/indexing metadata and binding reach
- M4. Add array-oriented Python interop and stronger data-plane options
- M5. Close the loop with benchmarks, docs, and one downstream adopter

## Progress Updates

- 2026-03-14:
  - Closed the runtime introspection parity slice for M1 by standardizing version/capability reporting across Python, Go, and WASM-facing docs.
  - M1 status: in progress, with capability matrix publication and binding-level runtime introspection now landed.
- 2026-05-20:
  - Advanced the M2/M5 transport adoption proof by adding and tightening the pure Go JSON-RPC route-preview path for service benchmarks.
  - Latest route-preview allocation slice reduced the standalone helper from 1409-1531 ns/op, 1136 B/op, 27 allocs/op to 1135-1156 ns/op, 744 B/op, 21 allocs/op on Apple M4.
  - Downstream service integration remains gated on rerunning `fi-mcp-gateway` benchmarks against production-shaped payloads.
- 2026-05-20 planning audit:
  - Reviewed the prior service-acceleration brainstorm and found its recommended scout/drop-in direction mostly implemented inside this repo.
  - Reconciled the ICC backlog against current code: most ICC utility surfaces now exist, with Gemma exactness, benchmark evidence, attachment-memory behavior, and fallback parity depth left as release gates.
  - Added a new post-route brainstorm and backlog. The next P1 path is to rerun the `fi-mcp-gateway` production-shaped route benchmark, make the service integration decision, and harden the implemented ICC utility surface for release.
  - Sources: `.loom/45-planning-audit-and-backlog-2026-05-20.md`, `.loom/brainstorm-post-route-preview-adoption-2026-05-20.md`.
- 2026-05-20 gateway rerun:
  - The production-shaped `fi-mcp-gateway` benchmark did not clear the route-preview service integration gate.
  - Current gateway route path: 8352-8644 ns/op, 4144 B/op, 80 allocs/op.
  - `fiaccel.ParseJSONRPCRoutePreview` candidate path: 8328-8560 ns/op, 4176 B/op, 82 allocs/op.
  - Decision: skip direct gateway integration for now and shift P1 implementation toward ICC release hardening or a code-memory ingestion proof.
  - Source: `docs/service-acceleration-kill-test-2026-05-20.md`.
- 2026-05-21 code-memory adoption proof:
  - The merged `py-codebase-memory` proof (`dab7b76`, including `08200dd feat(indexers): gate fi-accel adoption proof`) kept accelerated Python chunking and exact token counting opt-in.
  - Downstream default adoption missed the wall-time gate: the local tree-sitter plus heuristic token path remained the default, while `fi_accel.count_tokens` and `fi_accel.chunk_code` were only enabled through explicit environment modes.
  - Decision: mark FA-PLAN-06 as closed, gate failed for default promotion; carry the blocker forward only as a narrow `fi-accel` repeated-small-chunk token-counting fast path.
  - Source: `/Users/cblevins/workspace/libs/py-codebase-memory` commit `dab7b76`.
- 2026-05-22 FA-PLAN-07 Go parity decision:
  - Promote the existing untracked Go `Version()` and pure-Go Markdown text/code-fence helpers as supported additive surface.
  - Boundary: keep Go parity focused on service-friendly helpers that work with `CGO_ENABLED=0`; do not expand Go to TOML, TOON, ranking, or broad Rust-core parity until a named consumer and kill-test justify it.
  - Capability/docs/tests now identify `markdown_helpers` separately from FFI-backed `chunker_markdown`.
- 2026-05-23 FA-PLAN-04 hash-chain fixture:
  - Added an ICC-shaped audit-chain fixture to the Python release-gate tests.
  - Native and fallback `verify_hash_chain` now agree on valid, tampered-payload, and broken-link diagnostics for that fixture.
  - The fallback result shape now includes `expected_prev_hash` and `actual_prev_hash` on entry-hash mismatches, matching the native diagnostic payload.

## Plan

1. Define and publish a capability matrix. Status: mostly complete.
   - Document every promoted module by target: Rust, Python, Go, WASM.
   - Add a small `capabilities()` / `version()` style introspection surface in each binding.
   - Update the README to reflect the actual current surface, including chunking and JSON-RPC helpers.

2. Add an MCP transport module in the Rust core. Status: implemented enough for current gateway proof; downstream adoption still open.
   - Implement newline-delimited stdio frame parsing/serialization.
   - Implement Streamable HTTP helpers for request/response framing, `Mcp-Session-Id`, `MCP-Protocol-Version`, and resumability metadata.
   - Keep transport helpers low-level and reusable; do not bake in server frameworks.
   - Expose the highest-value transport helpers to Go and Python after the Rust API stabilizes.
   - Current decision: FA-PLAN-01 failed the service integration gate and FA-PLAN-02 was skipped; revisit gateway routing only through FA-PLAN-06A or a materially different extractor.

3. Fix the biggest binding-parity gaps. Status: partially complete; needs explicit product decisions for remaining Go/WASM gaps.
   - Go:
     - add JSON-RPC batch parsing
     - reject TOML, TOON, ranking, and broad parity work as out of scope until a named consumer and kill-test justify them
     - keep markdown/text chunking through the existing FFI-backed chunker APIs, and promote only the small pure-Go Markdown text/code-fence helper surface for `!cgo` workflows
     - make degraded `!cgo` behavior explicit in docs and capability reporting
   - WASM:
     - add tokenizer/count/truncate/split helpers
     - add `top_k_similar`, `compute_centroid`, and batch prep helpers where practical
     - keep code chunking optional unless parser bundle size is acceptable
   - Next action: decide FA-PLAN-08 before adding more WASM parity surface.

4. Expand chunk metadata for indexing workflows. Status: implemented in the library; downstream proof closed as opt-in, not default.
   - Define a richer chunk schema in Rust first.
   - Add fields that downstream indexers actually need, such as import summaries and stronger symbol/container metadata.
   - Preserve the current chunk schema fields so existing Python callers do not break.
   - Validated against `py-codebase-memory`; the proof preserved opt-in modes because the accelerated paths missed the downstream wall-time gate.
   - Next action: execute FA-PLAN-06B from `.loom/45-planning-audit-and-backlog-2026-05-20.md` if local code still rebuilds tokenizer state for repeated small chunks.

5. Improve Python vector/data interop. Status: still open.
   - Prototype NumPy-oriented APIs using the Rust `numpy` crate.
   - Benchmark nested-list input versus array-oriented paths.
   - Keep the feature optional until packaging and ABI implications are understood.

6. Evaluate optional Arrow-backed interchange. Status: still open and lower priority until a named consumer requires it.
   - Treat Arrow as a feature-gated experiment for high-throughput batch exchange, not a default dependency.
   - Validate whether it meaningfully helps real workspace flows before promoting it.

7. Back all promoted surfaces with benchmarks and examples. Status: now the primary release-hardening track.
   - Add benchmark coverage for tokenizer, transport, TOML/TOON, and any new interop paths.
   - Publish minimal examples for Rust, Python, Go, and JS/WASM.
   - Add downstream smoke tests where feasible.
   - Next action: execute FA-PLAN-04 and FA-PLAN-09 from `.loom/45-planning-audit-and-backlog-2026-05-20.md`.

## Active Backlog

- Done: FA-PLAN-04 ICC Utility Release Gate; hash-chain service fixture added for the remaining ICC verifier proof.
- P1: FA-PLAN-05 Gemma Fixture Harness.
- Done/Gated: FA-PLAN-06 Code-Memory Ingestion Proof.
- Done: FA-PLAN-06B Repeated Token Counting Fast Path.
- P2: FA-PLAN-06A Specialized Route Extractor Kill-Test.
- Done: FA-PLAN-07 Go Parity Decision.
- P2: FA-PLAN-08 WASM Tokenizer And Code-Chunking Kill-Test.
- P2: FA-PLAN-09 Benchmark Ledger.
- P3: FA-PLAN-10 Patch Repair Revisit.

See `.loom/45-planning-audit-and-backlog-2026-05-20.md` for acceptance criteria and evidence.

2026-05-23 update: FA-PLAN-06B landed as an additive batched tokenizer API
(`count_tokens_batch` and `Tokenizer.count_batch`) plus a repeatable
`py-codebase-memory` benchmark script. The May 23 fixture result makes local
`PythonIndexer` chunking plus exact `fi_accel` token counts defaultable once the
downstream integration switches away from per-chunk scalar `count_tokens` calls:
88.96 ms median for heuristic counts versus 95.54 ms median for batched exact
counts over 141 chunks. The full `fi_accel.chunk_code(model="gpt-4")` path is
also faster at 26.70 ms median with passing chunk parity, but should still be
promoted by an explicit downstream adoption change.

## Test Plan

- Rust:
  - `cargo test --workspace`
  - `cargo bench -p fi-accel-core`
- Python:
  - wheel build via `maturin`
  - Python tests for new APIs and capability reporting
- Go:
  - `go test ./...` in `go/fiaccel`
  - test both `cgo` and `!cgo` modes where possible
- WASM:
  - build validation via `wasm-pack`
  - smoke tests for exported functions
- Downstream validation:
  - pick at least one consumer from `services` or `libs` and replace one local helper path with `fi-accel`

## Rollout / Backout

- Rollout:
  - Land additive APIs behind feature flags where dependencies are heavier or semantics are still being proven.
  - Update README and per-language examples before asking downstream repos to adopt new surfaces.
- Backout:
  - Remove or disable new optional modules/features without touching existing stable entry points.
  - Keep old APIs intact until at least one release after the replacements are documented.

## Acceptance Criteria

- The repo has a documented capability matrix and updated README surface description.
- The Rust core exposes transport helpers that match the MCP transport spec well enough for reuse by adjacent repos.
- Go and WASM no longer look materially incomplete next to Python for the promoted module families.
- Chunk metadata is sufficient to support at least one indexing/intelligence consumer as an opt-in path, with default adoption blocked unless wall-time and schema gates pass.
- Python has a benchmark-backed plan or implementation for array-oriented vector interop.
- Benchmarks cover every module family that is being actively promoted.

## Risks / Dependencies

- Depends on maintaining alignment with the MCP transport spec.
- Depends on packaging discipline across PyO3, CGo/FFI, and WASM builds.
- Depends on choosing one downstream consumer for real adoption, otherwise the work risks remaining abstract.
- Arrow and NumPy work may add ABI/build complexity; keep them optional until proven.

## Sources

- [S1] `README.md:7-16`
- [S2] `crates/fi-accel-core/src/lib.rs:1-47`
- [S3] `crates/fi-accel-core/src/chunker/mod.rs:72-125`
- [S4] `crates/fi-accel-core/src/tokenizer.rs:18-68`
- [S5] `crates/fi-accel-core/benches/benchmarks.rs:11-225`
- [S6] `crates/fi-accel-py/src/lib.rs:297-324`
- [S7] `crates/fi-accel-py/src/lib.rs:450-552`
- [S8] `crates/fi-accel-wasm/src/lib.rs:193-219`
- [S9] `crates/fi-accel-wasm/src/lib.rs:297-345`
- [S10] `go/fiaccel/tokenizer.go:3-27`
- [S11] `go/fiaccel/fiaccel_nocgo.go:13-50`
- [S12] `/Users/cblevins/workspace/services/loom-core/README.md:63-71`
- [S13] `/Users/cblevins/workspace/services/mcp-orchestra/README.md:40-49`
- [S14] `/Users/cblevins/workspace/services/diff-surgeon/README.md:17-25`
- [S15] `/Users/cblevins/workspace/libs/py-codebase-memory/README.md:16-24`
- [S16] <https://modelcontextprotocol.io/specification/2025-06-18/basic/transports> (accessed 2026-03-08 via Tavily extract)
- [S17] <https://tree-sitter.github.io/tree-sitter/> (accessed 2026-03-08 via Tavily extract)
- [S18] <https://docs.rs/simd-json/latest/simd_json/> (accessed 2026-03-08 via Tavily extract)
- [S19] <https://docs.rs/numpy/latest/numpy/> (accessed 2026-03-08 via Tavily extract)
- [S20] <https://arrow.apache.org/docs/format/Columnar.html> (accessed 2026-03-08 via Tavily extract)
- [S21] `.loom/45-planning-audit-and-backlog-2026-05-20.md`
- [S22] `.loom/brainstorm-post-route-preview-adoption-2026-05-20.md`
- [S23] `/Users/cblevins/workspace/libs/py-codebase-memory` commit `dab7b76`
