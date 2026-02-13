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

// CosineSimilarityMatrix computes cosine similarity between all pairs of vectors
// from sets a and b. Returns a matrix where result[i][j] = cosine_similarity(a[i], b[j]).
//
// When CGo is enabled, uses the native Rust implementation with Rayon parallelization.
// Otherwise falls back to a pure Go implementation.
func CosineSimilarityMatrix(a, b [][]float32) ([][]float32, error) {
	return cosineSimilarityMatrix(a, b)
}

// ComputeCentroid computes the average (centroid) of a set of embeddings.
// Returns a single vector representing the mean of all input vectors.
//
// When CGo is enabled, uses the native Rust implementation with parallel summation.
// Otherwise falls back to a pure Go implementation.
func ComputeCentroid(embeddings [][]float32) ([]float32, error) {
	return computeCentroid(embeddings)
}
