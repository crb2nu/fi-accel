import math

import pytest

from fi_accel import hybrid_rank, rrf_fuse


def assert_close(left, right):
    assert math.isclose(left, right, rel_tol=1e-6, abs_tol=1e-6)


def test_hybrid_rank_weighted_sum_formula():
    ranked = hybrid_rank([0, 2, 1], [0.2, 0.95, 0.5], alpha=0.5)

    assert [item for item, _ in ranked] == [1, 0, 2]
    assert_close(ranked[0][1], 0.5 * (1.0 / 3.0) + 0.5 * 0.95)
    assert_close(ranked[1][1], 0.5 * 1.0 + 0.5 * 0.2)
    assert_close(ranked[2][1], 0.5 * (1.0 / 2.0) + 0.5 * 0.5)


def test_hybrid_rank_stable_ties():
    ranked = hybrid_rank([1, 1, 1], [0.25, 0.25, 0.25])

    assert ranked == [(0, 0.375), (1, 0.375), (2, 0.375)]


def test_hybrid_rank_rrf_method():
    ranked = hybrid_rank([2, 1, 0], [0.7, 0.7, 0.2], method="rrf", k=60)

    assert [item for item, _ in ranked] == [0, 2, 1]
    assert_close(ranked[0][1], (1.0 / 62.0) + (1.0 / 60.0))


def test_rrf_fuse_standard_formula():
    ranked = rrf_fuse([[10, 20, 30], [20, 10]], k=60)

    assert [item for item, _ in ranked] == [10, 20, 30]
    assert_close(ranked[0][1], (1.0 / 60.0) + (1.0 / 61.0))
    assert_close(ranked[1][1], (1.0 / 61.0) + (1.0 / 60.0))
    assert_close(ranked[2][1], 1.0 / 62.0)


@pytest.mark.parametrize(
    "kwargs",
    [
        {"lex_ranks": [0], "cosine_scores": []},
        {"lex_ranks": [0], "cosine_scores": [0.1], "alpha": -0.1},
        {"lex_ranks": [0], "cosine_scores": [0.1], "method": "rrf", "k": 0},
        {"lex_ranks": [0], "cosine_scores": [0.1], "method": "unknown"},
    ],
)
def test_hybrid_rank_rejects_invalid_inputs(kwargs):
    with pytest.raises(ValueError):
        hybrid_rank(**kwargs)


def test_rrf_fuse_rejects_invalid_k():
    with pytest.raises(ValueError):
        rrf_fuse([[1]], k=0)
