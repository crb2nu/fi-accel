"""Pure-Python fallback implementations for :mod:`fi_accel`.

The native extension is the preferred runtime. This module is intentionally
slow but dependency-free so downstream packages can import a stable API when a
platform wheel is unavailable.
"""

from __future__ import annotations

import difflib
import email
import hashlib
import html as html_lib
import json
import math
import re
from dataclasses import dataclass
from typing import Any, BinaryIO, Iterable


def _estimate_tokens(text: str) -> int:
    if not text:
        return 0
    words = re.findall(r"\S+", text)
    return max(1, int(len(words) * 1.3))


def count_tokens(text: str, model: str = "gpt-4") -> int:
    del model
    return _estimate_tokens(text)


def truncate_to_tokens(text: str, max_tokens: int, model: str = "gpt-4") -> str:
    del model
    if max_tokens <= 0:
        return ""
    if count_tokens(text) <= max_tokens:
        return text
    max_chars = max_tokens * 4
    return text[:max_chars]


def split_by_tokens(
    text: str, chunk_size: int, overlap: int = 0, model: str = "gpt-4"
) -> list[str]:
    del model
    if chunk_size <= 0:
        return []
    words = text.split()
    if not words:
        return []
    step = max(1, chunk_size - overlap)
    chunks: list[str] = []
    for start in range(0, len(words), step):
        chunks.append(" ".join(words[start : start + chunk_size]))
        if start + chunk_size >= len(words):
            break
    return chunks


class Tokenizer:
    def __init__(self, model: str = "gpt-4") -> None:
        self._model = model

    def count(self, text: str) -> int:
        return count_tokens(text, self._model)

    def truncate(self, text: str, max_tokens: int) -> str:
        return truncate_to_tokens(text, max_tokens, self._model)

    def split(self, text: str, chunk_size: int, overlap: int) -> list[str]:
        return split_by_tokens(text, chunk_size, overlap, self._model)

    def encode(self, text: str) -> list[int]:
        return [ord(ch) for ch in text]

    def decode(self, tokens: Iterable[int]) -> str:
        return "".join(chr(token) for token in tokens)


def normalize_l2(embeddings: list[list[float]]) -> list[list[float]]:
    result: list[list[float]] = []
    for vector in embeddings:
        norm = math.sqrt(sum(value * value for value in vector))
        if norm:
            result.append([value / norm for value in vector])
        else:
            result.append(list(vector))
    return result


def cosine_similarity(a: list[float], b: list[float]) -> float:
    if len(a) != len(b):
        raise ValueError("Vectors must have same length")
    return sum(left * right for left, right in zip(a, b))


def cosine_similarity_matrix(
    a: list[list[float]], b: list[list[float]]
) -> list[list[float]]:
    return [[cosine_similarity(left, right) for right in b] for left in a]


def top_k_similar(
    a: list[list[float]], b: list[list[float]], k: int
) -> list[list[tuple[int, float]]]:
    results: list[list[tuple[int, float]]] = []
    for left in a:
        scores = [(idx, cosine_similarity(left, right)) for idx, right in enumerate(b)]
        scores.sort(key=lambda item: item[1], reverse=True)
        results.append(scores[:k])
    return results


def compute_centroid(embeddings: list[list[float]]) -> list[float] | None:
    if not embeddings:
        return None
    dim = len(embeddings[0])
    if any(len(vector) != dim for vector in embeddings):
        raise ValueError("all embeddings must have same dimensionality")
    count = float(len(embeddings))
    return [sum(vector[i] for vector in embeddings) / count for i in range(dim)]


def prepare_embedding_batch(
    texts: list[str], max_tokens: int, model: str = "gpt-4"
) -> list[tuple[str, int, bool]]:
    prepared: list[tuple[str, int, bool]] = []
    for text in texts:
        tokens = count_tokens(text, model)
        if tokens <= max_tokens:
            prepared.append((text, tokens, False))
        else:
            truncated = truncate_to_tokens(text, max_tokens, model)
            prepared.append((truncated, count_tokens(truncated, model), True))
    return prepared


