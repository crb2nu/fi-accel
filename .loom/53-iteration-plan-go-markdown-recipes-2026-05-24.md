# RALPH Iteration Plan

## Review

- Source brainstorm: `.loom/brainstorm-post-go-markdown-parity-2026-05-22.md`.
- Chosen framing: runner-up F1 + F7, harden the promoted Go Markdown helper
  surface with runnable examples and copyable service recipes.
- Reason for choosing it: the recommended F2 + F4 path has mostly moved into
  downstream `py-codebase-memory` and benchmark-ledger decisions, while Go
  Markdown hardening is local, additive, and immediately shippable in this repo.

## Align

- Slice name: Go Markdown helper recipes.
- Scope in:
  - Add service-shaped tests for fenced model payload cleanup, transcript block
    extraction, compact text conversion, and runnable examples.
  - Add a concise recipe doc for service-edge usage.
  - Link the recipe doc from the Go README section.
- Scope out:
  - New public API.
  - Broad Go parity expansion.
  - Performance claims or benchmark changes.

## Prove

- Run `go test ./...` in `go/fiaccel`.
- Run `CGO_ENABLED=0 go test ./...` in `go/fiaccel`.

## Handoff

- If this slice passes, the Go Markdown surface has a minimal release-quality
  recipe path for service authors without changing runtime behavior.
