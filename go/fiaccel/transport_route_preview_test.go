package fiaccel

import (
	"bytes"
	"testing"
)

var routePreviewSink JSONRPCRoutePreview

func TestParseJSONRPCRoutePreviewToolsCall(t *testing.T) {
	payload := []byte(`{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"lookup","arguments":{"q":"hello"}}}`)

	preview, err := ParseJSONRPCRoutePreview(payload)
	if err != nil {
		t.Fatalf("ParseJSONRPCRoutePreview: %v", err)
	}
	if !preview.Valid || preview.Status != JSONRPCRouteStatusOK {
		t.Fatalf("unexpected status: %+v", preview)
	}
	if preview.Method != "tools/call" || !preview.HasID {
		t.Fatalf("unexpected route fields: %+v", preview)
	}
	if preview.ParamsName == nil || *preview.ParamsName != "lookup" {
		t.Fatalf("unexpected params.name: %+v", preview)
	}
	if !bytes.Equal(preview.ParamsArguments, []byte(`{"q":"hello"}`)) {
		t.Fatalf("unexpected params.arguments: %s", preview.ParamsArguments)
	}
	if preview.InputBytes != len(payload) {
		t.Fatalf("expected input bytes %d, got %d", len(payload), preview.InputBytes)
	}
}

func TestParseJSONRPCRoutePreviewNotification(t *testing.T) {
	preview, err := ParseJSONRPCRoutePreview([]byte(`{"jsonrpc":"2.0","method":"notifications/initialized"}`))
	if err != nil {
		t.Fatalf("ParseJSONRPCRoutePreview: %v", err)
	}
	if preview.HasID {
		t.Fatalf("expected notification without id: %+v", preview)
	}
	if preview.Method != "notifications/initialized" || preview.ParamsName != nil {
		t.Fatalf("unexpected notification preview: %+v", preview)
	}
	if len(preview.ParamsArguments) != 0 {
		t.Fatalf("unexpected notification params.arguments: %s", preview.ParamsArguments)
	}
}

func TestParseJSONRPCRoutePreviewNullID(t *testing.T) {
	preview, err := ParseJSONRPCRoutePreview([]byte(`{"jsonrpc":"2.0","id":null,"method":"ping"}`))
	if err != nil {
		t.Fatalf("ParseJSONRPCRoutePreview: %v", err)
	}
	if preview.HasID {
		t.Fatalf("expected null id to be non-request: %+v", preview)
	}
}

func TestParseJSONRPCRoutePreviewInvalidJSON(t *testing.T) {
	preview, err := ParseJSONRPCRoutePreview([]byte(`{"jsonrpc":"2.0","method":`))
	if err == nil {
		t.Fatal("expected invalid JSON error")
	}
	if preview.Valid || preview.Status != JSONRPCRouteStatusInvalidJSON {
		t.Fatalf("unexpected invalid JSON preview: %+v", preview)
	}
}

func TestParseJSONRPCRoutePreviewInvalidFieldTypes(t *testing.T) {
	preview, err := ParseJSONRPCRoutePreview([]byte(`{"jsonrpc":2,"method":"ping"}`))
	if err == nil {
		t.Fatal("expected invalid jsonrpc type error")
	}
	if preview.Valid || preview.Status != JSONRPCRouteStatusInvalidJSONRPC {
		t.Fatalf("unexpected invalid jsonrpc type preview: %+v", preview)
	}

	preview, err = ParseJSONRPCRoutePreview([]byte(`{"jsonrpc":"2.0","method":7}`))
	if err == nil {
		t.Fatal("expected invalid method type error")
	}
	if preview.Valid || preview.Status != JSONRPCRouteStatusInvalidJSONRPC {
		t.Fatalf("unexpected invalid method type preview: %+v", preview)
	}
}

func TestParseJSONRPCRoutePreviewMissingToolsName(t *testing.T) {
	preview, err := ParseJSONRPCRoutePreview([]byte(`{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"arguments":{}}}`))
	if err == nil {
		t.Fatal("expected missing params.name error")
	}
	if preview.Valid || preview.Status != JSONRPCRouteStatusMissingParamsName {
		t.Fatalf("unexpected missing params.name preview: %+v", preview)
	}
}

func BenchmarkParseJSONRPCRoutePreviewToolsCall(b *testing.B) {
	payload := []byte(`{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"lookup","arguments":{"q":"hello"}}}`)

	for i := 0; i < b.N; i++ {
		preview, err := ParseJSONRPCRoutePreview(payload)
		if err != nil {
			b.Fatal(err)
		}
		routePreviewSink = preview
	}
}
