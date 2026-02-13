//go:build cgo

package fiaccel

/*
#cgo CFLAGS: -I${SRCDIR}/../include
#cgo darwin,arm64 LDFLAGS: -L${SRCDIR}/../lib/darwin_arm64 -lfi_accel_ffi -lm -ldl -framework Security -framework CoreFoundation
#cgo linux,amd64 LDFLAGS: -L${SRCDIR}/../lib/linux_amd64 -lfi_accel_ffi -lm -ldl -lpthread

#include "fi_accel.h"
#include <stdlib.h>
#include <string.h>
*/
import "C"

import (
	"unsafe"
)

// Available reports whether the native fi-accel library is linked.
func Available() bool { return true }

func extractError(cerr *C.struct_FiError) error {
	if cerr == nil || cerr.code == C.FI_ERROR_CODE_OK {
		return nil
	}
	msg := C.GoStringN((*C.char)(unsafe.Pointer(&cerr.message[0])), C.int(cerr.message_len))
	return &Error{Code: ErrorCode(cerr.code), Message: msg}
}

// --- Tokenizer (CGo) ---

type cgoTokenizer struct {
	ptr *C.struct_FiTokenizer
}

func newTokenizer(model string) (Tokenizer, error) {
	cModel := C.CString(model)
	defer C.free(unsafe.Pointer(cModel))

	var cerr C.struct_FiError
	ptr := C.fi_tokenizer_new(cModel, &cerr)
	if ptr == nil {
		return nil, extractError(&cerr)
	}
	return &cgoTokenizer{ptr: ptr}, nil
}

func (t *cgoTokenizer) Count(text string) (int, error) {
	cText := C.CString(text)
	defer C.free(unsafe.Pointer(cText))

	var cerr C.struct_FiError
	n := C.fi_tokenizer_count(t.ptr, cText, C.uintptr_t(len(text)), &cerr)
	if err := extractError(&cerr); err != nil {
		return 0, err
	}
	return int(n), nil
}

func (t *cgoTokenizer) Encode(text string) ([]uint32, error) {
	cText := C.CString(text)
	defer C.free(unsafe.Pointer(cText))

	var cerr C.struct_FiError
	arr := C.fi_tokenizer_encode(t.ptr, cText, &cerr)
	if arr.data == nil {
		return nil, extractError(&cerr)
	}
	defer C.fi_token_array_free(arr)

	result := make([]uint32, arr.len)
	if arr.len > 0 {
		src := unsafe.Slice((*uint32)(unsafe.Pointer(arr.data)), arr.len)
		copy(result, src)
	}
	return result, nil
}

func (t *cgoTokenizer) Decode(tokens []uint32) (string, error) {
	if len(tokens) == 0 {
		return "", nil
	}

	var cerr C.struct_FiError
	buf := C.fi_tokenizer_decode(
		t.ptr,
		(*C.uint32_t)(unsafe.Pointer(&tokens[0])),
		C.uintptr_t(len(tokens)),
		&cerr,
	)
	if buf.data == nil {
		return "", extractError(&cerr)
	}
	defer C.fi_buffer_free(buf)

	return C.GoStringN((*C.char)(unsafe.Pointer(buf.data)), C.int(buf.len)), nil
}

func (t *cgoTokenizer) Truncate(text string, maxTokens int) (string, error) {
	cText := C.CString(text)
	defer C.free(unsafe.Pointer(cText))

	var cerr C.struct_FiError
	buf := C.fi_tokenizer_truncate(t.ptr, cText, C.uintptr_t(maxTokens), &cerr)
	if buf.data == nil {
		return "", extractError(&cerr)
	}
	defer C.fi_buffer_free(buf)

	return C.GoStringN((*C.char)(unsafe.Pointer(buf.data)), C.int(buf.len)), nil
}

func (t *cgoTokenizer) Close() {
	if t.ptr != nil {
		C.fi_tokenizer_free(t.ptr)
		t.ptr = nil
	}
}

// --- Embedding (CGo) ---

func cosineSimilarity(a, b []float32) (float32, error) {
	if len(a) != len(b) {
		return 0, &Error{Code: ErrInvalidArg, Message: "vectors must have same length"}
	}
	if len(a) == 0 {
		return 0, nil
	}

	var cerr C.struct_FiError
	result := C.fi_cosine_similarity(
		(*C.float)(unsafe.Pointer(&a[0])),
		(*C.float)(unsafe.Pointer(&b[0])),
		C.uintptr_t(len(a)),
		&cerr,
	)
	if err := extractError(&cerr); err != nil {
		return 0, err
	}
	return float32(result), nil
}

func dotProduct(a, b []float32) (float32, error) {
	if len(a) != len(b) {
		return 0, &Error{Code: ErrInvalidArg, Message: "vectors must have same length"}
	}
	if len(a) == 0 {
		return 0, nil
	}

	var cerr C.struct_FiError
	result := C.fi_dot_product(
		(*C.float)(unsafe.Pointer(&a[0])),
		(*C.float)(unsafe.Pointer(&b[0])),
		C.uintptr_t(len(a)),
		&cerr,
	)
	if err := extractError(&cerr); err != nil {
		return 0, err
	}
	return float32(result), nil
}

func normalizeL2(v []float32) error {
	if len(v) == 0 {
		return nil
	}

	var cerr C.struct_FiError
	C.fi_normalize_l2(
		(*C.float)(unsafe.Pointer(&v[0])),
		C.uintptr_t(len(v)),
		&cerr,
	)
	return extractError(&cerr)
}

func topKSimilar(query []float32, candidates [][]float32, k int) ([]TopKEntry, error) {
	if len(query) == 0 || len(candidates) == 0 {
		return nil, nil
	}

	dim := len(query)

	// Flatten candidates into a contiguous array
	flat := make([]float32, 0, len(candidates)*dim)
	for _, c := range candidates {
		if len(c) != dim {
			return nil, &Error{Code: ErrInvalidArg, Message: "all candidates must match query dimensionality"}
		}
		flat = append(flat, c...)
	}

	var cerr C.struct_FiError
	result := C.fi_top_k_similar(
		(*C.float)(unsafe.Pointer(&query[0])),
		(*C.float)(unsafe.Pointer(&flat[0])),
		C.uintptr_t(len(candidates)),
		C.uintptr_t(dim),
		C.uintptr_t(k),
		&cerr,
	)
	if result.entries == nil {
		return nil, extractError(&cerr)
	}
	defer C.fi_top_k_result_free(result)

	entries := make([]TopKEntry, result.len)
	src := unsafe.Slice((*C.struct_FiTopKEntry)(unsafe.Pointer(result.entries)), result.len)
	for i, e := range src {
		entries[i] = TopKEntry{
			Index: int(e.index),
			Score: float32(e.score),
		}
	}
	return entries, nil
}
