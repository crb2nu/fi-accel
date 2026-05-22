package fiaccel

import (
	"bytes"
	"testing"
)

var routePreviewSink JSONRPCRoutePreview
var prefixRouteSink JSONRPCPrefixRoute

var routePreviewSmallToolsCallPayload = []byte(`{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"lookup","arguments":{"q":"hello"}}}`)

var routePreviewGatewayToolsCallPayload = []byte(`{"jsonrpc":"2.0","id":"req-2026-05-20T18:40:00.000Z-00042","method":"tools/call","params":{"name":"agent_context__agent_context_recall_enhanced","arguments":{"query":"fi-accel route preview production payload benchmark","namespace":"fi-accel/ralph-loop","cross_agent":true,"include_decisions":true,"include_summaries":true,"include_tasks":true,"scope":["context","memory"],"token_budget":4000,"file_context":"services/fi-mcp-gateway/internal/mcpws/mcpws.go","metadata":{"tenant":"flexinfer","profile":"default","request_source":"websocket","trace_id":"01HVZ7W2N6V9P9FX4T6T2FQ6KM"}}}}`)

func TestParseJSONRPCRoutePreviewToolsCall(t *testing.T) {
	payload := routePreviewSmallToolsCallPayload

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

func TestParseJSONRPCRoutePreviewGatewayToolsCall(t *testing.T) {
	preview, err := ParseJSONRPCRoutePreview(routePreviewGatewayToolsCallPayload)
	if err != nil {
		t.Fatalf("ParseJSONRPCRoutePreview: %v", err)
	}
	if !preview.Valid || preview.Status != JSONRPCRouteStatusOK {
		t.Fatalf("unexpected status: %+v", preview)
	}
	if preview.Method != "tools/call" || !preview.HasID {
		t.Fatalf("unexpected route fields: %+v", preview)
	}
	if preview.ParamsName == nil || *preview.ParamsName != "agent_context__agent_context_recall_enhanced" {
		t.Fatalf("unexpected params.name: %+v", preview)
	}
	if !bytes.Contains(preview.ParamsArguments, []byte(`"token_budget":4000`)) {
		t.Fatalf("expected production-shaped arguments, got: %s", preview.ParamsArguments)
	}
	if preview.InputBytes != len(routePreviewGatewayToolsCallPayload) {
		t.Fatalf("expected input bytes %d, got %d", len(routePreviewGatewayToolsCallPayload), preview.InputBytes)
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

func TestExtractJSONRPCPrefixRouteGatewayToolsCall(t *testing.T) {
	route, err := ExtractJSONRPCPrefixRoute(routePreviewGatewayToolsCallPayload)
	if err != nil {
		t.Fatalf("ExtractJSONRPCPrefixRoute: %v", err)
	}
	if !route.Valid || route.Status != JSONRPCRouteStatusOK {
		t.Fatalf("unexpected status: %+v", route)
	}
	if route.Method != "tools/call" {
		t.Fatalf("unexpected method: %+v", route)
	}
	if route.ToolName != "agent_context__agent_context_recall_enhanced" {
		t.Fatalf("unexpected tool name: %+v", route)
	}
	if route.InputBytes != len(routePreviewGatewayToolsCallPayload) {
		t.Fatalf("expected input bytes %d, got %d", len(routePreviewGatewayToolsCallPayload), route.InputBytes)
	}
}

func TestExtractJSONRPCPrefixRouteParamsBeforeMethod(t *testing.T) {
	payload := []byte(`{"params":{"name":"lookup","arguments":{"q":"hello"}},"method":"tools/call"}`)

	route, err := ExtractJSONRPCPrefixRoute(payload)
	if err != nil {
		t.Fatalf("ExtractJSONRPCPrefixRoute: %v", err)
	}
	if route.ToolName != "lookup" {
		t.Fatalf("unexpected tool name: %+v", route)
	}
}

func TestExtractJSONRPCPrefixRouteNotificationMethod(t *testing.T) {
	route, err := ExtractJSONRPCPrefixRoute([]byte(`{"jsonrpc":"2.0","method":"notifications/initialized","params":{"ignored":true}}`))
	if err != nil {
		t.Fatalf("ExtractJSONRPCPrefixRoute: %v", err)
	}
	if route.Method != "notifications/initialized" || route.ToolName != "" {
		t.Fatalf("unexpected prefix route: %+v", route)
	}
}

func TestExtractJSONRPCPrefixRouteInvalidInputs(t *testing.T) {
	route, err := ExtractJSONRPCPrefixRoute([]byte(`{"jsonrpc":"2.0","method":7}`))
	if err == nil {
		t.Fatal("expected invalid method type error")
	}
	if route.Valid || route.Status != JSONRPCRouteStatusInvalidJSONRPC {
		t.Fatalf("unexpected invalid method preview: %+v", route)
	}

	route, err = ExtractJSONRPCPrefixRoute([]byte(`{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"arguments":{}}}`))
	if err == nil {
		t.Fatal("expected missing params.name error")
	}
	if route.Valid || route.Status != JSONRPCRouteStatusMissingParamsName {
		t.Fatalf("unexpected missing params.name route: %+v", route)
	}

	route, err = ExtractJSONRPCPrefixRoute([]byte(`{"jsonrpc":"2.0","id":1,"method":"tools/call","params":[]}`))
	if err == nil {
		t.Fatal("expected invalid params error")
	}
	if route.Valid || route.Status != JSONRPCRouteStatusInvalidParams {
		t.Fatalf("unexpected invalid params route: %+v", route)
	}
}

func BenchmarkParseJSONRPCRoutePreviewToolsCall(b *testing.B) {
	payload := routePreviewSmallToolsCallPayload
	b.SetBytes(int64(len(payload)))

	for i := 0; i < b.N; i++ {
		preview, err := ParseJSONRPCRoutePreview(payload)
		if err != nil {
			b.Fatal(err)
		}
		routePreviewSink = preview
	}
}

func BenchmarkExtractJSONRPCPrefixRouteGatewayToolsCall(b *testing.B) {
	payload := routePreviewGatewayToolsCallPayload
	b.SetBytes(int64(len(payload)))

	for i := 0; i < b.N; i++ {
		route, err := ExtractJSONRPCPrefixRoute(payload)
		if err != nil {
			b.Fatal(err)
		}
		prefixRouteSink = route
	}
}

func BenchmarkParseJSONRPCRoutePreviewGatewayToolsCall(b *testing.B) {
	payload := routePreviewGatewayToolsCallPayload
	b.SetBytes(int64(len(payload)))

	for i := 0; i < b.N; i++ {
		preview, err := ParseJSONRPCRoutePreview(payload)
		if err != nil {
			b.Fatal(err)
		}
		routePreviewSink = preview
	}
}
