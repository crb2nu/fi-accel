# RALPH Iteration Plan

## Review

- Roadmap milestone: M2/M5 transport helper adoption proof.
- Spec section(s): Go transport helpers and benchmark-backed promoted module families.
- Prior decisions to preserve: the first route-preview helper should remain pure Go and additive; downstream service integration stays gated on real service benchmarks.

## Align

- Slice name: Go route preview arguments exposure.
- Scope in: expose raw `tools/call` `params.arguments` from `ParseJSONRPCRoutePreview`, cover it in Go tests, update docs, and run a service-local integration probe in `fi-mcp-gateway`.
- Scope out: no production service merge until benchmark evidence clearly clears the kill-test threshold.
- Acceptance criteria: helper remains additive, existing status behavior is preserved, raw arguments are available to service routers, Go tests pass, and the service probe records wall-time/allocation results.
- Dependencies/blockers: service promotion depends on either stronger allocation wins or a benchmark on the exact production payload mix.

## Land

- Planned file areas: `go/fiaccel/transport.go`, `go/fiaccel/transport_route_preview_test.go`, `README.md`, `docs/service-acceleration-kill-test-2026-05-20.md`.
- Implementation steps:
  1. Add raw arguments to the Go route preview struct.
  2. Validate helper behavior and standalone benchmark impact.
  3. Probe `fi-mcp-gateway` route integration using the new field and record results.

## Prove

- Tests to run: `go test ./...` in `go/fiaccel`; service-local `go test ./internal/mcpws`.
- Lint/static checks: `gofmt`; `git diff --check`.
- CI checks: GitLab pipeline after branch push/MR.

## Handoff/Harvest

- Docs to update: README Go example and service acceleration kill-test report.
- Agent-context entries to add: arguments exposure decision and service probe finding.
- Next-slice candidates: reduce route-preview allocations or run the service benchmark against production-shaped payloads before merging the gateway integration.
