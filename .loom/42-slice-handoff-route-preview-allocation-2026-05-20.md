# RALPH Slice Handoff

## Slice Summary

- Milestone: M2/M5 transport helper adoption proof.
- Slice: Go route preview allocation reduction.
- Status: complete locally; MR/CI pending.

## What Landed

- Key changes: `ParseJSONRPCRoutePreview` now decodes top-level `jsonrpc` and `method` directly while keeping raw `id` and `params` only where routing needs presence checks or nested `tools/call` extraction. Invalid top-level field types now have focused tests and remain classified as invalid JSON-RPC.
- Key files: `go/fiaccel/transport.go`, `go/fiaccel/transport_route_preview_test.go`, `docs/service-acceleration-kill-test-2026-05-20.md`, `.loom/41-iteration-plan-route-preview-allocation-2026-05-20.md`.
- Validation results: `go test ./...` passed; `CGO_ENABLED=0 go test ./...` passed; route-preview benchmark improved from 1409-1531 ns/op, 1136 B/op, 27 allocs/op to 1135-1156 ns/op, 744 B/op, 21 allocs/op. CGo validation emitted the known macOS-version linker warnings for the local static FFI archive.

## What Is Still Open

- Remaining acceptance criteria: push/MR/CI.
- Known issues: allocation count is improved but still higher than the service-local single-envelope decode, so service integration should not be promoted on this result alone.
- Dependencies: downstream adoption depends on rerunning the `fi-mcp-gateway` route benchmark on production-shaped payloads with this lower-allocation helper.

## Next Actions

1. Rerun the `fi-mcp-gateway` route integration probe against this helper version.
2. Add or reuse production-shaped `tools/call` payload fixtures for the benchmark.
3. Merge the gateway integration only if it preserves behavior and clears the documented kill-test threshold.

## Context Links

- Agent-context session: `dda42b11730112ef`
- Task IDs: pending.
- Relevant docs/specs: `.loom/38-iteration-plan-route-preview-arguments-2026-05-20.md`, `docs/service-acceleration-kill-test-2026-05-20.md`, `.loom/30-implementation-plan.md`.
