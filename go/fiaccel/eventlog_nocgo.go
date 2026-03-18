//go:build !cgo

package fiaccel

func projectEventlog(_ []byte, _ []byte) ([]byte, error) {
	return nil, ErrNotAvailable
}

func summarizeEventlog(_ []byte, _ []byte) ([]byte, error) {
	return nil, ErrNotAvailable
}
