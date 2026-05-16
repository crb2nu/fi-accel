//! Ranking and fusion helpers for retrieval pipelines.

use std::collections::HashMap;
use std::fmt;

/// Ranking method used by [`hybrid_rank`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridRankMethod {
    /// Weighted lexical rank and cosine score:
    /// `alpha * (1 / (1 + lex_rank)) + (1 - alpha) * cosine`.
    WeightedSum,
    /// Reciprocal rank fusion over lexical ranks and cosine-derived ranks.
    Rrf { k: usize },
}

impl HybridRankMethod {
    /// Parse a method name with the RRF smoothing constant used when requested.
    pub fn from_name(name: &str, k: usize) -> Result<Self, RankingError> {
        match name {
            "weighted_sum" | "weighted-sum" | "weighted" => Ok(Self::WeightedSum),
            "rrf" | "reciprocal_rank_fusion" | "reciprocal-rank-fusion" => Ok(Self::Rrf { k }),
            _ => Err(RankingError::InvalidMethod(name.to_string())),
        }
    }
}

/// Errors returned by ranking helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RankingError {
    LengthMismatch {
        lex_ranks: usize,
        cosine_scores: usize,
    },
    InvalidAlpha,
    InvalidK,
    InvalidMethod(String),
}

impl fmt::Display for RankingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch {
                lex_ranks,
                cosine_scores,
            } => write!(
                f,
                "lex_ranks length ({lex_ranks}) must match cosine_scores length ({cosine_scores})"
            ),
            Self::InvalidAlpha => write!(f, "alpha must be finite and between 0.0 and 1.0"),
            Self::InvalidK => write!(f, "k must be greater than 0"),
            Self::InvalidMethod(method) => write!(f, "unsupported ranking method: {method}"),
        }
    }
}

impl std::error::Error for RankingError {}

/// Combine lexical ranks and cosine scores into a stable ranked list.
///
/// Returns `(item_index, score)` pairs sorted by descending score. Identical
/// scores keep deterministic original-index order.
pub fn hybrid_rank(
    lex_ranks: &[usize],
    cosine_scores: &[f32],
    alpha: f32,
    method: HybridRankMethod,
) -> Result<Vec<(usize, f32)>, RankingError> {
    validate_hybrid_inputs(lex_ranks, cosine_scores, alpha)?;

    match method {
        HybridRankMethod::WeightedSum => Ok(weighted_sum_rank(lex_ranks, cosine_scores, alpha)),
        HybridRankMethod::Rrf { k } => {
            if k == 0 {
                return Err(RankingError::InvalidK);
            }
            Ok(rrf_hybrid_rank(lex_ranks, cosine_scores, k))
        }
    }
}

/// String-wrapper convenience for callers that do not want to construct the enum.
pub fn hybrid_rank_by_name(
    lex_ranks: &[usize],
    cosine_scores: &[f32],
    alpha: f32,
    method: &str,
    k: usize,
) -> Result<Vec<(usize, f32)>, RankingError> {
    if k == 0 {
        return Err(RankingError::InvalidK);
    }

    let method = HybridRankMethod::from_name(method, k)?;
    hybrid_rank(lex_ranks, cosine_scores, alpha, method)
}

/// Fuse ranked lists using reciprocal rank fusion.
///
/// Each inner list contains item IDs in ranked order. The score contribution is
/// `1 / (k + rank)`, with `rank` starting at 0.
pub fn rrf_fuse(ranked_lists: &[Vec<usize>], k: usize) -> Result<Vec<(usize, f32)>, RankingError> {
    if k == 0 {
        return Err(RankingError::InvalidK);
    }

    let mut scores: HashMap<usize, f32> = HashMap::new();
    let mut first_seen: HashMap<usize, usize> = HashMap::new();
    let mut seen_order = 0usize;

    for ranked_list in ranked_lists {
        for (rank, &item_id) in ranked_list.iter().enumerate() {
            first_seen.entry(item_id).or_insert_with(|| {
                let order = seen_order;
                seen_order += 1;
                order
            });
            *scores.entry(item_id).or_insert(0.0) += 1.0 / (k + rank) as f32;
        }
    }

    let mut ranked: Vec<_> = scores.into_iter().collect();
    ranked.sort_by(|(left_id, left_score), (right_id, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| first_seen[left_id].cmp(&first_seen[right_id]))
            .then_with(|| left_id.cmp(right_id))
    });

    Ok(ranked)
}

fn validate_hybrid_inputs(
    lex_ranks: &[usize],
    cosine_scores: &[f32],
    alpha: f32,
) -> Result<(), RankingError> {
    if lex_ranks.len() != cosine_scores.len() {
        return Err(RankingError::LengthMismatch {
            lex_ranks: lex_ranks.len(),
            cosine_scores: cosine_scores.len(),
        });
    }
    if !(0.0..=1.0).contains(&alpha) || !alpha.is_finite() {
        return Err(RankingError::InvalidAlpha);
    }
    Ok(())
}

