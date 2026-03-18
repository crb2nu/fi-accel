//go:build cgo

package fiaccel

import (
	"net/http"
	"testing"
)

func TestRuntimeCapabilitiesCgo(t *testing.T) {
	caps := RuntimeCapabilities()
	if !caps.Transport || !caps.Eventlog || !caps.FFIAvailable || !caps.SIMDJSON {
		t.Fatalf("unexpected cgo capabilities: %+v", caps)
	}
}

func TestParseJSONRPCLines(t *testing.T) {
	messages, err := ParseJSONRPCLines([]byte("{\"jsonrpc\":\"2.0\",\"method\":\"ping\",\"id\":1}\n"))
	if err != nil {
		t.Fatalf("ParseJSONRPCLines: %v", err)
	}
	if len(messages) != 1 {
		t.Fatalf("expected 1 message, got %d", len(messages))
	}
	if messages[0].Method == nil || *messages[0].Method != "ping" {
		t.Fatalf("unexpected method: %+v", messages[0])
	}
}

func TestSerializeJSONRPCLines(t *testing.T) {
	method := "ping"
	output, err := SerializeJSONRPCLines([]JSONRPCMessage{
		{JSONRPC: "2.0", ID: []byte("1"), Method: &method},
	})
	if err != nil {
		t.Fatalf("SerializeJSONRPCLines: %v", err)
	}
	if string(output) != "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}" {
		t.Fatalf("unexpected output: %q", output)
	}
}

func TestParseSSEEvents(t *testing.T) {
	events, err := ParseSSEEvents([]byte("id: 1\nevent: message\ndata: hello\n\n"))
	if err != nil {
		t.Fatalf("ParseSSEEvents: %v", err)
	}
	if len(events) != 1 {
		t.Fatalf("expected 1 event, got %d", len(events))
	}
	if events[0].ID == nil || *events[0].ID != "1" || events[0].Data != "hello" {
		t.Fatalf("unexpected event: %+v", events[0])
	}
}

func TestExtractMCPHTTPMeta(t *testing.T) {
	meta, err := ExtractMCPHTTPMeta(http.Header{
		"Mcp-Session-Id":       []string{"abc"},
		"MCP-Protocol-Version": []string{"2025-11-05"},
		"Last-Event-Id":        []string{"42"},
		"Accept":               []string{"application/json, text/event-stream"},
	})
	if err != nil {
		t.Fatalf("ExtractMCPHTTPMeta: %v", err)
	}
	if meta.SessionID == nil || *meta.SessionID != "abc" {
		t.Fatalf("unexpected session id: %+v", meta)
	}
	if !meta.AcceptsSSE {
		t.Fatalf("expected AcceptsSSE=true: %+v", meta)
	}
}
