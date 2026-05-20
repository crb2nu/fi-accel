# RALPH Iteration Plan

## Review

- Roadmap milestone: M5 from `.loom/30-implementation-plan.md` - close the loop with benchmarks, docs, and one downstream adopter.
- Spec section(s): `.loom/20-product-spec.md` transport flow, Go service adoption, and benchmark-backed promotion requirements.
- Prior decisions to preserve:
  - Keep service semantics outside `fi-accel`; only promote low-level data-plane helpers.
  - Do not integrate downstream services until a measured service path shows either wall-time or allocation value.
  - The acceleration scout is aggregation-only and should feed concrete adoption decisions.

## Align

- Slice name: Service acceleration kill-test report
- Scope in:
  - Benchmark one `fi-mcp-gateway` JSON-RPC routing shape.
  - Benchmark one `loom-core` stdio/widget JSON-RPC parsing shape.
  - Benchmark current `fi-accel` Go JSON-RPC line parsing against the same class of payload.
  - Record whether the first-adopter assumption passed, failed, or needs a narrower helper.
- Scope out:
  - No downstream service code changes.
  - No new `fi-accel` API in this slice.
  - No production profiling or live traffic capture.
- Acceptance criteria:
  - Report includes commands, measured wall time, allocations, and bytes/op.
  - Report names the exact service helper path that could be replaced next.
  - Handoff notes identify the next implementation slice.
  - Agent context records the decision and follow-up task.
- Dependencies/blockers:
  - Current Go transport wrapper parses framed messages but does not expose a route-preview API, so it cannot yet replace the gateway route parse without leaving duplicate decoding in place.

## Land

- Planned file areas:
  - `.loom/34-iteration-plan-service-kill-test-2026-05-20.md`
  - `.loom/35-slice-handoff-service-kill-test-2026-05-20.md`
  - `docs/service-acceleration-kill-test-2026-05-20.md`
- Implementation steps:
  1. Run temporary local benchmarks in `fi-mcp-gateway`, `loom-core`, and `fi-accel` Go.
  2. Remove temporary benchmark files.
  3. Commit only the evidence docs and handoff update.

## Prove

- Tests to run:
  - `go test ./...` in `go/fiaccel`
  - `go test ./internal/mcpws -run '^$' -bench 'Benchmark(RouteMessageToolsCall|JSONRPCEnvelopeUnmarshal|PolicyAuthorizeAllowAll)$' -benchmem -count=5` in `services/fi-mcp-gateway` with temporary benchmark file
  - `go test ./cmd/mcp-loom-widget -run '^$' -bench 'BenchmarkDispatchToolsCallParseOnly$' -benchmem -count=5` in `services/loom-core` with temporary benchmark file
- Lint/static checks:
  - Documentation-only committed slice; no formatter required beyond Markdown review.
- CI checks:
  - Normal GitLab docs pipeline after push/MR.

## Handoff/Harvest

- Docs to update:
  - New kill-test report under `docs/`.
  - Slice handoff with the next implementation target.
- Agent-context entries to add:
  - Decision: current Go JSON-RPC parser is not a direct single-message parse win.
  - Finding: gateway route path is the best next target if `fi-accel` exposes a route-preview helper.
- Next-slice candidates:
  - Add a Go/Rust route-preview helper that extracts method, notification/request status, `params.name`, and input byte count in one pass.
  - Rerun the gateway benchmark and only integrate if it beats the existing full route path by at least 20% wall time or materially reduces allocations while preserving routing behavior.
