# RALPH Slice Handoff

## Slice Summary

- Milestone: M2/M5 transport helper adoption proof.
- Slice: Go route preview production-shaped benchmark fixture.
- Status: complete locally; MR/CI pending.

## What Landed

- Key changes: added reusable small and gateway-shaped `tools/call` route-preview payloads, added focused coverage for the gateway-shaped payload, kept the existing small benchmark, and added a second benchmark for the larger payload.
- Key files: `go/fiaccel/transport_route_preview_test.go`, `README.md`, `docs/service-acceleration-kill-test-2026-05-20.md`, `.loom/43-iteration-plan-route-preview-fixtures-2026-05-20.md`.
- Validation results: `go test ./...` passed; `CGO_ENABLED=0 go test ./...` passed; benchmark results on Apple M4 were 1241-1334 ns/op, 744 B/op, 21 allocs/op for the small payload and 5123-5436 ns/op, 1744 B/op, 22 allocs/op for the gateway-shaped payload. CGo validation emitted the known macOS-version linker warnings for the local static FFI archive.

## What Is Still Open

- Remaining acceptance criteria: push/MR/CI.
- Known issues: the larger fixture is representative rather than captured from live production traffic; service promotion still requires a `fi-mcp-gateway` benchmark with a matching or captured payload mix.
- Dependencies: downstream adoption depends on rerunning the gateway route benchmark and clearing the documented wall-time or allocation threshold.

## Next Actions

1. Add a matching production-shaped benchmark case to `fi-mcp-gateway`.
2. Compare service-local routing against `ParseJSONRPCRoutePreview` using the same payload shape.
3. Merge the gateway integration only if behavior is preserved and the kill-test threshold is met.

## Context Links

- Agent-context session: `690932ebddb82ea1`
- Task IDs: `255486e94b7eead8`.
- Relevant docs/specs: `.loom/43-iteration-plan-route-preview-fixtures-2026-05-20.md`, `docs/service-acceleration-kill-test-2026-05-20.md`.
