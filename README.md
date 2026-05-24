![Banner](assets/banner.png)

# fi-accel

High-performance Rust acceleration library for agent/MCP workflows.

Performance claims should be treated as release-backed only when they are
recorded in the [benchmark ledger](docs/benchmark-ledger.md) with command,
host, input shape, and date.

## Features

- **Token Counting**: 10-50x faster than Python tiktoken
- **Embedding Operations**: SIMD-accelerated similarity calculations (50-100x faster)
- **Ranking Helpers**: Hybrid lexical/cosine ranking and reciprocal rank fusion
- **Fuzzy Diff Matching**: Intelligent patch application for drifted files
- **Log Streaming**: Efficient incremental log reading
- **JSON Parsing**: SIMD-accelerated NDJSON parsing plus JSON-RPC helpers
- **Text Ingestion**: HTML-to-text extraction, encoding detection, and strict streaming NDJSON reads
- **Event Log Helpers**: Normalize and summarize JSONL timeline/audit streams
- **Acceleration Scout**: Summarize service hot-path observations for adoption planning
- **Transport Helpers**: JSON-RPC stdio framing, SSE parsing, and MCP Streamable HTTP header extraction
- **TOML Parsing**: Fast TOML parsing with JSON interop and config merging
- **TOON Parsing**: Token Oriented Object Notation for efficient LLM serialization
- **Chunking**: Syntax-aware chunking for code, markdown, and plain text
  - Code chunks now include AST-derived call edges for retrieval and navigation

## Installation

### Python

```bash
pip install fi-accel
```

### JavaScript/TypeScript (WASM)

```bash
pnpm add @flexinfer/fi-accel
```

### Go

The Go binding lives in [`go/fiaccel`](/Users/cblevins/workspace/libs/fi-accel/go/fiaccel) and uses the Rust FFI library when `cgo` is enabled.

## Capability Matrix

`full` means the binding exposes the module directly. `partial` means the binding exposes only part of the module surface. `unsupported` means the module is not exported by that binding.

| Surface | Rust core | Python | Go | WASM |
|---------|-----------|--------|----|------|
| tokenizer | full | full | full | unsupported |
| embedding | full | full | full | full |
| ranking | full | full | unsupported | unsupported |
| diff | full | full | partial | partial |
| log | full | full | unsupported | full |
| JSON / JSON-RPC | full | full | partial | full |
| text / encoding ingest | full | full | unsupported | unsupported |
| eventlog | full | full | full | full |
| acceleration scout | full | full | unsupported | unsupported |
| transport | full | full | full | full |
| markdown text helpers | unsupported | unsupported | full | unsupported |
| TOML | full | full | unsupported | full |
| TOON | full | full | unsupported | full |
| chunker (code) | full | full | full | unsupported |
| chunker (markdown/text) | full | full | full | full |
| runtime `capabilities()` | full | full | full | full |

## Usage

### Python

