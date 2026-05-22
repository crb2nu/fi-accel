#!/usr/bin/env python3
"""Benchmark a py-codebase-memory-shaped ingestion prototype.

The baseline uses py-codebase-memory's current local PythonIndexer path plus
stdlib hashing/ranking glue. The fi-accel path uses source decoding, code
chunking, batch hashing, and hybrid ranking helpers from fi_accel.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import statistics
import time
import tracemalloc
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

from codebase_memory.indexers.python_indexer import PythonIndexer

from fi_accel import chunk_code, detect_encoding, hybrid_rank, sha256_batch


@dataclass
class BenchResult:
    name: str
    median_ms: float
    min_ms: float
    max_ms: float
    median_peak_kib: float
    files: int
    chunks: int
    top_ids: list[str]


def _collect_python_files(repo: Path) -> list[Path]:
    src = repo / "src" / "codebase_memory"
    return sorted(
        path
        for path in src.rglob("*.py")
        if "__pycache__" not in path.parts and path.is_file()
    )


def _query_score(query_terms: set[str], text: str) -> float:
    haystack = text.lower()
    if not query_terms:
        return 0.0
    hits = sum(1 for term in query_terms if term in haystack)
    return hits / len(query_terms)


def _lex_ranks(scores: list[float]) -> list[int]:
    ordered = sorted(range(len(scores)), key=lambda idx: (-scores[idx], idx))
    ranks = [0] * len(scores)
    for rank, idx in enumerate(ordered):
        ranks[idx] = rank
    return ranks


def _python_hybrid_rank(
    lex_ranks: list[int], cosine_scores: list[float], alpha: float = 0.5
) -> list[tuple[int, float]]:
    scored = [
        (idx, alpha * (1.0 / (1 + rank)) + (1.0 - alpha) * cosine_scores[idx])
        for idx, rank in enumerate(lex_ranks)
    ]
    return sorted(scored, key=lambda item: (-item[1], item[0]))


def _local_ingest(files: list[Path], repo: Path, query: str) -> tuple[list[str], int]:
    indexer = PythonIndexer(max_chunk_tokens=512, overlap_tokens=64)
    records: list[dict[str, Any]] = []
    for path in files:
        rel = path.relative_to(repo)
        content = path.read_text(encoding="utf-8", errors="ignore")
        for chunk in indexer.parse(content, rel):
            payload = chunk.model_dump(mode="json")
            digest_input = f"{chunk.file_path}:{chunk.start_line}:{chunk.content[:100]}".encode()
            payload["content_hash"] = hashlib.sha256(digest_input).hexdigest()
            payload["search_text"] = "\n\n".join(
                part for part in [chunk.docstring, chunk.signature, chunk.content] if part
            )
            records.append(payload)

    return _rank_record_ids(records, query, ranker=_python_hybrid_rank), len(records)


def _fi_accel_ingest(files: list[Path], repo: Path, query: str) -> tuple[list[str], int]:
    records: list[dict[str, Any]] = []
    hash_inputs: list[bytes] = []
    for path in files:
        rel = path.relative_to(repo)
        raw = path.read_bytes()
        encoding = detect_encoding(raw).get("encoding") or "utf-8"
        content = raw.decode(str(encoding), errors="replace")
        chunks = chunk_code(
            content,
            "python",
            max_tokens=512,
            overlap_tokens=64,
            file_path=str(rel),
        )
        for chunk in chunks:
            search_text = "\n\n".join(
                str(chunk.get(field) or "")
                for field in ("docstring", "signature", "content")
                if chunk.get(field)
            )
            record = dict(chunk)
            record["search_text"] = search_text
            records.append(record)
            hash_inputs.append(
                f"{rel}:{chunk.get('start_line')}:{str(chunk.get('content', ''))[:100]}".encode()
            )

    for record, digest in zip(records, sha256_batch(hash_inputs), strict=True):
        record["content_hash"] = digest

    return _rank_record_ids(records, query, ranker=hybrid_rank), len(records)


def _rank_record_ids(
    records: list[dict[str, Any]],
    query: str,
    ranker: Any,
) -> list[str]:
    query_terms = {term for term in query.lower().split() if term}
    lexical_scores = [_query_score(query_terms, str(record.get("search_text") or "")) for record in records]
    lex_ranks = _lex_ranks(lexical_scores)

    max_len = max((len(str(record.get("search_text") or "")) for record in records), default=1)
    cosine_like_scores = [
        min(1.0, math.sqrt(score) * 0.85 + (len(str(record.get("search_text") or "")) / max_len) * 0.15)
        for score, record in zip(lexical_scores, records, strict=True)
    ]

    ranked = ranker(lex_ranks, cosine_like_scores, 0.5)
    return [str(records[idx].get("id") or records[idx].get("content_hash")) for idx, _ in ranked[:5]]


def _measure(name: str, repeats: int, files: list[Path], repo: Path, query: str, fn: Any) -> BenchResult:
    durations: list[float] = []
    peaks: list[float] = []
    top_ids: list[str] = []
    chunks = 0
    for _ in range(repeats):
        tracemalloc.start()
        start = time.perf_counter()
        top_ids, chunk_count = fn(files, repo, query)
        durations.append((time.perf_counter() - start) * 1000.0)
        _, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        peaks.append(peak / 1024.0)
        chunks = max(chunks, chunk_count)

    return BenchResult(
        name=name,
        median_ms=statistics.median(durations),
        min_ms=min(durations),
        max_ms=max(durations),
        median_peak_kib=statistics.median(peaks),
        files=len(files),
        chunks=chunks,
        top_ids=top_ids,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo",
        type=Path,
        default=Path("/Users/cblevins/workspace/libs/py-codebase-memory"),
    )
    parser.add_argument("--repeats", type=int, default=7)
    parser.add_argument("--query", default="python indexer token counting")
    args = parser.parse_args()

    repo = args.repo.resolve()
    files = _collect_python_files(repo)
    if not files:
        raise SystemExit(f"no Python files found under {repo}")

    local = _measure("local", args.repeats, files, repo, args.query, _local_ingest)
    accelerated = _measure("fi_accel", args.repeats, files, repo, args.query, _fi_accel_ingest)

    output = {
        "repo": str(repo),
        "query": args.query,
        "repeats": args.repeats,
        "input_bytes": sum(path.stat().st_size for path in files),
        "results": [asdict(local), asdict(accelerated)],
        "speed_ratio_fi_accel_vs_local": accelerated.median_ms / local.median_ms,
        "peak_python_allocation_ratio_fi_accel_vs_local": (
            accelerated.median_peak_kib / local.median_peak_kib
        ),
    }
    print(json.dumps(output, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
