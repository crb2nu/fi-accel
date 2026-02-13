package fiaccel

// StringSimilarity computes character-level similarity between two strings
// using Myers diff. Returns a value in [0.0, 1.0] where 1.0 means identical.
//
// When CGo is enabled, uses the native Rust implementation.
// Otherwise falls back to a pure Go Levenshtein distance.
func StringSimilarity(a, b string) (float32, error) {
	return stringSimilarity(a, b)
}

// LineSimilarity computes line-level similarity between two multiline strings.
// Each string is treated as a sequence of lines (split by newline).
// Returns a value in [0.0, 1.0] where 1.0 means identical.
//
// When CGo is enabled, uses the native Rust implementation.
// Otherwise falls back to a pure Go Levenshtein distance.
func LineSimilarity(a, b string) (float32, error) {
	return lineSimilarity(a, b)
}