fn weighted_sum_rank(lex_ranks: &[usize], cosine_scores: &[f32], alpha: f32) -> Vec<(usize, f32)> {
    let mut ranked: Vec<_> = lex_ranks
        .iter()
        .zip(cosine_scores.iter())
        .enumerate()
        .map(|(index, (&lex_rank, &cosine))| {
            let lex_score = 1.0 / (1 + lex_rank) as f32;
            let score = alpha * lex_score + (1.0 - alpha) * cosine;
            (index, score)
        })
        .collect();

    ranked.sort_by(|(left_index, left_score), (right_index, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left_index.cmp(right_index))
    });
    ranked
}

fn rrf_hybrid_rank(lex_ranks: &[usize], cosine_scores: &[f32], k: usize) -> Vec<(usize, f32)> {
    let mut cosine_ranked: Vec<_> = cosine_scores.iter().copied().enumerate().collect();
    cosine_ranked.sort_by(|(left_index, left_score), (right_index, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left_index.cmp(right_index))
    });

    let mut cosine_ranks = vec![0usize; cosine_scores.len()];
    for (rank, (index, _)) in cosine_ranked.into_iter().enumerate() {
        cosine_ranks[index] = rank;
    }

    let mut ranked: Vec<_> = lex_ranks
        .iter()
        .zip(cosine_ranks.iter())
        .enumerate()
        .map(|(index, (&lex_rank, &cosine_rank))| {
            let score = (1.0 / (k + lex_rank) as f32) + (1.0 / (k + cosine_rank) as f32);
            (index, score)
        })
        .collect();

    ranked.sort_by(|(left_index, left_score), (right_index, right_score)| {
        right_score
            .total_cmp(left_score)
            .then_with(|| left_index.cmp(right_index))
    });
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(left: f32, right: f32) {
        assert!((left - right).abs() < 1e-6, "{left} != {right}");
    }

    #[test]
    fn weighted_sum_uses_expected_formula() {
        let ranked = hybrid_rank(
            &[0, 2, 1],
            &[0.20, 0.95, 0.50],
            0.5,
            HybridRankMethod::WeightedSum,
        )
        .unwrap();

        assert_eq!(
            ranked.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            vec![1, 0, 2]
        );
        assert_close(ranked[0].1, 0.5 * (1.0 / 3.0) + 0.5 * 0.95);
        assert_close(ranked[1].1, 0.5 * 1.0 + 0.5 * 0.20);
        assert_close(ranked[2].1, 0.5 * (1.0 / 2.0) + 0.5 * 0.50);
    }

    #[test]
    fn weighted_sum_preserves_index_order_for_ties() {
        let ranked = hybrid_rank(
            &[1, 1, 1],
            &[0.25, 0.25, 0.25],
            0.5,
            HybridRankMethod::WeightedSum,
        )
        .unwrap();

        assert_eq!(ranked, vec![(0, 0.375), (1, 0.375), (2, 0.375)]);
    }

    #[test]
    fn rrf_fuse_uses_standard_formula() {
        let ranked = rrf_fuse(&[vec![10, 20, 30], vec![20, 10]], 60).unwrap();

        assert_eq!(ranked[0].0, 10);
        assert_close(ranked[0].1, (1.0 / 60.0) + (1.0 / 61.0));
        assert_eq!(ranked[1].0, 20);
        assert_close(ranked[1].1, (1.0 / 61.0) + (1.0 / 60.0));
        assert_eq!(ranked[2].0, 30);
        assert_close(ranked[2].1, 1.0 / 62.0);
    }

    #[test]
    fn rrf_hybrid_derives_cosine_ranks_stably() {
        let ranked = hybrid_rank(
            &[2, 1, 0],
            &[0.7, 0.7, 0.2],
            0.5,
            HybridRankMethod::Rrf { k: 60 },
        )
        .unwrap();

        assert_eq!(
            ranked.iter().map(|(i, _)| *i).collect::<Vec<_>>(),
            vec![0, 2, 1]
        );
        assert_close(ranked[0].1, (1.0 / 62.0) + (1.0 / 60.0));
    }

    #[test]
    fn validates_inputs() {
        assert_eq!(
            hybrid_rank(&[0], &[], 0.5, HybridRankMethod::WeightedSum).unwrap_err(),
            RankingError::LengthMismatch {
                lex_ranks: 1,
                cosine_scores: 0,
            }
        );
        assert_eq!(
            hybrid_rank(&[0], &[0.1], 1.5, HybridRankMethod::WeightedSum).unwrap_err(),
            RankingError::InvalidAlpha
        );
        assert_eq!(rrf_fuse(&[vec![1]], 0).unwrap_err(), RankingError::InvalidK);
        assert_eq!(
            HybridRankMethod::from_name("nope", 60).unwrap_err(),
            RankingError::InvalidMethod("nope".to_string())
        );
    }
}
