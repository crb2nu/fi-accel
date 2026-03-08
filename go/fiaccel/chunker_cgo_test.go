//go:build cgo

package fiaccel

import "testing"

func TestChunkCode(t *testing.T) {
	chunks, err := ChunkCode("def hello(name):\n    return name\n", "python", nil)
	if err != nil {
		t.Fatalf("ChunkCode: %v", err)
	}
	if len(chunks) == 0 {
		t.Fatal("expected code chunks")
	}
	if chunks[0].SymbolKind == nil || *chunks[0].SymbolKind == "" {
		t.Fatalf("expected symbol metadata: %+v", chunks[0])
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
