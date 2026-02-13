package fiaccel

// CosineSimilarity computes the cosine similarity between two vectors.
// Vectors should be pre-normalized for correct results.
func CosineSimilarity(a, b []float32) (float32, error) {
	return cosineSimilarity(a, b)
}

// DotProduct computes the dot product of two vectors.
func DotProduct(a, b []float32) (float32, error) {
	return dotProduct(a, b)
}

// NormalizeL2 normalizes a vector in-place to unit length.
func NormalizeL2(v []float32) error {
	return normalizeL2(v)
}

// TopKEntry represents a single top-k similarity result.
type TopKEntry struct {
	Index int
	Score float32
}

// TopKSimilar finds the k most similar vectors from candidates to the query.
// All vectors must have the same dimensionality.
func TopKSimilar(query []float32, candidates [][]float32, k int) ([]TopKEntry, error) {
	return topKSimilar(query, candidates, k)
}