```python
from fi_accel import (
    count_tokens,
    count_tokens_batch,
    truncate_to_tokens,
    split_by_tokens,
    cosine_similarity_matrix,
    hybrid_rank,
    rrf_fuse,
    FuzzyMatcher,
    LogReader,
    NDJSONReader,
    html_to_text,
    detect_encoding,
    parse_jsonrpc_batch,
    project_eventlog,
    summarize_eventlog,
    summarize_accel_observations,
    parse_toml,
    merge_toml,
    parse_toon,
    to_toon,
    extract_calls,
    chunk_code,
    chunk_markdown,
    chunk_text,
    capabilities,
)

# Token counting
tokens = count_tokens("Hello, world!", model="gpt-4")  # 4
batch_counts = count_tokens_batch(["Hello", "world"], model="gpt-4")
gemma_tokens = count_tokens("Hello, Gemma!", model="gemma")

# Token-based text splitting
chunks = split_by_tokens(long_text, chunk_size=500, overlap=50, model="gpt-4")

# Embedding similarity
matrix = cosine_similarity_matrix(query_embeddings, doc_embeddings)

# Hybrid ranking and reciprocal rank fusion
ranked = hybrid_rank([0, 2, 1], [0.2, 0.95, 0.5], alpha=0.5)
fused = rrf_fuse([[10, 20, 30], [20, 10]], k=60)

# Fuzzy patch application
matcher = FuzzyMatcher(threshold=0.6)
result = matcher.apply_patch(patch_text, file_content)

# Log streaming
reader = LogReader(initial_content)
new_lines = reader.append(new_content)
errors = reader.filter(r"ERROR|WARN")

# Text ingestion
text = html_to_text("<p>Read <a href='https://example.test'>docs</a></p>", preserve_links=True)
encoding = detect_encoding(b"\x93quoted\x94")
stream = NDJSONReader()
items = stream.append(b'{"ok": true}\n')

# JSON-RPC line parsing
messages = parse_jsonrpc_batch(b'{"jsonrpc":"2.0","method":"ping","id":1}\n')

# Event-log normalization and summary
events = project_eventlog(
    b'{"timestamp":"2026-03-18T10:00:00Z","status":"success","agent_id":"codex"}\n',
    event_type_field="status",
)
summary = summarize_eventlog(
    b'{"timestamp":"2026-03-18T10:00:00Z","event_type":"agent.started","agent_id":"codex"}\n'
)
hot_paths = summarize_accel_observations(
    b'{"service":"gateway","operation":"parse_jsonrpc","duration_ms":12.5,"input_bytes":4096}\n'
)

# TOML parsing
config = parse_toml(toml_string)  # Returns dict
merged = merge_toml(base_config, override_config)  # Deep merge

# TOON parsing (Token Oriented Object Notation)
data = parse_toon("key: value\ncount: 42")  # Returns dict
toon_str = to_toon({"name": "test", "items": [1, 2, 3]})  # Returns TOON string

# Chunking
calls = extract_calls("python", source_code)
code_chunks = chunk_code(source_code, "python")
markdown_chunks = chunk_markdown(markdown_doc)
text_chunks = chunk_text(notes)

# Indexing metadata is additive on each chunk:
# file_path, start_column, end_column, imports, parent_type,
# symbol_kind, container_name, container_kind, symbol_path, calls, definitions

# Runtime capability report
caps = capabilities()
```

Tokenizer model support includes OpenAI GPT models, Claude aliases, embeddings,
legacy Codex/Davinci aliases, and Gemma aliases (`gemma`, `gemma-26b`,
`gemma-26b-a4b`). Gemma aliases require the optional `gemma-tokenizer` feature
and a local Hugging Face `tokenizer.json` asset configured with
`FI_ACCEL_GEMMA_TOKENIZER_PATH` or placed at
`$XDG_CACHE_HOME/fi-accel/gemma/tokenizer.json` (falling back to
`~/.cache/fi-accel/gemma/tokenizer.json`). fi-accel does not download model
assets at runtime and no longer approximates Gemma with a GPT tokenizer.
The Rust test suite includes an opt-in exactness fixture for Gemma: run
`FI_ACCEL_GEMMA_TOKENIZER_PATH=/path/to/tokenizer.json cargo test -p fi-accel-core --features gemma-tokenizer gemma_fixture`
to compare fi-accel token counts against the Hugging Face tokenizer on a
100-string fixture. When the environment variable is unset, the fixture test
passes without loading a model asset.

### JavaScript/TypeScript

```typescript
import {
  WasmLogReader,
  parse_ndjson_str,
  parse_jsonrpc_batch_str,
  project_eventlog_str,
  summarize_eventlog_str,
  cosine_similarity,
  apply_fuzzy_patch,
  parse_toml,
  merge_toml,
  parse_toon,
  to_toon,
  chunk_auto,
  chunk_markdown,
  chunk_text,
  detect_content_type,
  capabilities,
} from '@flexinfer/fi-accel';

// Log reader
const reader = new WasmLogReader(initialContent);
const newLines = reader.append(newContent);
const errors = reader.filter('ERROR');

// JSON parsing
const messages = parse_ndjson_str(ndjsonData);
const rpcMessages = parse_jsonrpc_batch_str(jsonrpcLines);
const timeline = project_eventlog_str(ndjsonData, {
  event_type_field: 'status',
  actor_id_field: 'agent_id',
});
const eventSummary = summarize_eventlog_str(ndjsonData, undefined);

// Similarity
const sim = cosine_similarity([1, 0, 0], [1, 0, 0]); // 1.0

// Fuzzy patch
const result = apply_fuzzy_patch(patch, file, 0.6);

// TOML parsing
const config = parse_toml(tomlString);
const merged = merge_toml(baseToml, overlayToml);

// TOON parsing
const data = parse_toon("key: value\ncount: 42");
const toonStr = to_toon({ name: "test", items: [1, 2, 3] });

// Chunking
const autoChunks = chunk_auto(markdownDoc, "README.md");
const docChunks = chunk_markdown(markdownDoc);
const noteChunks = chunk_text(notes);
const kind = detect_content_type("src/example.ts");

// Runtime capability report
const caps = capabilities();
```

