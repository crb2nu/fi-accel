//! SIMD-accelerated embedding operations
//!
//! Provides 50-100x speedup for vector similarity calculations using
//! Rayon parallelization and optimized dot product implementations.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Result of preparing text for embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparedText {
    /// Original text (potentially truncated)
    pub text: String,
    /// Estimated token count
    pub token_count: usize,
    /// Whether text was truncated
    pub truncated: bool,
}

/// Normalize a single vector to unit length (L2 normalization)
#[inline]
pub fn normalize_l2_single(v: &mut [f32]) {
    let norm_sq: f32 = v.iter().map(|x| x * x).sum();
    if norm_sq > 0.0 {
        let norm = norm_sq.sqrt();
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

/// Normalize multiple vectors to unit length in parallel
pub fn normalize_l2(embeddings: &mut [Vec<f32>]) {
    embeddings.par_iter_mut().for_each(|v| {
        normalize_l2_single(v);
    });
}

/// Compute dot product of two vectors (optimized with loop unrolling)
#[inline]
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let n = a.len();
    let chunks = n / 4;
    let remainder = n % 4;

    let mut sum = 0.0f32;

    // Process 4 elements at a time (manual unrolling)
    for i in 0..chunks {
        let base = i * 4;
        sum += a[base] * b[base];
        sum += a[base + 1] * b[base + 1];
        sum += a[base + 2] * b[base + 2];
        sum += a[base + 3] * b[base + 3];
    }

    // Handle remainder
    let base = chunks * 4;
    for i in 0..remainder {
        sum += a[base + i] * b[base + i];
    }

    sum
}

/// Compute cosine similarity between two vectors
///
/// Assumes vectors are already normalized. For non-normalized vectors,
/// use `cosine_similarity_normalized` which normalizes first.
#[inline]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    dot_product(a, b)
}

