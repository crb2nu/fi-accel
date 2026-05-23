#!/usr/bin/env python3
"""Check py-codebase-memory Python chunk parity against fi_accel.chunk_code."""

from __future__ import annotations

import argparse
import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

from codebase_memory.indexers.python_indexer import PythonIndexer

from fi_accel import chunk_code


COMPARE_FIELDS = (
    "id",
    "chunk_type",
    "file_path",
    "start_line",
    "end_line",
    "start_column",
    "end_column",
    "name",
    "parent_name",
    "parent_type",
    "calls",
    "definitions",
)


@dataclass
class Divergence:
    index: int
    file_path: str | None
    reason: str
    local: dict[str, Any] | None
    fi_accel: dict[str, Any] | None
    differing_fields: dict[str, dict[str, Any]]


def _collect_python_files(repo: Path) -> list[Path]:
    src = repo / "src" / "codebase_memory"
    return sorted(
        path
        for path in src.rglob("*.py")
        if "__pycache__" not in path.parts and path.is_file()
    )


def _normalize_local_chunk(chunk: Any) -> dict[str, Any]:
    parent_type = chunk.parent_type.value if chunk.parent_type else None
    return {
        "id": chunk.id,
        "chunk_type": chunk.chunk_type.value,
        "file_path": str(chunk.file_path),
        "start_line": chunk.start_line,
        "end_line": chunk.end_line,
        "start_column": chunk.start_column,
        "end_column": chunk.end_column,
        "name": chunk.name,
        "parent_name": chunk.parent_name,
        "parent_type": parent_type,
        "calls": sorted(chunk.calls),
        "definitions": sorted(chunk.definitions),
    }


def _normalize_fi_accel_chunk(chunk: dict[str, Any]) -> dict[str, Any]:
    return {
        "id": chunk.get("id"),
        "chunk_type": chunk.get("chunk_type"),
        "file_path": str(chunk.get("file_path")),
        "start_line": chunk.get("start_line"),
        "end_line": chunk.get("end_line"),
        "start_column": chunk.get("start_column"),
        "end_column": chunk.get("end_column"),
        "name": chunk.get("name"),
        "parent_name": chunk.get("parent_name"),
        "parent_type": chunk.get("parent_type"),
        "calls": sorted(chunk.get("calls") or []),
        "definitions": sorted(chunk.get("definitions") or []),
    }


def _first_divergence(
    local_chunks: list[dict[str, Any]],
    fi_accel_chunks: list[dict[str, Any]],
) -> Divergence | None:
    for index, (local, accelerated) in enumerate(zip(local_chunks, fi_accel_chunks, strict=False)):
        differing_fields = {
            field: {"local": local[field], "fi_accel": accelerated[field]}
            for field in COMPARE_FIELDS
            if local[field] != accelerated[field]
        }
        if differing_fields:
            return Divergence(
                index=index,
                file_path=str(local.get("file_path") or accelerated.get("file_path")),
                reason="field_mismatch",
                local=local,
                fi_accel=accelerated,
                differing_fields=differing_fields,
            )

    if len(local_chunks) != len(fi_accel_chunks):
        index = min(len(local_chunks), len(fi_accel_chunks))
        local = local_chunks[index] if index < len(local_chunks) else None
        accelerated = fi_accel_chunks[index] if index < len(fi_accel_chunks) else None
        file_path = None
        if local is not None:
            file_path = str(local.get("file_path"))
        elif accelerated is not None:
            file_path = str(accelerated.get("file_path"))
        return Divergence(
            index=index,
            file_path=file_path,
            reason="chunk_count_mismatch",
            local=local,
            fi_accel=accelerated,
            differing_fields={},
        )

    return None


def check_parity(repo: Path) -> dict[str, Any]:
    repo = repo.resolve()
    files = _collect_python_files(repo)
    if not files:
        raise SystemExit(f"no Python files found under {repo / 'src' / 'codebase_memory'}")

    indexer = PythonIndexer(max_chunk_tokens=512, overlap_tokens=64)
    local_chunks: list[dict[str, Any]] = []
    fi_accel_chunks: list[dict[str, Any]] = []

    for path in files:
        rel_path = path.relative_to(repo)
        content = path.read_text(encoding="utf-8", errors="ignore")
        local_chunks.extend(
            _normalize_local_chunk(chunk) for chunk in indexer.parse(content, rel_path)
        )
        fi_accel_chunks.extend(
            _normalize_fi_accel_chunk(chunk)
            for chunk in chunk_code(
                content,
                "python",
                max_tokens=512,
                overlap_tokens=64,
                file_path=str(rel_path),
            )
        )

    divergence = _first_divergence(local_chunks, fi_accel_chunks)
    return {
        "repo": str(repo),
        "source_glob": "src/codebase_memory/**/*.py",
        "files": len(files),
        "local_chunks": len(local_chunks),
        "fi_accel_chunks": len(fi_accel_chunks),
        "compared_fields": list(COMPARE_FIELDS),
        "parity": divergence is None,
        "first_divergence": asdict(divergence) if divergence else None,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--repo",
        type=Path,
        default=Path("/Users/cblevins/workspace/libs/py-codebase-memory"),
    )
    args = parser.parse_args()

    result = check_parity(args.repo)
    print(json.dumps(result, indent=2, sort_keys=True))
    if not result["parity"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
