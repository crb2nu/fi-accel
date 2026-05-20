# RALPH Iteration Plan

## Review

- Roadmap milestone: M2/M5 transport helper adoption proof.
- Spec section(s): Go transport helpers, benchmark-backed promoted module families, and service adoption validation.
- Prior decisions to preserve: route-preview remains pure Go and additive; downstream service integration stays gated on measured `fi-mcp-gateway` route behavior.

## Align

- Slice name: Go route preview allocation reduction.
- Scope in: reduce allocations in `ParseJSONRPCRoutePreview` without changing the public result shape, preserve invalid JSON versus invalid JSON-RPC status behavior, update focused tests, and record benchmark evidence.
- Scope out: no downstream service integration, no Rust/FFI route-preview rewrite, no new service payload semantics.
- Acceptance criteria: helper still exposes method, non-null id presence, `tools/call` name, raw arguments, input bytes, and validation status; invalid field-type payloads are classified as invalid JSON-RPC; Go tests pass with and without CGo; standalone benchmark shows lower allocation count than the prior 27 allocs/op baseline.
- Dependencies/blockers: service promotion remains blocked until `fi-mcp-gateway` is rerun against this lower-allocation helper on production-shaped payloads.

## Land

- Planned file areas: `go/fiaccel/transport.go`, `go/fiaccel/transport_route_preview_test.go`, `docs/service-acceleration-kill-test-2026-05-20.md`.
- Implementation steps:
  1. Replace the custom raw top-level route envelope with direct string/raw-message decoding for the fields needed by routing.
  2. Add edge-case coverage for invalid top-level field types.
  3. Rerun Go tests, CGo-disabled tests, and route-preview benchmarks.

## Prove

- Tests to run:
  - `go test ./...`
  - `CGO_ENABLED=0 go test ./...`
  - `go test ./... -run '^$' -bench 'BenchmarkParseJSONRPCRoutePreviewToolsCall$' -benchmem -count=5`
- Lint/static checks:
  - `gofmt`
  - `git diff --check`
- CI checks:
  - GitLab pipeline after branch push/MR.

## Handoff/Harvest

- Docs to update: service acceleration kill-test report.
- Agent-context entries to add: route-preview allocation decision and benchmark finding.
- Next-slice candidates: rerun `fi-mcp-gateway` route integration benchmark with production-shaped payloads, then merge service integration only if behavior is preserved and the documented kill-test threshold is met.
