//go:build !cgo

package fiaccel

import "testing"

func TestRuntimeCapabilitiesNoCgo(t *testing.T) {
	caps := RuntimeCapabilities()
	if caps.Transport || caps.Eventlog || caps.FFIAvailable || caps.SIMDJSON {
		t.Fatalf("unexpected !cgo capabilities: %+v", caps)
	}
}

func TestTransportHelpersReturnErrNotAvailable(t *testing.T) {
	if _, err := ParseJSONRPCLines([]byte("{}\n")); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from ParseJSONRPCLines, got %v", err)
	}
	if _, err := SerializeJSONRPCLines(nil); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from SerializeJSONRPCLines, got %v", err)
	}
	if _, err := ParseSSEEvents([]byte("data: hi\n\n")); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from ParseSSEEvents, got %v", err)
	}
	if _, err := ExtractMCPHTTPMetaBytes([]byte("Accept: text/event-stream\r\n")); err != ErrNotAvailable {
		t.Fatalf("expected ErrNotAvailable from ExtractMCPHTTPMetaBytes, got %v", err)
	}
}
