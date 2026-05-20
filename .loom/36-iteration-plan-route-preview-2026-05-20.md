# RALPH Iteration Plan

## Review

- Roadmap milestone: M2/M5 transport helper adoption proof.
- Spec section(s): Go transport helpers and benchmark-backed promoted module families.
- Prior decisions to preserve: the 2026-05-20 service kill-test showed the existing Go `ParseJSONRPCLines` wrapper is not a worthwhile service integration target for single-message routes; add a narrower route-preview helper before downstream adoption.

## Align

- Slice name: Go JSON-RPC route preview helper.
- Scope in: add a pure Go helper that previews one JSON-RPC payload, returns method, non-null id presence, `tools/call` `params.name`, input bytes, and validation status; add Go tests and benchmark coverage; update README and the kill-test doc.
- Scope out: no downstream service integration, no Rust/FFI replacement, no changes to MCP routing behavior in `fi-mcp-gateway` or `loom-core`.
- Acceptance criteria: helper is additive, works with and without CGo, distinguishes requests from notifications by non-null id presence, reports invalid JSON/schema status, and has benchmark output for a `tools/call` payload.
- Dependencies/blockers: service promotion remains blocked until the helper is benchmarked against `fi-mcp-gateway`'s `routeMessage` path.

## Land

- Planned file areas: `go/fiaccel/transport.go`, Go transport tests, README, service kill-test doc.
- Implementation steps:
  1. Add the route preview data model and parser.
  2. Cover valid request, notification, null id, invalid JSON, and missing `tools/call` name cases.
  3. Add usage/performance documentation and rerun Go validation.

## Prove

- Tests to run: `go test ./...`; `CGO_ENABLED=0 go test ./...`; `go test ./... -run '^$' -bench 'BenchmarkParseJSONRPCRoutePreviewToolsCall$' -benchmem -count=5`.
- Lint/static checks: `gofmt`; `git diff --check`.
- CI checks: GitLab pipeline after push/MR.

## Handoff/Harvest

- Docs to update: README Go example and `docs/service-acceleration-kill-test-2026-05-20.md`.
- Agent-context entries to add: route-preview API decision, benchmark finding, next service-adoption blocker.
- Next-slice candidates: copy the helper benchmark into `fi-mcp-gateway` route benchmarks and promote service integration only if it reaches the documented kill-test threshold.
