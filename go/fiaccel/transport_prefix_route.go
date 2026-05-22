package fiaccel

import (
	"strconv"
	"strings"
)

// JSONRPCPrefixRoute is the minimal route shape needed by gateway prefix
// routing. It intentionally omits JSON-RPC id and raw params.arguments.
type JSONRPCPrefixRoute struct {
	Method       string             `json:"method,omitempty"`
	ToolName     string             `json:"tool_name,omitempty"`
	InputBytes   int                `json:"input_bytes"`
	Valid        bool               `json:"valid"`
	Status       JSONRPCRouteStatus `json:"status"`
	ErrorMessage string             `json:"error,omitempty"`
}

// ExtractJSONRPCPrefixRoute extracts only the top-level method and, for
// tools/call requests, params.name. It is intended for gateway prefix routing
// after a caller has already performed any full-message JSON-RPC validation it
// requires.
func ExtractJSONRPCPrefixRoute(data []byte) (JSONRPCPrefixRoute, error) {
	route := JSONRPCPrefixRoute{
		InputBytes: len(data),
		Status:     JSONRPCRouteStatusOK,
	}

	scanner := jsonRouteScanner{data: data}
	if err := scanner.skipWS(); err != nil {
		return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, err.Error())
	}
	if !scanner.consume('{') {
		return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, "expected JSON object")
	}
	if err := scanner.skipWS(); err != nil {
		return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, err.Error())
	}
	if scanner.consume('}') {
		return invalidPrefixRoute(route, JSONRPCRouteStatusMissingMethod, "method is required")
	}

	methodSeen := false
	paramsStart := -1
	paramsEnd := -1

	for {
		key, err := scanner.readString()
		if err != nil {
			return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, err.Error())
		}
		if err := scanner.skipWS(); err != nil {
			return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, err.Error())
		}
		if !scanner.consume(':') {
			return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, "expected object colon")
		}
		if err := scanner.skipWS(); err != nil {
			return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, err.Error())
		}

		switch {
		case key.equal(data, "method"):
			if scanner.peek() != '"' {
				return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSONRPC, "method must be a string")
			}
			method, err := scanner.readStringValue()
			if err != nil {
				return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, err.Error())
			}
			if method == "" {
				return invalidPrefixRoute(route, JSONRPCRouteStatusMissingMethod, "method is required")
			}
			route.Method = method
			methodSeen = true
			if method != "tools/call" {
				route.Valid = true
				return route, nil
			}
			if paramsStart >= 0 {
				return finishPrefixToolRoute(route, data[paramsStart:paramsEnd])
			}

		case key.equal(data, "params"):
			paramsStart = scanner.pos
			if methodSeen && route.Method == "tools/call" {
				return finishPrefixToolRoute(route, data[paramsStart:])
			}
			if err := scanner.skipValue(); err != nil {
				return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, err.Error())
			}
			paramsEnd = scanner.pos

		default:
			if err := scanner.skipValue(); err != nil {
				return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, err.Error())
			}
		}

		if err := scanner.skipWS(); err != nil {
			return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, err.Error())
		}
		if scanner.consume('}') {
			break
		}
		if !scanner.consume(',') {
			return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, "expected object comma")
		}
		if err := scanner.skipWS(); err != nil {
			return invalidPrefixRoute(route, JSONRPCRouteStatusInvalidJSON, err.Error())
		}
	}

	if !methodSeen {
		return invalidPrefixRoute(route, JSONRPCRouteStatusMissingMethod, "method is required")
	}
	if route.Method == "tools/call" {
		if paramsStart < 0 {
			return invalidPrefixRoute(route, JSONRPCRouteStatusMissingParamsName, errMissingParamsName.Message)
		}
		return finishPrefixToolRoute(route, data[paramsStart:paramsEnd])
	}

	route.Valid = true
	return route, nil
}

func finishPrefixToolRoute(route JSONRPCPrefixRoute, params []byte) (JSONRPCPrefixRoute, error) {
	name, ok, err := scanToolsCallName(params)
	if err != nil {
		return invalidPrefixRoute(route, routePreviewParamStatus(err), err.Error())
	}
	if !ok {
		return invalidPrefixRoute(route, JSONRPCRouteStatusMissingParamsName, errMissingParamsName.Message)
	}
	route.ToolName = name
	route.Valid = true
	return route, nil
}

