package fiaccel

import "testing"

func TestVersion(t *testing.T) {
	if got := Version(); got != "0.1.0" {
		t.Fatalf("Version() = %q, want %q", got, "0.1.0")
	}
}
