# RALPH Slice Handoff

## Slice Summary

- Milestone: M2/M5 transport helper adoption proof.
- Slice: Go JSON-RPC route preview helper.
- Status: complete locally; MR/CI pending at handoff time until pushed.

## What Landed

- Key changes: added `ParseJSONRPCRoutePreview` as a pure Go helper that previews one JSON-RPC payload and returns method, non-null id presence, `tools/call` `params.name`, input bytes, validity, and a route status.
- Key files: `go/fiaccel/transport.go`, `go/fiaccel/transport_route_preview_test.go`, `docs/service-acceleration-kill-test-2026-05-20.md`, `README.md`.
- Validation results: `gofmt`, `git diff --check`, `CGO_ENABLED=0 go test ./...`, default `go test ./...`, and `BenchmarkParseJSONRPCRoutePreviewToolsCall` passed. The CGo runs used the ignored local `go/lib/darwin_arm64/libfi_accel_ffi.a` archive and emitted the known macOS version linker warnings.

## What Is Still Open

- Remaining acceptance criteria: push branch, create MR, and confirm GitLab CI.
- Known issues: route-preview is benchmarked standalone, not yet wired into `fi-mcp-gateway`.
- Dependencies: downstream adoption should compare against the existing `routeMessage` service benchmark and preserve service-local routing behavior.

## Next Actions

1. Replace the duplicate `fi-mcp-gateway` route decode with `ParseJSONRPCRoutePreview` in a service-local branch.
2. Rerun `BenchmarkRouteMessageToolsCall` and behavior tests in `fi-mcp-gateway`.
3. Promote integration only if it preserves behavior and keeps the documented >=20% wall-time improvement or material allocation reduction.

## Context Links

- Agent-context session: `0e6b38742d533216`
- Task IDs: none
- Relevant docs/specs: `.loom/36-iteration-plan-route-preview-2026-05-20.md`, `docs/service-acceleration-kill-test-2026-05-20.md`, `.loom/30-implementation-plan.md`