def hybrid_rank(
    lex_ranks: list[int],
    cosine_scores: list[float],
    alpha: float = 0.5,
    method: str = "weighted_sum",
    k: int = 60,
) -> list[tuple[int, float]]:
    if len(lex_ranks) != len(cosine_scores):
        raise ValueError("lex_ranks length must match cosine_scores length")
    if not 0.0 <= alpha <= 1.0 or not math.isfinite(alpha):
        raise ValueError("alpha must be finite and between 0 and 1")
    if k <= 0:
        raise ValueError("k must be greater than 0")

    if method in {"weighted_sum", "weighted-sum", "weighted"}:
        scored = [
            (idx, alpha * (1.0 / (1 + rank)) + (1.0 - alpha) * score)
            for idx, (rank, score) in enumerate(zip(lex_ranks, cosine_scores))
        ]
    elif method in {"rrf", "reciprocal_rank_fusion", "reciprocal-rank-fusion"}:
        cosine_order = sorted(
            enumerate(cosine_scores), key=lambda item: (-item[1], item[0])
        )
        cosine_ranks = {idx: rank for rank, (idx, _) in enumerate(cosine_order)}
        scored = [
            (idx, (1.0 / (k + rank)) + (1.0 / (k + cosine_ranks[idx])))
            for idx, rank in enumerate(lex_ranks)
        ]
    else:
        raise ValueError(f"unsupported ranking method: {method}")

    return sorted(scored, key=lambda item: (-item[1], item[0]))


def rrf_fuse(ranked_lists: list[list[Any]], k: int = 60) -> list[tuple[Any, float]]:
    if k <= 0:
        raise ValueError("k must be greater than 0")
    scores: dict[Any, float] = {}
    first_seen: dict[Any, int] = {}
    order = 0
    for ranked_list in ranked_lists:
        for rank, item in enumerate(ranked_list):
            if item not in first_seen:
                first_seen[item] = order
                order += 1
            scores[item] = scores.get(item, 0.0) + (1.0 / (k + rank))
    return sorted(scores.items(), key=lambda item: (-item[1], first_seen[item[0]]))


class FuzzyMatcher:
    def __init__(self, threshold: float = 0.6, max_offset: int = 100) -> None:
        self.threshold = threshold
        self.max_offset = max_offset

    def similarity(self, a: str, b: str) -> float:
        return difflib.SequenceMatcher(None, a, b).ratio()

    def apply_patch(self, patch: str, file: str) -> dict[str, Any]:
        del patch
        return {"content": file, "applied": 0, "fuzzy_applied": 0, "failed": 0, "details": []}