### Go

See [Go Markdown helper recipes](docs/go-markdown-recipes.md) for focused
service-edge examples covering fenced model payload cleanup, transcript block
extraction, compact text storage, and capability guards.

```go
package main

import (
	"fmt"
	"net/http"

	"gitlab.flexinfer.ai/libs/fi-accel/go/fiaccel"
)

func main() {
	fmt.Println(fiaccel.Version())

	caps := fiaccel.RuntimeCapabilities()
	fmt.Println(caps.Transport, caps.MarkdownHelpers)

	text := fiaccel.MarkdownToText("# Title\n\nRead the [docs](https://example.test).")
	fmt.Println(text)

	stats := fiaccel.AnalyzeMarkdown("A short note for review.", 0)
	fmt.Println(stats.WordCount, stats.ReadingTimeMinutes)

	messages, _ := fiaccel.ParseJSONRPCLines([]byte("{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1}\n"))
	fmt.Println(len(messages))

	preview, _ := fiaccel.ParseJSONRPCRoutePreview([]byte(`{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"lookup","arguments":{"q":"hello"}}}`))
	fmt.Println(preview.Method, preview.HasID, *preview.ParamsName, string(preview.ParamsArguments))

	route, _ := fiaccel.ExtractJSONRPCPrefixRoute([]byte(`{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"lookup","arguments":{"q":"hello"}}}`))
	fmt.Println(route.Method, route.ToolName)

	events, _ := fiaccel.ParseSSEEvents([]byte("event: message\ndata: hello\n\n"))
	fmt.Println(events[0].Data)

	meta, _ := fiaccel.ExtractMCPHTTPMeta(http.Header{
		"Accept":         []string{"application/json, text/event-stream"},
		"Mcp-Session-Id": []string{"abc123"},
	})
	fmt.Println(meta.AcceptsSSE)

	chunks, _ := fiaccel.ChunkCode("def hello(name):\n    return name\n", "python", nil)
	fmt.Println(chunks[0].SymbolKind)
}
```

## Performance

| Operation | Python | fi-accel | Speedup |
|-----------|--------|----------|---------|
| Token count (10KB) | 5ms | 0.1ms | 50x |
| Similarity matrix (1K x 1K) | 2s | 20ms | 100x |
| Fuzzy patch (10K lines) | 500ms | 5ms | 100x |
| NDJSON parse (1MB) | 50ms | 5ms | 10x |
| JSON-RPC framing (100 msgs) | 20ms | 2ms | 10x |
| JSON-RPC route preview (small tools/call) | gateway route baseline: 2.22-2.44us | Go helper: 1.24-1.33us | ~1.8x |
| JSON-RPC route preview (gateway-shaped tools/call) | pending service rerun | Go helper: 5.12-5.44us | fixture baseline |
| JSON-RPC prefix route extraction (gateway-shaped tools/call) | gateway route gate: 2.22-2.44us, 1608 B/op | Go helper: 235-239ns, 64 B/op | clears gate |
| SSE parse (100 events) | 10ms | 1ms | 10x |
| TOML parse (100KB) | 10ms | 1ms | 10x |
| TOON parse (10KB) | 5ms | 0.5ms | 10x |

## Building from Source

### Prerequisites

- Rust 1.83+
- Python 3.9+ (for PyO3)
- wasm-pack (for WASM)

### Commands

```bash
# Build core library
cargo build --release -p fi-accel-core

# Build Python wheel
cd python && maturin build --release

# Build WASM
cd crates/fi-accel-wasm && wasm-pack build --target bundler --out-dir ../../pkg
```

## License

MIT
