package fiaccel

import (
	"math"
	"testing"
)

func TestAvailable(t *testing.T) {
	// Just verify it doesn't panic
	_ = Available()
}

func TestNewTokenizer(t *testing.T) {
	tok, err := NewTokenizer("gpt-4")
	if err != nil {
		t.Fatalf("NewTokenizer: %v", err)
	}
	defer tok.Close()

	n, err := tok.Count("Hello, world!")
	if err != nil {
		t.Fatalf("Count: %v", err)
	}
	if n == 0 {
		t.Error("Count returned 0 for non-empty string")
	}
	t.Logf("Token count for 'Hello, world!': %d (available=%v)", n, Available())
}

func TestTokenizerTruncate(t *testing.T) {
	tok, err := NewTokenizer("gpt-4")
	if err != nil {
		t.Fatalf("NewTokenizer: %v", err)
	}
	defer tok.Close()

	text := "This is a longer text that should be truncated to fit within the token limit for testing purposes."
	truncated, err := tok.Truncate(text, 5)
	if err != nil {
		t.Fatalf("Truncate: %v", err)
	}
	if len(truncated) >= len(text) {
		t.Error("Truncate did not shorten text")
	}
	t.Logf("Truncated to 5 tokens: %q", truncated)
}

func TestCosineSimilarity(t *testing.T) {
	a := []float32{1, 0, 0}
	b := []float32{1, 0, 0}

	// Normalize
	_ = NormalizeL2(a)
	_ = NormalizeL2(b)

	sim, err := CosineSimilarity(a, b)
	if err != nil {
		t.Fatalf("CosineSimilarity: %v", err)
	}
	if math.Abs(float64(sim-1.0)) > 0.001 {
		t.Errorf("Expected similarity ~1.0 for identical vectors, got %f", sim)
	}
}

func TestCosineSimilarityOrthogonal(t *testing.T) {
	a := []float32{1, 0}
	b := []float32{0, 1}

	_ = NormalizeL2(a)
	_ = NormalizeL2(b)

	sim, err := CosineSimilarity(a, b)
	if err != nil {
		t.Fatalf("CosineSimilarity: %v", err)
	}
	if math.Abs(float64(sim)) > 0.001 {
		t.Errorf("Expected similarity ~0 for orthogonal vectors, got %f", sim)
	}
}

func TestDotProduct(t *testing.T) {
	a := []float32{1, 2, 3, 4, 5}
	b := []float32{2, 3, 4, 5, 6}

	dot, err := DotProduct(a, b)
	if err != nil {
		t.Fatalf("DotProduct: %v", err)
	}
	// 1*2 + 2*3 + 3*4 + 4*5 + 5*6 = 70
	if math.Abs(float64(dot-70.0)) > 0.001 {
		t.Errorf("Expected 70.0, got %f", dot)
	}
}

func TestNormalizeL2(t *testing.T) {
	v := []float32{3, 4}
	err := NormalizeL2(v)
	if err != nil {
		t.Fatalf("NormalizeL2: %v", err)
	}

	norm := float64(v[0]*v[0]) + float64(v[1]*v[1])
	if math.Abs(norm-1.0) > 0.001 {
		t.Errorf("Expected unit norm, got %f", math.Sqrt(norm))
	}
}

func TestTopKSimilar(t *testing.T) {
	query := []float32{1, 0, 0}
	candidates := [][]float32{
		{1, 0, 0},     // Most similar
		{0.5, 0.5, 0}, // Somewhat similar
		{0, 1, 0},     // Orthogonal
	}

	entries, err := TopKSimilar(query, candidates, 2)
	if err != nil {
		t.Fatalf("TopKSimilar: %v", err)
	}
	if len(entries) != 2 {
		t.Fatalf("Expected 2 entries, got %d", len(entries))
	}
	if entries[0].Index != 0 {
		t.Errorf("Expected index 0 as most similar, got %d", entries[0].Index)
	}
}

func TestStringSimilarityIdentical(t *testing.T) {
	sim, err := StringSimilarity("hello world", "hello world")
	if err != nil {
		t.Fatalf("StringSimilarity: %v", err)
	}
	if math.Abs(float64(sim-1.0)) > 0.001 {
		t.Errorf("Expected ~1.0 for identical strings, got %f", sim)
	}
}

func TestStringSimilaritySimilar(t *testing.T) {
	sim, err := StringSimilarity("hello world", "hello word")
	if err != nil {
		t.Fatalf("StringSimilarity: %v", err)
	}
	if sim < 0.8 {
		t.Errorf("Expected > 0.8 for similar strings, got %f", sim)
	}
	t.Logf("Similarity 'hello world' vs 'hello word': %f (available=%v)", sim, Available())
}

func TestStringSimilarityDifferent(t *testing.T) {
	sim, err := StringSimilarity("hello", "goodbye")
	if err != nil {
		t.Fatalf("StringSimilarity: %v", err)
	}
	if sim > 0.5 {
		t.Errorf("Expected < 0.5 for different strings, got %f", sim)
	}
}

func TestStringSimilarityEmpty(t *testing.T) {
	sim, err := StringSimilarity("", "hello")
	if err != nil {
		t.Fatalf("StringSimilarity: %v", err)
	}
	if sim != 0.0 {
		t.Errorf("Expected 0.0 for empty vs non-empty, got %f", sim)
	}
}

func TestLineSimilarity(t *testing.T) {
	a := "line one\nline two\nline three"
	b := "line one\nline two\nline three"
	sim, err := LineSimilarity(a, b)
	if err != nil {
		t.Fatalf("LineSimilarity: %v", err)
	}
	if math.Abs(float64(sim-1.0)) > 0.001 {
		t.Errorf("Expected ~1.0 for identical lines, got %f", sim)
	}
}

func TestLineSimilarityPartial(t *testing.T) {
	a := "line one\nline two\nline three"
	b := "line one\nline TWO\nline three"
	sim, err := LineSimilarity(a, b)
	if err != nil {
		t.Fatalf("LineSimilarity: %v", err)
	}
	if sim < 0.8 || sim >= 1.0 {
		t.Errorf("Expected high but <1.0 similarity, got %f", sim)
	}
}

func TestMismatchedVectorLength(t *testing.T) {
	a := []float32{1, 2}
	b := []float32{1, 2, 3}

	_, err := CosineSimilarity(a, b)
	if err == nil {
		t.Error("Expected error for mismatched vector lengths")
	}
}