func scanToolsCallName(data []byte) (string, bool, error) {
	scanner := jsonRouteScanner{data: data}
	if err := scanner.skipWS(); err != nil {
		return "", false, err
	}
	if scanner.matchLiteral("null") {
		return "", false, nil
	}
	if !scanner.consume('{') {
		return "", false, &Error{Code: ErrInvalidArg, Message: "tools/call params must be an object"}
	}
	if err := scanner.skipWS(); err != nil {
		return "", false, err
	}
	if scanner.consume('}') {
		return "", false, nil
	}

	for {
		key, err := scanner.readString()
		if err != nil {
			return "", false, err
		}
		if err := scanner.skipWS(); err != nil {
			return "", false, err
		}
		if !scanner.consume(':') {
			return "", false, errJSONRouteSyntax("expected object colon")
		}
		if err := scanner.skipWS(); err != nil {
			return "", false, err
		}

		if key.equal(data, "name") {
			if scanner.peek() == 'n' && scanner.matchLiteral("null") {
				return "", false, nil
			}
			if scanner.peek() != '"' {
				return "", false, &Error{Code: ErrInvalidArg, Message: "tools/call params.name must be a string"}
			}
			name, err := scanner.readStringValue()
			if err != nil {
				return "", false, err
			}
			if strings.TrimSpace(name) == "" {
				return "", false, nil
			}
			return name, true, nil
		}

		if err := scanner.skipValue(); err != nil {
			return "", false, err
		}
		if err := scanner.skipWS(); err != nil {
			return "", false, err
		}
		if scanner.consume('}') {
			return "", false, nil
		}
		if !scanner.consume(',') {
			return "", false, errJSONRouteSyntax("expected object comma")
		}
		if err := scanner.skipWS(); err != nil {
			return "", false, err
		}
	}
}

type jsonRouteScanner struct {
	data []byte
	pos  int
}

type jsonRouteString struct {
	start   int
	end     int
	qstart  int
	qend    int
	escaped bool
}

func (s *jsonRouteScanner) skipWS() error {
	for s.pos < len(s.data) {
		switch s.data[s.pos] {
		case ' ', '\n', '\r', '\t':
			s.pos++
		default:
			return nil
		}
	}
	return nil
}

func (s *jsonRouteScanner) peek() byte {
	if s.pos >= len(s.data) {
		return 0
	}
	return s.data[s.pos]
}

func (s *jsonRouteScanner) consume(ch byte) bool {
	if s.pos < len(s.data) && s.data[s.pos] == ch {
		s.pos++
		return true
	}
	return false
}

func (s *jsonRouteScanner) matchLiteral(lit string) bool {
	if len(s.data)-s.pos < len(lit) {
		return false
	}
	for i := range lit {
		if s.data[s.pos+i] != lit[i] {
			return false
		}
	}
	s.pos += len(lit)
	return true
}

func (s *jsonRouteScanner) readStringValue() (string, error) {
	token, err := s.readString()
	if err != nil {
		return "", err
	}
	return token.value(s.data)
}

func (s *jsonRouteScanner) readString() (jsonRouteString, error) {
	if !s.consume('"') {
		return jsonRouteString{}, errJSONRouteSyntax("expected string")
	}
	token := jsonRouteString{qstart: s.pos - 1, start: s.pos}

	for s.pos < len(s.data) {
		ch := s.data[s.pos]
		switch {
		case ch == '"':
			token.end = s.pos
			s.pos++
			token.qend = s.pos
			return token, nil
		case ch == '\\':
			token.escaped = true
			s.pos++
			if s.pos >= len(s.data) {
				return jsonRouteString{}, errJSONRouteSyntax("unterminated escape")
			}
			esc := s.data[s.pos]
			switch esc {
			case '"', '\\', '/', 'b', 'f', 'n', 'r', 't':
				s.pos++
			case 'u':
				if s.pos+4 >= len(s.data) {
					return jsonRouteString{}, errJSONRouteSyntax("invalid unicode escape")
				}
				for i := 1; i <= 4; i++ {
					if !isHex(s.data[s.pos+i]) {
						return jsonRouteString{}, errJSONRouteSyntax("invalid unicode escape")
					}
				}
				s.pos += 5
			default:
				return jsonRouteString{}, errJSONRouteSyntax("invalid escape")
			}
		case ch < 0x20:
			return jsonRouteString{}, errJSONRouteSyntax("invalid control character")
		default:
			s.pos++
		}
	}

	return jsonRouteString{}, errJSONRouteSyntax("unterminated string")
}

