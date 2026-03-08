//go:build !cgo

package fiaccel

func chunkCode(_, _ string, _ ChunkConfig) ([]byte, error) {
	return nil, ErrNotAvailable
}

func chunkMarkdown(_ string, _ ChunkConfig) ([]byte, error) {
	return nil, ErrNotAvailable
}

func chunkText(_ string, _ ChunkConfig) ([]byte, error) {
	return nil, ErrNotAvailable
}

func chunkAuto(_, _ string, _ ChunkConfig) ([]byte, error) {
	return nil, ErrNotAvailable
}

func detectContentType(_ string) (string, error) {
	return "", ErrNotAvailable
}
