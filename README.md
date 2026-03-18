![Banner](assets/banner.png)

# fi-accel

High-performance Rust acceleration library for agent/MCP workflows.

## Features

- **Token Counting**: 10-50x faster than Python tiktoken
- **Embedding Operations**: SIMD-accelerated similarity calculations (50-100x faster)
- **Fuzzy Diff Matching**: Intelligent patch application for drifted files
- **Log Streaming**: Efficient incremental log reading
- **JSON Parsing**: SIMD-accelerated NDJSON parsing plus JSON-RPC helpers
- **Event Log Helpers**: Normalize and summarize JSONL timeline/audit streams
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
| diff | full | full | partial | partial |
| log | full | full | unsupported | full |
| JSON / JSON-RPC | full | full | partial | full |
| eventlog | full | full | full | full |
| transport | full | full | full | full |
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
    truncate_to_tokens,
    split_by_tokens,
    cosine_similarity_matrix,
    FuzzyMatcher,
    LogReader,
    parse_jsonrpc_batch,
    project_eventlog,
    summarize_eventlog,
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

# Token-based text splitting
chunks = split_by_tokens(long_text, chunk_size=500, overlap=50, model="gpt-4")

# Embedding similarity
matrix = cosine_similarity_matrix(query_embeddings, doc_embeddings)

# Fuzzy patch application
matcher = FuzzyMatcher(threshold=0.6)
result = matcher.apply_patch(patch_text, file_content)

# Log streaming
reader = LogReader(initial_content)
new_lines = reader.append(new_content)
errors = reader.filter(r"ERROR|WARN")

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

```go
package main

import (
	"fmt"
	"net/http"

	"gitlab.flexinfer.ai/libs/fi-accel/go/fiaccel"
)

func main() {
	caps := fiaccel.RuntimeCapabilities()
	fmt.Println(caps.Transport)

	messages, _ := fiaccel.ParseJSONRPCLines([]byte("{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1}\n"))
	fmt.Println(len(messages))

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