def parse_unified_diff(patch: str) -> list[dict[str, int]]:
    hunks: list[dict[str, int]] = []
    header_re = re.compile(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@")
    for line in patch.splitlines():
        match = header_re.match(line)
        if match:
            old_start, old_count, new_start, new_count = match.groups()
            hunks.append(
                {
                    "old_start": int(old_start),
                    "old_count": int(old_count or "1"),
                    "new_start": int(new_start),
                    "new_count": int(new_count or "1"),
                }
            )
    return hunks


class LogReader:
    def __init__(self, content: str = "") -> None:
        self._content = content
        self._lines = content.splitlines()

    def append(self, content: str) -> list[str]:
        old_len = len(self._lines)
        self._content += content
        self._lines = self._content.splitlines()
        return self._lines[old_len:]

    def tail(self, n: int) -> list[str]:
        return self._lines[-n:] if n else []

    def filter(self, pattern: str) -> list[str]:
        compiled = re.compile(pattern)
        return [line for line in self._lines if compiled.search(line)]

    def search(self, substring: str) -> list[tuple[int, str]]:
        return [
            (idx + 1, line)
            for idx, line in enumerate(self._lines)
            if substring in line
        ]

    def context(self, line_num: int, before: int, after: int) -> list[str]:
        start = max(0, line_num - before - 1)
        end = min(len(self._lines), line_num + after)
        return self._lines[start:end]

    def all_lines(self) -> list[str]:
        return list(self._lines)

    def __len__(self) -> int:
        return len(self._lines)


def parse_ndjson(data: bytes) -> list[Any]:
    values: list[Any] = []
    for line in data.decode("utf-8", errors="replace").splitlines():
        if line.strip():
            try:
                values.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    return values


class NDJSONReader:
    def __init__(self, initial_content: bytes = b"") -> None:
        self._buffer = b""
        self._values: list[Any] = []
        self._line = 0
        if initial_content:
            self.append(initial_content)

    def append(self, data: bytes) -> list[Any]:
        payload = self._buffer + data
        lines = payload.split(b"\n")
        self._buffer = lines.pop() if payload and not payload.endswith(b"\n") else b""
        emitted: list[Any] = []
        for raw in lines:
            self._line += 1
            if not raw.strip():
                continue
            try:
                value = json.loads(raw.decode("utf-8"))
            except Exception as exc:
                raise ValueError(f"invalid NDJSON at line {self._line}: {exc}") from exc
            self._values.append(value)
            emitted.append(value)
        return emitted

    def tail(self, n: int) -> list[Any]:
        return self._values[-n:] if n else []

    def buffered_len(self) -> int:
        return len(self._buffer)

    def __len__(self) -> int:
        return len(self._values)


def parse_jsonrpc_batch(data: bytes) -> list[dict[str, Any]]:
    return [json.loads(line) for line in data.decode().splitlines() if line.strip()]


def parse_jsonrpc_lines(data: bytes) -> list[dict[str, Any]]:
    return parse_jsonrpc_batch(data)


def serialize_jsonrpc_lines(messages: list[dict[str, Any]]) -> bytes:
    return ("\n".join(json.dumps(message, separators=(",", ":")) for message in messages) + "\n").encode()


def parse_sse_events(data: bytes) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for block in data.decode("utf-8", errors="replace").split("\n\n"):
        if not block.strip():
            continue
        event: dict[str, Any] = {"event": None, "data": "", "id": None, "retry": None}
        data_lines: list[str] = []
        for line in block.splitlines():
            if line.startswith("event:"):
                event["event"] = line[6:].strip()
            elif line.startswith("data:"):
                data_lines.append(line[5:].lstrip())
            elif line.startswith("id:"):
                event["id"] = line[3:].strip()
            elif line.startswith("retry:"):
                try:
                    event["retry"] = int(line[6:].strip())
                except ValueError:
                    event["retry"] = None
        event["data"] = "\n".join(data_lines)
        events.append(event)
    return events


def extract_mcp_http_meta(headers: bytes) -> dict[str, Any]:
    text = headers.decode("utf-8", errors="replace")
    parsed: dict[str, str] = {}
    for line in text.splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            parsed[key.lower().strip()] = value.strip()
    accept = parsed.get("accept", "")
    return {
        "accepts_sse": "text/event-stream" in accept,
        "accepts_json": "application/json" in accept,
        "session_id": parsed.get("mcp-session-id"),
        "protocol_version": parsed.get("mcp-protocol-version"),
    }


def project_eventlog(
    data: bytes,
    timestamp_field: str = "timestamp",
    event_type_field: str = "event_type",
    actor_id_field: str | None = "agent_id",
    actor_type_field: str | None = "agent_type",
    data_field: str | None = None,
    limit: int | None = None,
) -> list[dict[str, Any]]:
    rows = parse_ndjson(data)
    if limit is not None:
        rows = rows[:limit]
    events: list[dict[str, Any]] = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        events.append(
            {
                "timestamp": row.get(timestamp_field),
                "event_type": row.get(event_type_field),
                "actor_id": row.get(actor_id_field) if actor_id_field else None,
                "actor_type": row.get(actor_type_field) if actor_type_field else None,
                "data": row.get(data_field) if data_field else row,
            }
        )
    return events


def summarize_eventlog(*args: Any, **kwargs: Any) -> dict[str, Any]:
    events = project_eventlog(*args, **kwargs)
    by_type: dict[str, int] = {}
    by_actor: dict[str, int] = {}
    for event in events:
        event_type = str(event.get("event_type"))
        actor_id = str(event.get("actor_id"))
        by_type[event_type] = by_type.get(event_type, 0) + 1
        by_actor[actor_id] = by_actor.get(actor_id, 0) + 1
    return {"total": len(events), "by_event_type": by_type, "by_actor_id": by_actor}


def _field_value(row: dict[str, Any], field_path: str) -> Any:
    current: Any = row
    for segment in field_path.split("."):
        if not isinstance(current, dict) or segment not in current:
            return None
        current = current[segment]
    return current


def summarize_accel_observations(
    data: bytes,
    operation_field: str = "operation",
    service_field: str | None = "service",
    duration_ms_field: str = "duration_ms",
    input_bytes_field: str | None = "input_bytes",
    fallback_field: str | None = "fallback",
    limit: int | None = None,
) -> list[dict[str, Any]]:
    rows = parse_ndjson(data)
    if limit is not None:
        rows = rows[:limit]

    grouped: dict[tuple[str | None, str], dict[str, Any]] = {}
    for row in rows:
        if not isinstance(row, dict):
            continue
        operation = _field_value(row, operation_field)
        duration = _field_value(row, duration_ms_field)
        if operation in (None, ""):
            continue
        try:
            duration_ms = float(duration)
        except (TypeError, ValueError):
            continue
        if not math.isfinite(duration_ms) or duration_ms < 0:
            continue

        service = _field_value(row, service_field) if service_field else None
        key = (str(service) if service is not None else None, str(operation))
        group = grouped.setdefault(
            key,
            {
                "durations": [],
                "total_input_bytes": 0,
                "input_sample_count": 0,
                "fallback_count": 0,
            },
        )
        group["durations"].append(duration_ms)

        input_bytes = _field_value(row, input_bytes_field) if input_bytes_field else None
        try:
            parsed_input_bytes = int(input_bytes)
        except (TypeError, ValueError):
            parsed_input_bytes = None
        if parsed_input_bytes is not None and parsed_input_bytes >= 0:
            group["total_input_bytes"] += parsed_input_bytes
            group["input_sample_count"] += 1

        fallback = _field_value(row, fallback_field) if fallback_field else False
        if isinstance(fallback, str):
            fallback = fallback.lower() in {"true", "1", "yes"}
        if bool(fallback):
            group["fallback_count"] += 1

    summaries: list[dict[str, Any]] = []
    for (service, operation), group in grouped.items():
        durations = sorted(group["durations"])
        sample_count = len(durations)
        total_duration_ms = sum(durations)
        fallback_count = group["fallback_count"]
        fallback_rate = fallback_count / sample_count
        p95_index = min(len(durations) - 1, max(0, math.ceil(len(durations) * 0.95) - 1))
        summary: dict[str, Any] = {
            "operation": operation,
            "sample_count": sample_count,
            "total_duration_ms": total_duration_ms,
            "avg_duration_ms": total_duration_ms / sample_count,
            "p95_duration_ms": durations[p95_index],
            "max_duration_ms": durations[-1],
            "total_input_bytes": group["total_input_bytes"],
            "fallback_count": fallback_count,
            "fallback_rate": fallback_rate,
            "candidate_score": total_duration_ms * (1.0 + fallback_rate),
        }
        if service is not None:
            summary["service"] = service
        if group["input_sample_count"]:
            summary["avg_input_bytes"] = (
                group["total_input_bytes"] / group["input_sample_count"]
            )
        summaries.append(summary)

    return sorted(
        summaries,
        key=lambda item: (
            -item["candidate_score"],
            -item["total_duration_ms"],
            -item["sample_count"],
            item["operation"],
        ),
    )


def sha256_batch(items: list[bytes]) -> list[str]:
    return [hashlib.sha256(item).hexdigest() for item in items]


def verify_hash_chain(
    events: list[dict[str, Any]],
    hash_field: str = "entry_hash",
    prev_field: str = "prev_hash",
    payload_fields: list[str] | None = None,
    seed: str = "",
) -> dict[str, Any]:
    previous = seed
    for index, event in enumerate(events):
        if event.get(prev_field) != previous:
            return {
                "valid": False,
                "total_events": len(events),
                "verified_events": index,
                "mismatch_index": index,
                "expected_prev_hash": previous,
                "actual_prev_hash": event.get(prev_field),
                "error": f"{prev_field} mismatch",
            }
        if payload_fields is None:
            payload = {k: v for k, v in event.items() if k != hash_field}
        else:
            payload = {k: event[k] for k in payload_fields if k in event}
        encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode()
        expected = hashlib.sha256(encoded).hexdigest()
        if event.get(hash_field) != expected:
            return {
                "valid": False,
                "total_events": len(events),
                "verified_events": index,
                "mismatch_index": index,
                "expected_hash": expected,
                "actual_hash": event.get(hash_field),
                "error": f"{hash_field} mismatch",
            }
        previous = expected
    return {"valid": True, "total_events": len(events), "verified_events": len(events)}


def html_to_text(html: str, preserve_links: bool = False) -> str:
    text = re.sub(r"(?is)<(script|style|noscript)\b[^>]*>.*?</\1>", "", html)
    if preserve_links:
        text = re.sub(
            r"(?is)<a\b[^>]*href=[\"']?([^\"'\s>]+)[^>]*>(.*?)</a>",
            lambda match: f"{html_to_text(match.group(2))} ({html_lib.unescape(match.group(1))})",
            text,
        )
    text = re.sub(r"(?i)<br\s*/?>", "\n", text)
    text = re.sub(r"(?i)</?(p|div|h[1-6]|li|tr|table|section|article|body)\b[^>]*>", "\n\n", text)
    text = re.sub(r"(?s)<[^>]+>", "", text)
    lines = [" ".join(line.split()) for line in html_lib.unescape(text).splitlines()]
    paragraphs = [line for line in lines if line]
    return "\n\n".join(paragraphs)


def detect_encoding(data: bytes, sample_bytes: int = 8192) -> dict[str, Any]:
    sample = data[:sample_bytes]
    if sample.startswith(b"\xef\xbb\xbf"):
        return {"encoding": "utf-8", "confidence": 1.0, "bom": True}
    if all(byte <= 0x7F for byte in sample):
        return {"encoding": "ascii", "confidence": 1.0, "bom": False}
    try:
        sample.decode("utf-8")
        return {"encoding": "utf-8", "confidence": 0.99, "bom": False}
    except UnicodeDecodeError:
        pass
    if any(byte in sample for byte in (0x91, 0x92, 0x93, 0x94, 0x96, 0x97)):
        return {"encoding": "windows-1252", "confidence": 0.75, "bom": False}
    return {"encoding": "iso-8859-1", "confidence": 0.55, "bom": False}


try:
    import tomllib as _tomllib
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11 without tomli.
    _tomllib = None


def parse_toml(toml_str: str) -> dict[str, Any]:
    if _tomllib is None:
        raise RuntimeError("TOML fallback parsing requires Python 3.11+")
    return _tomllib.loads(toml_str)


def validate_toml(toml_str: str) -> bool:
    parse_toml(toml_str)
    return True


def json_to_toml(obj: dict[str, Any]) -> str:
    lines = []
    for key, value in obj.items():
        lines.append(f"{key} = {json.dumps(value)}")
    return "\n".join(lines)


def merge_toml(base: str, overlay: str) -> dict[str, Any]:
    merged = parse_toml(base)
    merged.update(parse_toml(overlay))
    return merged


def parse_toon(toon_str: str) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for line in toon_str.splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            result[key.strip()] = value.strip()
    return result


def to_toon(obj: dict[str, Any]) -> str:
    return "\n".join(f"{key}: {value}" for key, value in obj.items())


def validate_toon(toon_str: str) -> bool:
    parse_toon(toon_str)
    return True


def json_to_toon(obj: dict[str, Any]) -> str:
    return to_toon(obj)


def _chunk_dict(content: str, chunk_type: str, file_path: str | None = None) -> dict[str, Any]:
    return {
        "id": "",
        "content": content,
        "kind": chunk_type,
        "chunk_type": chunk_type,
        "language": None,
        "file_path": file_path,
        "start_line": 1,
        "end_line": max(1, content.count("\n") + 1),
        "start_column": 0,
        "end_column": 0,
        "token_count": count_tokens(content),
        "name": None,
        "parent_name": None,
        "signature": None,
        "docstring": None,
        "imports": [],
        "parent_type": None,
        "symbol_kind": None,
        "container_name": None,
        "container_kind": None,
        "symbol_path": None,
        "calls": [],
        "definitions": [],
    }


def extract_calls(language: str, content: str) -> list[str]:
    del language
    return re.findall(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(", content)


def chunk_code(
    content: str,
    language: str,
    max_tokens: int = 512,
    overlap_tokens: int = 50,
    file_path: str | None = None,
    model: str | None = None,
) -> list[dict[str, Any]]:
    del max_tokens, overlap_tokens, model
    chunk = _chunk_dict(content, "code", file_path)
    chunk["language"] = language
    chunk["calls"] = extract_calls(language, content)
    return [chunk]


def chunk_markdown(
    content: str,
    max_tokens: int = 512,
    overlap_tokens: int = 50,
    file_path: str | None = None,
    model: str | None = None,
) -> list[dict[str, Any]]:
    del max_tokens, overlap_tokens, model
    return [_chunk_dict(content, "markdown", file_path)]


def chunk_text(
    content: str,
    max_tokens: int = 512,
    overlap_tokens: int = 50,
    file_path: str | None = None,
    model: str | None = None,
) -> list[dict[str, Any]]:
    del max_tokens, overlap_tokens, model
    return [_chunk_dict(content, "text", file_path)]


def detect_content_type(file_path: str) -> str:
    lower = file_path.lower()
    if lower.endswith((".md", ".markdown")):
        return "markdown"
    if lower.endswith((".py", ".go", ".rs", ".js", ".ts", ".tsx", ".jsx")):
        return "code"
    return "text"


def chunk_auto(
    content: str,
    file_path: str,
    max_tokens: int = 512,
    overlap_tokens: int = 50,
    model: str | None = None,
) -> list[dict[str, Any]]:
    kind = detect_content_type(file_path)
    if kind == "markdown":
        return chunk_markdown(content, max_tokens, overlap_tokens, file_path, model)
    if kind == "code":
        language = file_path.rsplit(".", 1)[-1]
        return chunk_code(content, language, max_tokens, overlap_tokens, file_path, model)
    return chunk_text(content, max_tokens, overlap_tokens, file_path, model)


def chunk_batch(
    files: list[tuple[str, str]],
    max_tokens: int = 512,
    overlap_tokens: int = 50,
    model: str | None = None,
) -> list[dict[str, Any]]:
    chunks: list[dict[str, Any]] = []
    for file_path, content in files:
        chunks.extend(chunk_auto(content, file_path, max_tokens, overlap_tokens, model))
    return chunks


class MultiPatternMatcher:
    def __init__(self, patterns: list[str], regex: bool = False) -> None:
        self.patterns = patterns
        self.regex = regex
        self._compiled = [re.compile(pattern) for pattern in patterns] if regex else []

    def find_all(self, text: str) -> list[tuple[int, int, int]]:
        matches: list[tuple[int, int, int]] = []
        if self.regex:
            for index, pattern in enumerate(self._compiled):
                matches.extend((index, m.start(), m.end()) for m in pattern.finditer(text))
        else:
            for index, pattern in enumerate(self.patterns):
                start = text.find(pattern)
                while pattern and start != -1:
                    matches.append((index, start, start + len(pattern)))
                    start = text.find(pattern, start + 1)
        return sorted(matches, key=lambda item: (item[1], item[2], item[0]))


def parse_eml(data: bytes) -> dict[str, Any]:
    message = email.message_from_bytes(data)
    headers = {key.lower(): value for key, value in message.items()}
    body_text = ""
    body_html = None
    attachments = []
    for part in message.walk():
        if part.is_multipart():
            continue
        payload = part.get_payload(decode=True) or b""
        content_type = part.get_content_type()
        filename = part.get_filename()
        if filename or part.get_content_disposition() == "attachment":
            attachments.append(
                {
                    "filename": filename,
                    "content_type": content_type,
                    "sha256": hashlib.sha256(payload).hexdigest(),
                    "size": len(payload),
                }
            )
        elif content_type == "text/plain" and not body_text:
            body_text = payload.decode(part.get_content_charset() or "utf-8", errors="replace").strip()
        elif content_type == "text/html" and body_html is None:
            body_html = payload.decode(part.get_content_charset() or "utf-8", errors="replace").strip()
    return {"headers": headers, "body_text": body_text, "body_html": body_html, "attachments": attachments}


def parse_mbox(data: bytes) -> list[dict[str, Any]]:
    chunks = re.split(br"(?m)^From .*$", data)
    return [parse_eml(chunk.strip()) for chunk in chunks if chunk.strip()]


def capabilities() -> dict[str, bool]:
    return {
        "tokenizer": False,
        "simd_json": False,
        "transport": True,
        "eventlog": True,
        "accel_scout": True,
        "ranking": True,
        "hashing": True,
        "text": True,
        "encoding": True,
        "chunker_code": True,
        "chunker_markdown": True,
        "chunker_text": True,
        "pattern_matcher": True,
        "mail_parser": True,
        "ffi_available": False,
        "fallback": True,
    }


__all__ = [
    "count_tokens",
    "truncate_to_tokens",
    "split_by_tokens",
    "Tokenizer",
    "normalize_l2",
    "cosine_similarity",
    "cosine_similarity_matrix",
    "top_k_similar",
    "compute_centroid",
    "prepare_embedding_batch",
    "hybrid_rank",
    "rrf_fuse",
    "FuzzyMatcher",
    "parse_unified_diff",
    "LogReader",
    "parse_ndjson",
    "NDJSONReader",
    "parse_jsonrpc_batch",
    "parse_jsonrpc_lines",
    "serialize_jsonrpc_lines",
    "parse_sse_events",
    "extract_mcp_http_meta",
    "project_eventlog",
    "summarize_eventlog",
    "summarize_accel_observations",
    "sha256_batch",
    "verify_hash_chain",
    "html_to_text",
    "detect_encoding",
    "parse_toml",
    "validate_toml",
    "json_to_toml",
    "merge_toml",
    "parse_toon",
    "to_toon",
    "validate_toon",
    "json_to_toon",
    "extract_calls",
    "chunk_code",
    "chunk_markdown",
    "chunk_text",
    "chunk_auto",
    "chunk_batch",
    "detect_content_type",
    "MultiPatternMatcher",
    "parse_eml",
    "parse_mbox",
    "capabilities",
]
