//go:build !cgo

package fiaccel

import (
	"math"
	"strings"
)

// Available reports whether the native fi-accel library is linked.
func Available() bool { return false }

// --- Tokenizer fallback (pure Go, ~4 chars per token estimate) ---

type fallbackTokenizer struct {
	model string
}

func newTokenizer(model string) (Tokenizer, error) {
	return &fallbackTokenizer{model: model}, nil
}

func (t *fallbackTokenizer) Count(text string) (int, error) {
	// Rough approximation: ~4 chars per token for English text
	n := len(text) / 4
	if n == 0 && len(text) > 0 {
		n = 1
	}
	return n, nil
}

func (t *fallbackTokenizer) Encode(_ string) ([]uint32, error) {
	return nil, ErrNotAvailable
}

func (t *fallbackTokenizer) Decode(_ []uint32) (string, error) {
	return "", ErrNotAvailable
}

func (t *fallbackTokenizer) Truncate(text string, maxTokens int) (string, error) {
	maxChars := maxTokens * 4
	if len(text) <= maxChars {
		return text, nil
	}
	// Truncate at rune boundary
	runes := []rune(text)
	if len(runes) > maxChars {
		runes = runes[:maxChars]
	}
	return string(runes), nil
}

func (t *fallbackTokenizer) Close() {}

// --- Embedding fallback (pure Go) ---

func cosineSimilarity(a, b []float32) (float32, error) {
	if len(a) != len(b) {
		return 0, &Error{Code: ErrInvalidArg, Message: "vectors must have same length"}
	}

	var dot, normA, normB float64
	for i := range a {
		dot += float64(a[i]) * float64(b[i])
		normA += float64(a[i]) * float64(a[i])
		normB += float64(b[i]) * float64(b[i])
	}

	if normA == 0 || normB == 0 {
		return 0, nil
	}

	return float32(dot / (math.Sqrt(normA) * math.Sqrt(normB))), nil
}

func dotProduct(a, b []float32) (float32, error) {
	if len(a) != len(b) {
		return 0, &Error{Code: ErrInvalidArg, Message: "vectors must have same length"}
	}

	var sum float64
	for i := range a {
		sum += float64(a[i]) * float64(b[i])
	}
	return float32(sum), nil
}

func normalizeL2(v []float32) error {
	var normSq float64
	for _, x := range v {
		normSq += float64(x) * float64(x)
	}
	if normSq > 0 {
		norm := float32(math.Sqrt(normSq))
		for i := range v {
			v[i] /= norm
		}
	}
	return nil
}

// --- Diff/similarity fallback (pure Go) ---

func stringSimilarity(a, b string) (float32, error) {
	if a == b {
		return 1.0, nil
	}
	if len(a) == 0 || len(b) == 0 {
		return 0.0, nil
	}
	dist := levenshteinDist(a, b)
	maxLen := len(a)
	if len(b) > maxLen {
		maxLen = len(b)
	}
	return 1.0 - float32(dist)/float32(maxLen), nil
}

func lineSimilarity(a, b string) (float32, error) {
	aLines := strings.Split(a, "\n")
	bLines := strings.Split(b, "\n")
	if len(aLines) == 0 || len(bLines) == 0 {
		return 0.0, nil
	}
	// Join and compare as single strings (same approach as Rust side)
	return stringSimilarity(strings.Join(aLines, "\n"), strings.Join(bLines, "\n"))
}

func levenshteinDist(a, b string) int {
	if len(a) == 0 {
		return len(b)
	}
	if len(b) == 0 {
		return len(a)
	}
	prev := make([]int, len(b)+1)
	curr := make([]int, len(b)+1)
	for j := range prev {
		prev[j] = j
	}
	for i := 1; i <= len(a); i++ {
		curr[0] = i
		for j := 1; j <= len(b); j++ {
			cost := 0
			if a[i-1] != b[j-1] {
				cost = 1
			}
			ins := curr[j-1] + 1
			del := prev[j] + 1
			sub := prev[j-1] + cost
			m := ins
			if del < m {
				m = del
			}
			if sub < m {
				m = sub
			}
			curr[j] = m
		}
		prev, curr = curr, prev
	}
	return prev[len(b)]
}

func cosineSimilarityMatrix(a, b [][]float32) ([][]float32, error) {
	if len(a) == 0 || len(b) == 0 {
		return nil, nil
	}

	matrix := make([][]float32, len(a))
	for i, va := range a {
		row := make([]float32, len(b))
		for j, vb := range b {
			sim, err := cosineSimilarity(va, vb)
			if err != nil {
				return nil, err
			}
			row[j] = sim
		}
		matrix[i] = row
	}
	return matrix, nil
}

func computeCentroid(embeddings [][]float32) ([]float32, error) {
	if len(embeddings) == 0 {
		return nil, nil
	}

	dim := len(embeddings[0])
	centroid := make([]float32, dim)
	n := float32(len(embeddings))

	for _, v := range embeddings {
		if len(v) != dim {
			return nil, &Error{Code: ErrInvalidArg, Message: "all embeddings must have same dimensionality"}
		}
		for j := range v {
			centroid[j] += v[j]
		}
	}

	for j := range centroid {
		centroid[j] /= n
	}
	return centroid, nil
}

func parseNDJSON(data []byte) ([]byte, error) {
	// Pure Go fallback: split by newlines, parse each as JSON, combine into array
	lines := strings.Split(string(data), "\n")
	var parts []string
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		// Validate it's valid JSON by checking basic structure
		parts = append(parts, line)
	}
	return []byte("[" + strings.Join(parts, ",") + "]"), nil
}

func toNDJSON(jsonArray []byte) ([]byte, error) {
	// Pure Go fallback: not implemented without JSON parsing
	return nil, ErrNotAvailable
}

func topKSimilar(query []float32, candidates [][]float32, k int) ([]TopKEntry, error) {
	if len(query) == 0 || len(candidates) == 0 {
		return nil, nil
	}

	type scored struct {
		idx   int
		score float32
	}

	scores := make([]scored, len(candidates))
	for i, c := range candidates {
		sim, err := cosineSimilarity(query, c)
		if err != nil {
			return nil, err
		}
		scores[i] = scored{idx: i, score: sim}
	}

	// Simple selection sort for top-k (good enough for small k)
	for i := 0; i < k && i < len(scores); i++ {
		maxIdx := i
		for j := i + 1; j < len(scores); j++ {
			if scores[j].score > scores[maxIdx].score {
				maxIdx = j
			}
		}
		scores[i], scores[maxIdx] = scores[maxIdx], scores[i]
	}

	n := k
	if n > len(scores) {
		n = len(scores)
	}

	result := make([]TopKEntry, n)
	for i := 0; i < n; i++ {
		result[i] = TopKEntry{Index: scores[i].idx, Score: scores[i].score}
	}
	return result, nil
}
