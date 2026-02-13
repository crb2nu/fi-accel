//go:build cgo

package fiaccel

/*
#include "fi_accel.h"
#include <stdlib.h>
*/
import "C"

import "unsafe"

// StringSimilarity computes character-level similarity between two strings
// using Myers diff (via Rust). Returns a value in [0.0, 1.0].
func stringSimilarity(a, b string) (float32, error) {
	cA := C.CString(a)
	defer C.free(unsafe.Pointer(cA))
	cB := C.CString(b)
	defer C.free(unsafe.Pointer(cB))

	var cerr C.struct_FiError
	result := C.fi_string_similarity(cA, cB, &cerr)
	if err := extractError(&cerr); err != nil {
		return 0, err
	}
	return float32(result), nil
}

// LineSimilarity computes line-level similarity between two multiline strings.
// Each string is treated as a sequence of lines. Returns a value in [0.0, 1.0].
func lineSimilarity(a, b string) (float32, error) {
	cA := C.CString(a)
	defer C.free(unsafe.Pointer(cA))
	cB := C.CString(b)
	defer C.free(unsafe.Pointer(cB))

	var cerr C.struct_FiError
	result := C.fi_line_similarity(cA, cB, &cerr)
	if err := extractError(&cerr); err != nil {
		return 0, err
	}
	return float32(result), nil
}