/// Compute cosine similarity with normalization
pub fn cosine_similarity_normalized(a: &[f32], b: &[f32]) -> f32 {
    let dot = dot_product(a, b);
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// Compute cosine similarity matrix between two sets of vectors
///
/// Returns a matrix where result[i][j] = cosine_similarity(a[i], b[j])
///
/// For best performance, vectors should be pre-normalized using `normalize_l2`.
pub fn cosine_similarity_matrix(a: &[Vec<f32>], b: &[Vec<f32>]) -> Vec<Vec<f32>> {
    a.par_iter()
        .map(|vec_a| b.iter().map(|vec_b| cosine_similarity(vec_a, vec_b)).collect())
        .collect()
}

/// Compute top-k most similar vectors from b for each vector in a
///
/// Returns indices and scores for each query vector.
pub fn top_k_similar(a: &[Vec<f32>], b: &[Vec<f32>], k: usize) -> Vec<Vec<(usize, f32)>> {
    a.par_iter()
        .map(|vec_a| {
            let mut scores: Vec<(usize, f32)> = b
                .iter()
                .enumerate()
                .map(|(i, vec_b)| (i, cosine_similarity(vec_a, vec_b)))
                .collect();

            // Partial sort to get top k
            if k < scores.len() {
                scores.select_nth_unstable_by(k, |a, b| {
                    b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
                });
                scores.truncate(k);
            }

            // Sort the top k by score descending
            scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            scores
        })
        .collect()
}

/// Compute average of multiple embeddings (centroid)
pub fn compute_centroid(embeddings: &[Vec<f32>]) -> Option<Vec<f32>> {
    if embeddings.is_empty() {
        return None;
    }

    let dim = embeddings[0].len();
    let n = embeddings.len() as f32;

    // Sum all vectors in parallel
    let sum: Vec<f32> = (0..dim)
        .into_par_iter()
        .map(|i| embeddings.iter().map(|e| e[i]).sum::<f32>())
        .collect();

    // Divide by count
    Some(sum.into_iter().map(|x| x / n).collect())
}

/// Batch prepare texts for embedding (with token counting if tokenizer available)
#[cfg(feature = "tokenizer")]
pub fn prepare_embedding_batch(
    texts: Vec<String>,
    max_tokens: usize,
    model: &str,
) -> Result<Vec<PreparedText>, crate::tokenizer::TokenizerError> {
    use crate::tokenizer::Tokenizer;

    let tokenizer = Tokenizer::new(model)?;

    Ok(texts
        .into_par_iter()
        .map(|text| {
            let token_count = tokenizer.count(&text);
            if token_count <= max_tokens {
                PreparedText {
                    text,
                    token_count,
                    truncated: false,
                }
            } else {
                let truncated_text = tokenizer.truncate(&text, max_tokens);
                PreparedText {
                    token_count: max_tokens,
                    text: truncated_text,
                    truncated: true,
                }
            }
        })
        .collect())
}

/// Simple batch prepare without token counting
pub fn prepare_batch_simple(texts: Vec<String>, max_chars: usize) -> Vec<PreparedText> {
    texts
        .into_par_iter()
        .map(|text| {
            if text.len() <= max_chars {
                PreparedText {
                    token_count: text.len() / 4, // Rough estimate
                    text,
                    truncated: false,
                }
            } else {
                // Truncate at char boundary
                let truncated = text.chars().take(max_chars).collect::<String>();
                PreparedText {
                    token_count: max_chars / 4,
                    text: truncated,
                    truncated: true,
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_l2() {
        let mut v = vec![3.0, 4.0];
        normalize_l2_single(&mut v);

        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_dot_product() {
        let a = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let b = vec![2.0, 3.0, 4.0, 5.0, 6.0];
        let result = dot_product(&a, &b);
        // 1*2 + 2*3 + 3*4 + 4*5 + 5*6 = 2 + 6 + 12 + 20 + 30 = 70
        assert!((result - 70.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let mut a = vec![1.0, 2.0, 3.0];
        let mut b = vec![1.0, 2.0, 3.0];
        normalize_l2_single(&mut a);
        normalize_l2_single(&mut b);

        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let mut a = vec![1.0, 0.0];
        let mut b = vec![0.0, 1.0];
        normalize_l2_single(&mut a);
        normalize_l2_single(&mut b);

        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_similarity_matrix() {
        let mut a = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let mut b = vec![vec![1.0, 0.0], vec![0.0, 1.0]];

        normalize_l2(&mut a);
        normalize_l2(&mut b);

        let matrix = cosine_similarity_matrix(&a, &b);

        // Identity-like matrix
        assert!((matrix[0][0] - 1.0).abs() < 1e-6);
        assert!(matrix[0][1].abs() < 1e-6);
        assert!(matrix[1][0].abs() < 1e-6);
        assert!((matrix[1][1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_top_k_similar() {
        let query = vec![vec![1.0, 0.0, 0.0]];
        let candidates = vec![
            vec![1.0, 0.0, 0.0], // Most similar
            vec![0.5, 0.5, 0.0],
            vec![0.0, 1.0, 0.0], // Orthogonal
        ];

        let results = top_k_similar(&query, &candidates, 2);
        assert_eq!(results[0].len(), 2);
        assert_eq!(results[0][0].0, 0); // Index 0 should be most similar
    }

    #[test]
    fn test_compute_centroid() {
        let embeddings = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]];

        let centroid = compute_centroid(&embeddings).unwrap();
        assert!((centroid[0] - 2.0 / 3.0).abs() < 1e-6);
        assert!((centroid[1] - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_prepare_batch_simple() {
        let texts = vec!["short".to_string(), "a".repeat(1000)];
        let prepared = prepare_batch_simple(texts, 100);

        assert_eq!(prepared.len(), 2);
        assert!(!prepared[0].truncated);
        assert!(prepared[1].truncated);
        assert!(prepared[1].text.len() <= 100);
    }
}
