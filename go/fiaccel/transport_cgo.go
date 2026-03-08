//go:build cgo

package fiaccel

/*
#include "fi_accel.h"
*/
import "C"

import "unsafe"

func parseJSONRPCLines(data []byte) ([]byte, error) {
	if len(data) == 0 {
		return []byte("[]"), nil
	}

	var cerr C.struct_FiError
	result := C.fi_parse_jsonrpc_lines(
		(*C.uint8_t)(unsafe.Pointer(&data[0])),
		C.uintptr_t(len(data)),
		&cerr,
	)
	if result.data == nil {
		return nil, extractError(&cerr)
	}
	defer C.fi_buffer_free(result)

	return C.GoBytes(unsafe.Pointer(result.data), C.int(result.len)), nil
}

func serializeJSONRPCLines(jsonArray []byte) ([]byte, error) {
	if len(jsonArray) == 0 {
		return []byte{}, nil
	}

	var cerr C.struct_FiError
	result := C.fi_serialize_jsonrpc_lines(
		(*C.uint8_t)(unsafe.Pointer(&jsonArray[0])),
		C.uintptr_t(len(jsonArray)),
		&cerr,
	)
	if result.data == nil {
		return nil, extractError(&cerr)
	}
	defer C.fi_buffer_free(result)

	return C.GoBytes(unsafe.Pointer(result.data), C.int(result.len)), nil
}

func parseSSEEvents(data []byte) ([]byte, error) {
	if len(data) == 0 {
		return []byte("[]"), nil
	}

	var cerr C.struct_FiError
	result := C.fi_parse_sse_events(
		(*C.uint8_t)(unsafe.Pointer(&data[0])),
		C.uintptr_t(len(data)),
		&cerr,
	)
	if result.data == nil {
		return nil, extractError(&cerr)
	}
	defer C.fi_buffer_free(result)

	return C.GoBytes(unsafe.Pointer(result.data), C.int(result.len)), nil
}

func extractMCPHTTPMeta(headers []byte) ([]byte, error) {
	if len(headers) == 0 {
		return []byte("{}"), nil
	}

	var cerr C.struct_FiError
	result := C.fi_extract_mcp_http_meta(
		(*C.uint8_t)(unsafe.Pointer(&headers[0])),
		C.uintptr_t(len(headers)),
		&cerr,
	)
	if result.data == nil {
		return nil, extractError(&cerr)
	}
	defer C.fi_buffer_free(result)

	return C.GoBytes(unsafe.Pointer(result.data), C.int(result.len)), nil
}