func (s *jsonRouteScanner) skipValue() error {
	if err := s.skipWS(); err != nil {
		return err
	}
	switch s.peek() {
	case '"':
		_, err := s.readString()
		return err
	case '{':
		return s.skipObject()
	case '[':
		return s.skipArray()
	case 't':
		if s.matchLiteral("true") {
			return nil
		}
	case 'f':
		if s.matchLiteral("false") {
			return nil
		}
	case 'n':
		if s.matchLiteral("null") {
			return nil
		}
	default:
		if s.skipNumber() {
			return nil
		}
	}
	return errJSONRouteSyntax("expected JSON value")
}

func (s *jsonRouteScanner) skipObject() error {
	if !s.consume('{') {
		return errJSONRouteSyntax("expected object")
	}
	if err := s.skipWS(); err != nil {
		return err
	}
	if s.consume('}') {
		return nil
	}
	for {
		if _, err := s.readString(); err != nil {
			return err
		}
		if err := s.skipWS(); err != nil {
			return err
		}
		if !s.consume(':') {
			return errJSONRouteSyntax("expected object colon")
		}
		if err := s.skipValue(); err != nil {
			return err
		}
		if err := s.skipWS(); err != nil {
			return err
		}
		if s.consume('}') {
			return nil
		}
		if !s.consume(',') {
			return errJSONRouteSyntax("expected object comma")
		}
		if err := s.skipWS(); err != nil {
			return err
		}
	}
}

func (s *jsonRouteScanner) skipArray() error {
	if !s.consume('[') {
		return errJSONRouteSyntax("expected array")
	}
	if err := s.skipWS(); err != nil {
		return err
	}
	if s.consume(']') {
		return nil
	}
	for {
		if err := s.skipValue(); err != nil {
			return err
		}
		if err := s.skipWS(); err != nil {
			return err
		}
		if s.consume(']') {
			return nil
		}
		if !s.consume(',') {
			return errJSONRouteSyntax("expected array comma")
		}
		if err := s.skipWS(); err != nil {
			return err
		}
	}
}

func (s *jsonRouteScanner) skipNumber() bool {
	start := s.pos
	if s.consume('-') && s.pos >= len(s.data) {
		return false
	}
	if s.consume('0') {
		// Leading zero is only valid as the whole integer part.
	} else if isDigitOneToNine(s.peek()) {
		for isDigit(s.peek()) {
			s.pos++
		}
	} else {
		s.pos = start
		return false
	}
	if s.consume('.') {
		if !isDigit(s.peek()) {
			s.pos = start
			return false
		}
		for isDigit(s.peek()) {
			s.pos++
		}
	}
	if s.peek() == 'e' || s.peek() == 'E' {
		s.pos++
		if s.peek() == '+' || s.peek() == '-' {
			s.pos++
		}
		if !isDigit(s.peek()) {
			s.pos = start
			return false
		}
		for isDigit(s.peek()) {
			s.pos++
		}
	}
	return true
}

func (s jsonRouteString) equal(data []byte, value string) bool {
	if s.escaped {
		decoded, err := s.value(data)
		return err == nil && decoded == value
	}
	if s.end-s.start != len(value) {
		return false
	}
	for i := range value {
		if data[s.start+i] != value[i] {
			return false
		}
	}
	return true
}

func (s jsonRouteString) value(data []byte) (string, error) {
	if !s.escaped {
		return string(data[s.start:s.end]), nil
	}
	return strconv.Unquote(string(data[s.qstart:s.qend]))
}

type errJSONRouteSyntax string

func (e errJSONRouteSyntax) Error() string {
	return string(e)
}

func isHex(ch byte) bool {
	return (ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')
}

func isDigit(ch byte) bool {
	return ch >= '0' && ch <= '9'
}

func isDigitOneToNine(ch byte) bool {
	return ch >= '1' && ch <= '9'
}

func invalidPrefixRoute(route JSONRPCPrefixRoute, status JSONRPCRouteStatus, message string) (JSONRPCPrefixRoute, error) {
	route.Valid = false
	route.Status = status
	route.ErrorMessage = message
	return route, &Error{Code: ErrInvalidArg, Message: message}
}
