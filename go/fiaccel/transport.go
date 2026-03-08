package fiaccel

import (
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
