//go:build cgo

package fiaccel

import "testing"

func TestChunkCode(t *testing.T) {
	config := &ChunkConfig{FilePath: "demo.py"}
	chunks, err := ChunkCode("def helper(name):\n    return name.upper()\n\ndef hello(name):\n    message = format_name(name)\n    logger.info(message)\n    return emit(helper(message))\n", "python", config)
	if err != nil {
		t.Fatalf("ChunkCode: %v", err)
	}
	if len(chunks) < 2 {
		t.Fatalf("expected at least 2 code chunks, got %d", len(chunks))
	}

	var chunk *Chunk
	for i := range chunks {
		if chunks[i].Name != nil && *chunks[i].Name == "hello" {
			chunk = &chunks[i]
			break
		}
	}
	if chunk == nil {
		t.Fatalf("expected hello chunk, got %+v", chunks)
	}
	if chunk.SymbolKind == nil || *chunk.SymbolKind == "" {
		t.Fatalf("expected symbol metadata: %+v", *chunk)
	}
	if chunk.ChunkType != "function" {
		t.Fatalf("expected chunk_type=function: %+v", *chunk)
	}
	if chunk.FilePath == nil || *chunk.FilePath != "demo.py" {
		t.Fatalf("expected file_path metadata: %+v", *chunk)
	}
	if len(chunk.Calls) != 4 {
		t.Fatalf("expected call metadata: %+v", *chunk)
	}
	expectedCalls := map[string]bool{
		"emit": false,
		"format_name": false,
		"helper": false,
		"logger.info": false,
	}
	for _, call := range chunk.Calls {
		if _, ok := expectedCalls[call]; ok {
			expectedCalls[call] = true
		}
	}
	for call, seen := range expectedCalls {
		if !seen {
			t.Fatalf("expected call %q in %+v", call, *chunk)
		}
	}
	if len(chunk.Definitions) != 1 || chunk.Definitions[0] != "hello" {
		t.Fatalf("expected definition metadata: %+v", *chunk)
	}
}

func TestChunkMarkdown(t *testing.T) {
	chunks, err := ChunkMarkdown("# Title\n\nBody\n", nil)
	if err != nil {
		t.Fatalf("ChunkMarkdown: %v", err)
	}
	if len(chunks) != 1 {
		t.Fatalf("expected 1 markdown chunk, got %d", len(chunks))
	}
	if chunks[0].ContainerKind == nil || *chunks[0].ContainerKind != "document" {
		t.Fatalf("unexpected metadata: %+v", chunks[0])
	}
}

func TestChunkAuto(t *testing.T) {
	chunks, err := ChunkAuto("hello\n\nworld\n", "notes.txt", nil)
	if err != nil {
		t.Fatalf("ChunkAuto: %v", err)
	}
	if len(chunks) != 2 {
		t.Fatalf("expected 2 chunks, got %d", len(chunks))
	}
}

func TestDetectContentType(t *testing.T) {
	kind, err := DetectContentType("README.md")
	if err != nil {
		t.Fatalf("DetectContentType: %v", err)
	}
	if kind != "markdown" {
		t.Fatalf("expected markdown, got %q", kind)
	}
}
