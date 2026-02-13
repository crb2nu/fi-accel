package fiaccel

// Tokenizer provides BPE token counting, encoding, decoding, and truncation.
type Tokenizer interface {
	// Count returns the exact number of BPE tokens in text.
	Count(text string) (int, error)

	// Encode converts text to token IDs.
	Encode(text string) ([]uint32, error)

	// Decode converts token IDs back to text.
	Decode(tokens []uint32) (string, error)

	// Truncate shortens text to fit within maxTokens.
	Truncate(text string, maxTokens int) (string, error)

	// Close releases the underlying tokenizer resources.
	Close()
}

// NewTokenizer creates a new Tokenizer for the given model name.
//
// When CGo is enabled and the fi-accel static library is linked, this
// returns a native Rust-backed tokenizer. Otherwise it returns a
// fallback that estimates tokens at ~4 chars per token.
func NewTokenizer(model string) (Tokenizer, error) {
	return newTokenizer(model)
}
