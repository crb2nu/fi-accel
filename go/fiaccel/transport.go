package fiaccel

import (
	"bytes"
	"encoding/json"
	"net/http"
	"sort"
	"strings"
)

// JSONRPCError is the JSON-RPC error payload.
type JSONRPCError struct {
	Code    int             `json:"code"`
	Message string          `json:"message"`
	Data    json.RawMessage `json:"data,omitempty"`
}

// JSONRPCMessage is a single JSON-RPC request, notification, or response.
type JSONRPCMessage struct {
	JSONRPC string          `json:"jsonrpc"`
	ID      json.RawMessage `json:"id,omitempty"`
	Method  *string         `json:"method,omitempty"`
	Params  json.RawMessage `json:"params,omitempty"`
	Result  json.RawMessage `json:"result,omitempty"`
	Error   *JSONRPCError   `json:"error,omitempty"`
}

// SSEEvent is a parsed Server-Sent Event.
type SSEEvent struct {
	ID    *string `json:"id,omitempty"`
	Event *string `json:"event,omitempty"`
	Data  string  `json:"data"`
}

// McpHTTPMeta is the MCP Streamable HTTP metadata derived from request headers.
type McpHTTPMeta struct {
	SessionID       *string `json:"session_id,omitempty"`
	ProtocolVersion *string `json:"protocol_version,omitempty"`
	LastEventID     *string `json:"last_event_id,omitempty"`
	AcceptsSSE      bool    `json:"accepts_sse"`
}

// JSONRPCRouteStatus is the validation status for a route preview.
type JSONRPCRouteStatus string

const (
	JSONRPCRouteStatusOK                JSONRPCRouteStatus = "ok"
	JSONRPCRouteStatusInvalidJSON       JSONRPCRouteStatus = "invalid_json"
	JSONRPCRouteStatusInvalidJSONRPC    JSONRPCRouteStatus = "invalid_jsonrpc"
	JSONRPCRouteStatusMissingMethod     JSONRPCRouteStatus = "missing_method"
	JSONRPCRouteStatusInvalidParams     JSONRPCRouteStatus = "invalid_params"
	JSONRPCRouteStatusMissingParamsName JSONRPCRouteStatus = "missing_params_name"
)

// JSONRPCRoutePreview is the minimal routing information needed by MCP
// gateways before they perform method-specific decoding.
type JSONRPCRoutePreview struct {
	Method          string             `json:"method,omitempty"`
	HasID           bool               `json:"has_id"`
	ParamsName      *string            `json:"params_name,omitempty"`
	ParamsArguments json.RawMessage    `json:"params_arguments,omitempty"`
	InputBytes      int                `json:"input_bytes"`
	Valid           bool               `json:"valid"`
	Status          JSONRPCRouteStatus `json:"status"`
	ErrorMessage    string             `json:"error,omitempty"`
}

// ParseJSONRPCLines parses newline-delimited JSON-RPC messages.
func ParseJSONRPCLines(data []byte) ([]JSONRPCMessage, error) {
	raw, err := parseJSONRPCLines(data)
	if err != nil {
		return nil, err
	}
	if len(raw) == 0 {
		return nil, nil
	}

	var messages []JSONRPCMessage
	if err := json.Unmarshal(raw, &messages); err != nil {
		return nil, &Error{Code: ErrFailed, Message: err.Error()}
	}
	return messages, nil
}

// ParseJSONRPCRoutePreview parses a single JSON-RPC payload into routing
// metadata. It is intentionally pure Go so services can use it without CGo.
func ParseJSONRPCRoutePreview(data []byte) (JSONRPCRoutePreview, error) {
	preview := JSONRPCRoutePreview{
		InputBytes: len(data),
		Status:     JSONRPCRouteStatusOK,
	}

	var envelope jsonrpcRouteEnvelope
	if err := json.Unmarshal(data, &envelope); err != nil {
		return invalidRoutePreview(preview, JSONRPCRouteStatusInvalidJSON, err.Error())
	}

	jsonrpc, err := envelope.JSONRPC.stringValue("jsonrpc")
	if err != nil || jsonrpc != "2.0" {
		return invalidRoutePreview(preview, JSONRPCRouteStatusInvalidJSONRPC, "jsonrpc must be \"2.0\"")
	}

	method, err := envelope.Method.stringValue("method")
	if err != nil {
		return invalidRoutePreview(preview, JSONRPCRouteStatusInvalidJSONRPC, err.Error())
	}
	if method == "" {
		return invalidRoutePreview(preview, JSONRPCRouteStatusMissingMethod, "method is required")
	}
	preview.Method = method
	preview.HasID = envelope.ID.hasNonNullValue()

	if method == "tools/call" {
		name, args, err := routePreviewToolParams(envelope.Params)
		if err != nil {
			return invalidRoutePreview(preview, routePreviewParamStatus(err), err.Error())
		}
		preview.ParamsName = &name
		preview.ParamsArguments = args
	}

	preview.Valid = true
	return preview, nil
}

