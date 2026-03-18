//go:build cgo

package fiaccel

/*
#include "fi_accel.h"
*/
import "C"

import "unsafe"

func projectEventlog(data []byte, configJSON []byte) ([]byte, error) {
	if len(data) == 0 {
		return []byte("[]"), nil
	}

	var configPtr *C.uint8_t
	if len(configJSON) > 0 {
		configPtr = (*C.uint8_t)(unsafe.Pointer(&configJSON[0]))
	}

	var cerr C.struct_FiError
	result := C.fi_project_eventlog(
		(*C.uint8_t)(unsafe.Pointer(&data[0])),
		C.uintptr_t(len(data)),
		configPtr,
		C.uintptr_t(len(configJSON)),
		&cerr,
	)
	if result.data == nil {
		return nil, extractError(&cerr)
	}
	defer C.fi_buffer_free(result)

	return C.GoBytes(unsafe.Pointer(result.data), C.int(result.len)), nil
}

func summarizeEventlog(data []byte, configJSON []byte) ([]byte, error) {
	if len(data) == 0 {
		return []byte("{}"), nil
	}

	var configPtr *C.uint8_t
	if len(configJSON) > 0 {
		configPtr = (*C.uint8_t)(unsafe.Pointer(&configJSON[0]))
	}

	var cerr C.struct_FiError
	result := C.fi_summarize_eventlog(
		(*C.uint8_t)(unsafe.Pointer(&data[0])),
		C.uintptr_t(len(data)),
		configPtr,
		C.uintptr_t(len(configJSON)),
		&cerr,
	)
	if result.data == nil {
		return nil, extractError(&cerr)
	}
	defer C.fi_buffer_free(result)

	return C.GoBytes(unsafe.Pointer(result.data), C.int(result.len)), nil
}
