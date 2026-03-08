//go:build !cgo

package fiaccel

func parseJSONRPCLines(_ []byte) ([]byte, error) {
	return nil, ErrNotAvailable
}

func serializeJSONRPCLines(_ []byte) ([]byte, error) {
	return nil, ErrNotAvailable
}

func parseSSEEvents(_ []byte) ([]byte, error) {
	return nil, ErrNotAvailable
}

func extractMCPHTTPMeta(_ []byte) ([]byte, error) {
	return nil, ErrNotAvailable
}