// SerializeJSONRPCLines serializes JSON-RPC messages into line-delimited framing.
func SerializeJSONRPCLines(messages []JSONRPCMessage) ([]byte, error) {
	payload, err := json.Marshal(messages)
	if err != nil {
		return nil, &Error{Code: ErrInvalidArg, Message: err.Error()}
	}
	return serializeJSONRPCLines(payload)
}

// ParseSSEEvents parses an SSE stream into discrete events.
func ParseSSEEvents(data []byte) ([]SSEEvent, error) {
	raw, err := parseSSEEvents(data)
	if err != nil {
		return nil, err
	}
	if len(raw) == 0 {
		return nil, nil
	}

	var events []SSEEvent
	if err := json.Unmarshal(raw, &events); err != nil {
		return nil, &Error{Code: ErrFailed, Message: err.Error()}
	}
	return events, nil
}

// ExtractMCPHTTPMeta extracts MCP transport metadata from an HTTP header map.
func ExtractMCPHTTPMeta(headers http.Header) (McpHTTPMeta, error) {
	return ExtractMCPHTTPMetaBytes(encodeHTTPHeaders(headers))
}

// ExtractMCPHTTPMetaBytes extracts MCP transport metadata from a raw header block.
func ExtractMCPHTTPMetaBytes(headerBlock []byte) (McpHTTPMeta, error) {
	raw, err := extractMCPHTTPMeta(headerBlock)
	if err != nil {
		return McpHTTPMeta{}, err
	}

	var meta McpHTTPMeta
	if len(raw) == 0 {
		return meta, nil
	}
	if err := json.Unmarshal(raw, &meta); err != nil {
		return McpHTTPMeta{}, &Error{Code: ErrFailed, Message: err.Error()}
	}
	return meta, nil
}

func encodeHTTPHeaders(headers http.Header) []byte {
	if len(headers) == 0 {
		return nil
	}

	keys := make([]string, 0, len(headers))
	for key := range headers {
		keys = append(keys, key)
	}
	sort.Strings(keys)

	var builder strings.Builder
	for _, key := range keys {
		for _, value := range headers[key] {
			builder.WriteString(key)
			builder.WriteString(": ")
			builder.WriteString(value)
			builder.WriteString("\r\n")
		}
	}
	return []byte(builder.String())
}

type jsonRawField struct {
	Present bool
	Value   json.RawMessage
}

func (field *jsonRawField) UnmarshalJSON(data []byte) error {
	field.Present = true
	field.Value = append(field.Value[:0], data...)
	return nil
}

func (field jsonRawField) hasNonNullValue() bool {
	if !field.Present {
		return false
	}
	return !bytes.Equal(bytes.TrimSpace(field.Value), []byte("null"))
}

func (field jsonRawField) stringValue(name string) (string, error) {
	if !field.hasNonNullValue() {
		return "", nil
	}

	var value string
	if err := json.Unmarshal(field.Value, &value); err != nil {
		return "", &Error{Code: ErrInvalidArg, Message: name + " must be a string"}
	}
	return value, nil
}

type jsonrpcRouteEnvelope struct {
	JSONRPC jsonRawField `json:"jsonrpc"`
	ID      jsonRawField `json:"id"`
	Method  jsonRawField `json:"method"`
	Params  jsonRawField `json:"params"`
}

type jsonrpcToolsCallParams struct {
	Name      *string         `json:"name"`
	Arguments json.RawMessage `json:"arguments,omitempty"`
}

var errMissingParamsName = &Error{Code: ErrInvalidArg, Message: "tools/call params.name is required"}

func routePreviewToolParams(params jsonRawField) (string, json.RawMessage, error) {
	if !params.hasNonNullValue() {
		return "", nil, errMissingParamsName
	}

	var decoded jsonrpcToolsCallParams
	if err := json.Unmarshal(params.Value, &decoded); err != nil {
		return "", nil, &Error{Code: ErrInvalidArg, Message: "tools/call params must be an object"}
	}
	if decoded.Name == nil || *decoded.Name == "" {
		return "", nil, errMissingParamsName
	}
	return *decoded.Name, decoded.Arguments, nil
}

func routePreviewParamStatus(err error) JSONRPCRouteStatus {
	if err == errMissingParamsName {
		return JSONRPCRouteStatusMissingParamsName
	}
	return JSONRPCRouteStatusInvalidParams
}

func invalidRoutePreview(preview JSONRPCRoutePreview, status JSONRPCRouteStatus, message string) (JSONRPCRoutePreview, error) {
	preview.Valid = false
	preview.Status = status
	preview.ErrorMessage = message
	return preview, &Error{Code: ErrInvalidArg, Message: message}
}
