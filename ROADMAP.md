# fi-accel Roadmap

> Last Updated: 2026-07-02
> Tier: 2 (see workspace AGENTS.md "Portfolio Tiers")
> Tracking Issue: https://gitlab.flexinfer.ai/libs/fi-accel/-/issues/12

## Current Status

Active (feature work through 2026-05; CI touch 2026-07). fi-accel is a Rust
acceleration workspace with PyO3 wheel, WASM, and Go bindings, consumed by ICC
(`icc-integration` backlog) and by codebase-memory tooling (chunk parity,
batched token counts — `cb2fe168`, `8a4a705e`). The FA-PLAN slice train landed
through 2026-05-25: batched token counts, markdown helpers promoted to Go,
JSON-RPC prefix route extractor, hash-chain service fixture. On 2026-07-02 the
repo was onboarded to the platform/gitops rust CI template (`e884be1a`), with
`CI_TEMPLATE_NONBLOCKING=true` set transitionally because `crates/fi-accel-ffi`
carries 32 pre-existing clippy `missing_safety_doc` errors (`02d0cb8d`) — that
is tracked quality debt, not a green bill of health.

- **Plan store**: plan-workspace-portfolio-refresh-2026-h2-roadmaps-quality-baselin-f3db23
- **Deployed**: not deployed (library; publishes py wheel + npm package from CI)
- **CI**: rust template family (platform/gitops `/ci/templates/rust.yml`), lint/test currently non-blocking via `CI_TEMPLATE_NONBLOCKING`

## Now

- [ ] Pay down fi-accel-ffi clippy `missing_safety_doc` debt and remove `CI_TEMPLATE_NONBLOCKING` so template lint/test jobs block again (#13)
- [ ] FA-ICC-04: Gemma tokenizer support in `count_tokens` / `truncate_to_tokens` (#4)

## Next

- [ ] FA-ICC-01: Batch SHA-256 hashing (#1)
- [ ] FA-ICC-02: HTML → plain text extraction (#2)
- [ ] FA-ICC-05: Charset / encoding detection (#5)
- [ ] FA-ICC-06: Streaming NDJSON reader (#6)
- [ ] FA-ICC-09: Reciprocal rank fusion helper (#9) · FA-ICC-11: pure-Python fallback beside the wheel (#11)

## Later

- P3 ICC integrations: SIMD multi-pattern matcher (#7), EML/mbox parser (#8), hash-linked event chain verifier (#10)

## Backlog

Full backlog: [P1 issues](https://gitlab.flexinfer.ai/libs/fi-accel/-/issues/?label_name[]=P1) ·
[P2](https://gitlab.flexinfer.ai/libs/fi-accel/-/issues/?label_name[]=P2) ·
[P3](https://gitlab.flexinfer.ai/libs/fi-accel/-/issues/?label_name[]=P3) ·
[Milestones](https://gitlab.flexinfer.ai/libs/fi-accel/-/milestones)
