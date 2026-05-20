# Service Acceleration Kill-Test (2026-05-20)

## Summary

The first-adopter kill-test did not justify wiring the current Go `fi-accel` JSON-RPC parser directly into a service. The current wrapper is slower than the simple single-message `encoding/json` envelope parses used by `fi-mcp-gateway` and `loom-core`, but it allocates less than the gateway's full route parse. The next useful slice is therefore a narrower route-preview helper, not service integration.

## Question

Can an active workspace service replace a local CPU-side JSON-RPC parsing path with existing or near-additive `fi-accel` helpers and get either:

- at least 20% wall-time improvement, or
- a clear allocation/copy reduction with a small additive integration?

## Candidates

### `fi-mcp-gateway`

Relevant path: `/Users/cblevins/workspace/services/fi-mcp-gateway/internal/mcpws/mcpws.go`

- `session.Run` decodes each inbound WebSocket message into `jsonrpcEnvelope` to decide request versus notification.
- `session.routeMessage` decodes the same payload again, then decodes `params` and `params.arguments` for routing.
- `policy.AllowAll` was benchmarked separately to confirm policy itself is not the local hot path.

Temporary benchmark command:

```bash
go test ./internal/mcpws -run '^$' -bench 'Benchmark(RouteMessageToolsCall|JSONRPCEnvelopeUnmarshal|PolicyAuthorizeAllowAll)$' -benchmem -count=5
```

Results on Apple M4:

| Benchmark | ns/op | B/op | allocs/op |
| --- | ---: | ---: | ---: |
| `BenchmarkRouteMessageToolsCall` | 2221-2444 | 1608 | 36 |
| `BenchmarkJSONRPCEnvelopeUnmarshal` | 910-1327 | 456 | 11 |
| `BenchmarkPolicyAuthorizeAllowAll` | 0.25 | 0 | 0 |

### `loom-core`

Relevant path: `/Users/cblevins/workspace/services/loom-core/cmd/mcp-loom-widget/main.go`

- `server.Serve` scans newline-delimited JSON-RPC messages.
- `server.dispatch` decodes a single `rpcRequest` envelope and routes by method.

Temporary benchmark command:

```bash
go test ./cmd/mcp-loom-widget -run '^$' -bench 'BenchmarkDispatchToolsCallParseOnly$' -benchmem -count=5
```

Results on Apple M4:

| Benchmark | ns/op | B/op | allocs/op |
| --- | ---: | ---: | ---: |
| `BenchmarkDispatchToolsCallParseOnly` | 849-870 | 456 | 11 |

### `fi-accel` Go Transport

Temporary benchmark command:

```bash
go test ./... -run '^$' -bench 'BenchmarkParseJSONRPCLinesSingleKillTest$' -benchmem -count=5
```

Results on Apple M4:

| Benchmark | ns/op | B/op | allocs/op |
| --- | ---: | ---: | ---: |
| `BenchmarkParseJSONRPCLinesSingleKillTest` | 1972-2002 | 992 | 15 |

The command also passed the normal `go test ./...` sweep in `go/fiaccel`. The linker emitted macOS-version warnings for the bundled static FFI archive, but the test binary linked and passed.

## Decision

Do not integrate the current Go `ParseJSONRPCLines` wrapper into `fi-mcp-gateway` or `loom-core` as-is. For single-message request parsing, the existing service-local `encoding/json` decode is faster. The existing `fi-accel` wrapper is only allocation-competitive against the gateway's full route path, and it does not yet expose the route fields needed to remove the gateway's duplicate decode.

## Next Slice

Add a route-preview helper in `fi-accel` instead of a downstream service change. The helper should parse a single JSON-RPC payload and return:

- `method`
- whether an `id` is present and non-null
- `params.name` for `tools/call`
- input byte count
- parse/validation status

Rerun the `fi-mcp-gateway` route benchmark after that helper exists. Promote service integration only if the helper beats the current `routeMessage` path by at least 20% wall time or preserves behavior while materially reducing allocations.

## Route Preview Slice

The follow-up slice added `ParseJSONRPCRoutePreview` to the Go binding as a pure Go helper so services can use it with or without CGo. It previews a single JSON-RPC payload and reports method, non-null id presence, `tools/call` `params.name`, input bytes, `valid`, and a parse/validation status.

Validation now covers:

- request versus notification id handling, including `id: null`
- invalid JSON
- missing `tools/call` `params.name`

The next service-adoption step is still gated on rerunning `fi-mcp-gateway` route benchmarks with this helper in place.

Temporary benchmark command:

```bash
go test ./... -run '^$' -bench 'BenchmarkParseJSONRPCRoutePreviewToolsCall$' -benchmem -count=5
```

Results on Apple M4:

| Benchmark | ns/op | B/op | allocs/op |
| --- | ---: | ---: | ---: |
| `BenchmarkParseJSONRPCRoutePreviewToolsCall` | 1324-1348 | 1096 | 26 |

This clears the standalone helper side of the kill-test against the earlier `fi-mcp-gateway` `routeMessage` baseline of 2221-2444 ns/op and 1608 B/op. It is not yet a downstream integration result; the next slice should replace the duplicate route decode in `fi-mcp-gateway` and preserve routing behavior under service-local tests.
