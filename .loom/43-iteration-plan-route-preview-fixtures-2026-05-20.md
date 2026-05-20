# RALPH Iteration Plan

## Review

- Roadmap milestone: M2/M5 transport helper adoption proof.
- Spec section(s): Go transport helpers, benchmark-backed promoted module families, and downstream service adoption validation.
- Prior decisions to preserve: route-preview remains pure Go and additive; service integration remains gated on measured `fi-mcp-gateway` route behavior.

## Align

- Slice name: Go route preview production-shaped benchmark fixture.
- Scope in: add a gateway-shaped `tools/call` route-preview fixture, assert the helper extracts the same routing fields on that payload, preserve the existing small helper benchmark, and record benchmark evidence.
- Scope out: no downstream service integration, no service code changes, no parser semantic changes.
- Acceptance criteria: focused Go tests pass, both CGo and `CGO_ENABLED=0` sweeps pass, benchmarks report small and gateway-shaped payload performance, and docs identify the next service benchmark step.
- Dependencies/blockers: service promotion remains blocked until `fi-mcp-gateway` reruns its route benchmark with production-shaped payloads and clears the documented kill-test threshold.

## Land

- Planned file areas: `go/fiaccel/transport_route_preview_test.go`, `README.md`, `docs/service-acceleration-kill-test-2026-05-20.md`, `.loom/43-iteration-plan-route-preview-fixtures-2026-05-20.md`.
- Implementation steps:
  1. Add reusable route-preview benchmark payloads for the existing small tools call and a larger gateway-shaped tools call.
  2. Add focused coverage proving the larger payload still exposes method, id presence, tool name, raw arguments, and input byte count.
  3. Run Go test sweeps and benchmark both payload sizes.

## Prove

- Tests to run:
  - `go test ./...`
  - `CGO_ENABLED=0 go test ./...`
  - `go test ./... -run '^$' -bench 'BenchmarkParseJSONRPCRoutePreview(ToolsCall|GatewayToolsCall)$' -benchmem -count=5`
- Lint/static checks:
  - `gofmt`
  - `git diff --check`
- CI checks:
  - GitLab pipeline after branch push/MR.

## Handoff/Harvest

- Docs to update: service acceleration kill-test report and README performance row.
- Agent-context entries to add: fixture decision and benchmark finding.
- Next-slice candidates: rerun `fi-mcp-gateway` route integration with matching production-shaped payloads, then merge service integration only if behavior is preserved and the kill-test threshold is met.
