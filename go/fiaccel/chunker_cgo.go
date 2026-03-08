//go:build cgo

package fiaccel

/*
#include "fi_accel.h"
#include <stdlib.h>
*/
import "C"

import "unsafe"

func chunkCode(content, language string, config ChunkConfig) ([]byte, error) {
	cContent := C.CString(content)
	defer C.free(unsafe.Pointer(cContent))
	cLanguage := C.CString(language)
	defer C.free(unsafe.Pointer(cLanguage))
	cFilePath, freeFilePath := optionalCString(config.FilePath)
	defer freeFilePath()
	cModel, freeModel := optionalCString(config.Model)
	defer freeModel()

	var cerr C.struct_FiError
	result := C.fi_chunk_code(
		cContent,
		cLanguage,
		C.uintptr_t(config.MaxTokens),
		C.uintptr_t(config.OverlapTokens),
		cFilePath,
		cModel,
		&cerr,
	)
	return bufferToBytes(result, &cerr)
}

func chunkMarkdown(content string, config ChunkConfig) ([]byte, error) {
	cContent := C.CString(content)
	defer C.free(unsafe.Pointer(cContent))
	cFilePath, freeFilePath := optionalCString(config.FilePath)
	defer freeFilePath()
	cModel, freeModel := optionalCString(config.Model)
	defer freeModel()

	var cerr C.struct_FiError
	result := C.fi_chunk_markdown(
		cContent,
		C.uintptr_t(config.MaxTokens),
		C.uintptr_t(config.OverlapTokens),
		cFilePath,
		cModel,
		&cerr,
	)
	return bufferToBytes(result, &cerr)
}

func chunkText(content string, config ChunkConfig) ([]byte, error) {
	cContent := C.CString(content)
	defer C.free(unsafe.Pointer(cContent))
	cFilePath, freeFilePath := optionalCString(config.FilePath)
	defer freeFilePath()
	cModel, freeModel := optionalCString(config.Model)
	defer freeModel()

	var cerr C.struct_FiError
	result := C.fi_chunk_text(
		cContent,
		C.uintptr_t(config.MaxTokens),
		C.uintptr_t(config.OverlapTokens),
		cFilePath,
		cModel,
		&cerr,
	)
	return bufferToBytes(result, &cerr)
}

func chunkAuto(content, filePath string, config ChunkConfig) ([]byte, error) {
	cContent := C.CString(content)
	defer C.free(unsafe.Pointer(cContent))
	cFilePath := C.CString(filePath)
	defer C.free(unsafe.Pointer(cFilePath))
	cModel, freeModel := optionalCString(config.Model)
	defer freeModel()

	var cerr C.struct_FiError
	result := C.fi_chunk_auto(
		cContent,
		cFilePath,
		C.uintptr_t(config.MaxTokens),
		C.uintptr_t(config.OverlapTokens),
		cModel,
		&cerr,
	)
	return bufferToBytes(result, &cerr)
}

func detectContentType(filePath string) (string, error) {
	cFilePath := C.CString(filePath)
	defer C.free(unsafe.Pointer(cFilePath))

	var cerr C.struct_FiError
	result := C.fi_detect_content_type(cFilePath, &cerr)
	if result.data == nil {
		return "", extractError(&cerr)
	}
	defer C.fi_buffer_free(result)
	return C.GoStringN((*C.char)(unsafe.Pointer(result.data)), C.int(result.len)), nil
}

func optionalCString(value string) (*C.char, func()) {
	if value == "" {
		return nil, func() {}
	}
	ptr := C.CString(value)
	return ptr, func() { C.free(unsafe.Pointer(ptr)) }
}

func bufferToBytes(result C.struct_FiBuffer, cerr *C.struct_FiError) ([]byte, error) {
	if result.data == nil {
		return nil, extractError(cerr)
	}
	defer C.fi_buffer_free(result)
	return C.GoBytes(unsafe.Pointer(result.data), C.int(result.len)), nil
}
