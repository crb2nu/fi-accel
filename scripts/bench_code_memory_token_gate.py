#!/usr/bin/env python3
"""Benchmark py-codebase-memory-shaped token counting gates.

The benchmark keeps local PythonIndexer chunking as the downstream contract and
compares heuristic token estimates with exact fi_accel.count_tokens calls. It
also reports the full fi_accel.chunk_code path now that the chunk parity fixture
has a passing gate.
"""

from __future__ import annotations

import argparse
import json
import statistics
import time
import tracemalloc
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import codebase_memory.indexers.base as base_indexer_module
import codebase_memory.indexers.python_indexer as python_indexer_module
from codebase_memory.indexers.python_indexer import PythonIndexer

from fi_accel import chunk_code, count_tokens


@dataclass
class BenchResult:
    name: str
    median_ms: float
    min_ms: float
    max_ms: float
    median_peak_kib: float
    files: int
    chunks: int
    total_tokens: int


def _collect_python_files(repo: Path) -> list[Path]:
    src = repo / "src" / "codebase_memory"
    return sorted(
        path
        for path in src.rglob("*.py")
        if "__pycache__" not in path.parts and path.is_file()
    )


def _force_local_heuristic() -> None:
    base_indexer_module._token_mode = "heuristic"
    base_indexer_module._fi_accel_count_tokens = None
    base_indexer_module._fi_accel_checked = False
    python_indexer_module._chunk_mode = "local"
    python_indexer_module._fi_accel_chunk_code = None
    python_indexer_module._fi_accel_chunk_checked = False


def _local_chunks(files: list[Path], repo: Path) -> list[Any]:
    _force_local_heuristic()
    indexer = PythonIndexer(max_chunk_tokens=512, overlap_tokens=64)
    chunks: list[Any] = []
    for path in files:
        chunks.extend(
            indexer.parse(
                path.read_text(encoding="utf-8", errors="ignore"),
                path.relative_to(repo),
            )
        )
    return chunks


def _local_heuristic(files: list[Path], repo: Path, _model: str) -> tuple[int, int]:
    chunks = _local_chunks(files, repo)
    return len(chunks), sum(chunk.token_count for chunk in chunks)


def _local_exact_count_tokens(files: list[Path], repo: Path, model: str) -> tuple[int, int]:
    chunks = _local_chunks(files, repo)
    return len(chunks), sum(count_tokens(chunk.content, model=model) for chunk in chunks)


def _fi_accel_chunker_exact(files: list[Path], repo: Path, model: str) -> tuple[int, int]:
    chunks: list[dict[str, Any]] = []
    for path in files:
        chunks.extend(
            chunk_code(
                path.read_text(encoding="utf-8", errors="ignore"),
                "python",
                max_tokens=512,
                overlap_tokens=64,
                file_path=str(path.relative_to(repo)),
                model=model,
            )
        )
    return len(chunks), sum(int(chunk["token_count"]) for chunk in chunks)


def _measure(
    name: str,
    repeats: int,
    files: list[Path],
    repo: Path,
    model: str,
    fn: Any,
) -> BenchResult:
    durations: list[float] = []
    peaks: list[float] = []
    chunks = 0
    total_tokens = 0
    for _ in range(repeats):
        tracemalloc.start()
        start = time.perf_counter()
        chunks, total_tokens = fn(files, repo, model)
        durations.append((time.perf_counter() - start) * 1000.0)
        _, peak = tracemalloc.get_traced_memory()
        tracemalloc.stop()
        peaks.append(peak / 1024.0)

    return BenchResult(
        name=name,
        median_ms=statistics.median(durations),
        min_ms=min(durations),
        max_ms=max(durations),
        median_peak_kib=statistics.median(peaks),
        files=len(files),
        chunks=chunks,
        total_tokens=total_tokens,
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo",
        type=Path,
        default=Path("/Users/cblevins/workspace/libs/py-codebase-memory"),
    )
    parser.add_argument("--repeats", type=int, default=7)
    parser.add_argument("--model", default="gpt-4")
    args = parser.parse_args()

    repo = args.repo.resolve()
    files = _collect_python_files(repo)
    if not files:
        raise SystemExit(f"no Python files found under {repo / 'src' / 'codebase_memory'}")

    results = [
        _measure("local_heuristic", args.repeats, files, repo, args.model, _local_heuristic),
        _measure(
            "local_cached_fi_accel_count_tokens",
            args.repeats,
            files,
            repo,
            args.model,
            _local_exact_count_tokens,
        ),
        _measure(
            "fi_accel_chunker_cached_token_counts",
            args.repeats,
            files,
            repo,
            args.model,
            _fi_accel_chunker_exact,
        ),
    ]

    output = {
        "repo": str(repo),
        "source_glob": "src/codebase_memory/**/*.py",
        "repeats": args.repeats,
        "model": args.model,
        "input_bytes": sum(path.stat().st_size for path in files),
        "results": [asdict(result) for result in results],
    }
    print(json.dumps(output, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
