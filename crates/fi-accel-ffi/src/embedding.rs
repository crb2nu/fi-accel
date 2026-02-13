//! FFI bindings for the embedding module

use crate::types::*;

/// Compute cosine similarity between two f32 vectors
///
/// Both vectors must have the same length `dim`.
/// Vectors should be pre-normalized for best results.
#[no_mangle]
pub unsafe extern "C" fn fi_cosine_similarity(
    a: *const f32,
    b: *const f32,
    dim: usize,
    err: *mut FiError,
) -> f32 {
    if a.is_null() || b.is_null() {
        if let Some(e) = err.as_mut() {
            e.set(FiErrorCode::InvalidArg, "vector pointer is null");
        }
        return 0.0;
    }

    let a_slice = std::slice::from_raw_parts(a, dim);
    let b_slice = std::slice::from_raw_parts(b, dim);

    fi_accel_core::embedding::cosine_similarity(a_slice, b_slice)
}

/// Compute dot product between two f32 vectors
#[no_mangle]
pub unsafe extern "C" fn fi_dot_product(
    a: *const f32,
    b: *const f32,
    dim: usize,
    err: *mut FiError,
) -> f32 {
    if a.is_null() || b.is_null() {
        if let Some(e) = err.as_mut() {
            e.set(FiErrorCode::InvalidArg, "vector pointer is null");
        }
        return 0.0;
    }

    let a_slice = std::slice::from_raw_parts(a, dim);
    let b_slice = std::slice::from_raw_parts(b, dim);

    fi_accel_core::embedding::dot_product(a_slice, b_slice)
}

/// Normalize a vector in-place to unit length (L2 normalization)
#[no_mangle]
pub unsafe extern "C" fn fi_normalize_l2(
    v: *mut f32,
    dim: usize,
    err: *mut FiError,
) {
    if v.is_null() {
        if let Some(e) = err.as_mut() {
            e.set(FiErrorCode::InvalidArg, "vector pointer is null");
        }
        return;
    }

    let v_slice = std::slice::from_raw_parts_mut(v, dim);
    fi_accel_core::embedding::normalize_l2_single(v_slice);
}

/// Find top-k most similar vectors to a query
///
/// - `query`: query vector of length `dim`
/// - `candidates`: flat array of `num_candidates * dim` floats
/// - `num_candidates`: number of candidate vectors
/// - `dim`: vector dimensionality
/// - `k`: number of results to return
///
/// Returns a FiTopKResult that must be freed with `fi_top_k_result_free`.
#[no_mangle]
pub unsafe extern "C" fn fi_top_k_similar(
    query: *const f32,
    candidates: *const f32,
    num_candidates: usize,
    dim: usize,
    k: usize,
    err: *mut FiError,
) -> FiTopKResult {
    if query.is_null() || candidates.is_null() {
        if let Some(e) = err.as_mut() {
            e.set(FiErrorCode::InvalidArg, "pointer is null");
        }
        return FiTopKResult::null();
    }

    let query_slice = std::slice::from_raw_parts(query, dim);

    // Convert flat array to Vec<Vec<f32>>
    let candidate_vecs: Vec<Vec<f32>> = (0..num_candidates)
        .map(|i| {
            let start = i * dim;
            std::slice::from_raw_parts(candidates.add(start), dim).to_vec()
        })
        .collect();

    let query_vec = vec![query_slice.to_vec()];
    let results = fi_accel_core::embedding::top_k_similar(&query_vec, &candidate_vecs, k);

    if results.is_empty() {
        return FiTopKResult::null();
    }

    FiTopKResult::from_vec(results.into_iter().next().unwrap_or_default())
}

/// Compute cosine similarity matrix between two sets of vectors
///
/// - `a`: flat array of `num_a * dim` floats
/// - `b`: flat array of `num_b * dim` floats
/// - `num_a`, `num_b`: number of vectors in each set
/// - `dim`: vector dimensionality
///
/// Returns a flat FiFloatArray of `num_a * num_b` floats (row-major).
/// Must be freed with `fi_float_array_free`.
#[no_mangle]
pub unsafe extern "C" fn fi_cosine_similarity_matrix(
    a: *const f32,
    b: *const f32,
    num_a: usize,
    num_b: usize,
    dim: usize,
    err: *mut FiError,
) -> FiFloatArray {
    if a.is_null() || b.is_null() {
        if let Some(e) = err.as_mut() {
            e.set(FiErrorCode::InvalidArg, "pointer is null");
        }
        return FiFloatArray::null();
    }

    let a_vecs: Vec<Vec<f32>> = (0..num_a)
        .map(|i| std::slice::from_raw_parts(a.add(i * dim), dim).to_vec())
        .collect();
    let b_vecs: Vec<Vec<f32>> = (0..num_b)
        .map(|i| std::slice::from_raw_parts(b.add(i * dim), dim).to_vec())
        .collect();

    let matrix = fi_accel_core::embedding::cosine_similarity_matrix(&a_vecs, &b_vecs);

    // Flatten row-major
    let flat: Vec<f32> = matrix.into_iter().flat_map(|row| row).collect();
    FiFloatArray::from_vec(flat)
}

/// Compute the centroid (average) of a set of embeddings
///
/// - `embeddings`: flat array of `num_embeddings * dim` floats
/// - `num_embeddings`: number of vectors
/// - `dim`: vector dimensionality
///
/// Returns a FiFloatArray of length `dim`.
/// Must be freed with `fi_float_array_free`.
#[no_mangle]
pub unsafe extern "C" fn fi_compute_centroid(
    embeddings: *const f32,
    num_embeddings: usize,
    dim: usize,
    err: *mut FiError,
) -> FiFloatArray {
    if embeddings.is_null() {
        if let Some(e) = err.as_mut() {
            e.set(FiErrorCode::InvalidArg, "pointer is null");
        }
        return FiFloatArray::null();
    }

    if num_embeddings == 0 {
        return FiFloatArray::null();
    }

    let vecs: Vec<Vec<f32>> = (0..num_embeddings)
        .map(|i| std::slice::from_raw_parts(embeddings.add(i * dim), dim).to_vec())
        .collect();

    match fi_accel_core::embedding::compute_centroid(&vecs) {
        Some(centroid) => FiFloatArray::from_vec(centroid),
        None => FiFloatArray::null(),
    }
}
