package fiaccel

// ParseNDJSON parses newline-delimited JSON data and returns a JSON array string.
//
// When CGo is enabled, uses the native Rust implementation (with optional SIMD acceleration).
// Otherwise falls back to a pure Go implementation.
func ParseNDJSON(data []byte) ([]byte, error) {
	return parseNDJSON(data)
}

// ToNDJSON converts a JSON array to newline-delimited JSON format.
//
// When CGo is enabled, uses the native Rust implementation.
// Otherwise returns ErrNotAvailable.
func ToNDJSON(jsonArray []byte) ([]byte, error) {
	return toNDJSON(jsonArray)
}
