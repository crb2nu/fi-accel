# Go Markdown Helper Recipes

The Go Markdown helpers are pure Go and work in both `cgo` and `CGO_ENABLED=0`
builds. They are intended for service edges that need small, predictable
cleanup before routing, storage, or metric extraction.

## Strip a wrapped model payload

Use `StripMarkdownCodeFence` when the whole response is a single fenced
triple-backtick block. The helper intentionally returns the original trimmed
input when prose surrounds the fence, so callers do not accidentally discard
context.

```go
payload := "```json\n{\"tool\":\"lookup\",\"arguments\":{\"q\":\"hello\"}}\n```"
clean := fiaccel.StripMarkdownCodeFence(payload)
```

Expected `clean`:

```json
{"tool":"lookup","arguments":{"q":"hello"}}
```

## Extract fenced blocks from a transcript

Use `ExtractMarkdownCodeBlocks` when a service wants to inspect fenced payloads
inside a larger assistant transcript.

```go
blocks := fiaccel.ExtractMarkdownCodeBlocks(transcript)
for _, block := range blocks {
    fmt.Println(block.Language, block.Content)
}
```

The helper preserves block order and trims only trailing line endings from each
block body.

## Store compact user-visible text

Use `MarkdownToText` before indexing, previewing, or storing Markdown where
fenced code payloads should not become searchable prose.

```go
text := fiaccel.MarkdownToText(markdownNote)
stats := fiaccel.AnalyzeMarkdown(markdownNote, 0)
```

`AnalyzeMarkdown` uses the same text normalization as `MarkdownToText`, then
reports word count, an approximate token estimate, and a reading-time estimate.

## Capability guard

The helper surface is advertised through the Go runtime capability report:

```go
caps := fiaccel.RuntimeCapabilities()
if !caps.MarkdownHelpers {
    return errors.New("fi-accel Go markdown helpers unavailable")
}
```

This should be true for both normal and `CGO_ENABLED=0` builds.
