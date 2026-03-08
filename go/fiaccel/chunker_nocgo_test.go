//go:build !cgo

package fiaccel

import "testing"

func TestChunkerHelpersReturnErrNotAvailable(t *testing.T) {
	if _, err := ChunkCode("x", "python", nil); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from ChunkCode, got %v", err)
	}
	if _, err := ChunkMarkdown("x", nil); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from ChunkMarkdown, got %v", err)
	}
	if _, err := ChunkText("x", nil); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from ChunkText, got %v", err)
	}
	if _, err := ChunkAuto("x", "notes.txt", nil); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from ChunkAuto, got %v", err)
	}
	if _, err := DetectContentType("notes.txt"); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from DetectContentType, got %v", err)
	}
}
