//go:build !cgo

package fiaccel

import "testing"

func TestEventlogHelpersReturnErrNotAvailable(t *testing.T) {
	if _, err := ProjectEventlog([]byte("{}\n"), nil); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from ProjectEventlog, got %v", err)
	}
	if _, err := SummarizeEventlog([]byte("{}\n"), nil); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from SummarizeEventlog, got %v", err)
	}
}
