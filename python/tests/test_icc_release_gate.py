import hashlib
import importlib
import io
import json
import math

import pytest

import fi_accel
from fi_accel import fallback


NATIVE_AVAILABLE = not fi_accel.capabilities().get("fallback", False)
NATIVE = importlib.import_module("fi_accel.fi_accel") if NATIVE_AVAILABLE else fi_accel


def assert_scores_close(left, right):
    assert len(left) == len(right)
    for (left_item, left_score), (right_item, right_score) in zip(left, right):
        assert left_item == right_item
        assert math.isclose(left_score, right_score, rel_tol=1e-6, abs_tol=1e-6)


def canonical_sha256(value):
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(payload).hexdigest()


def build_hash_chain():
    events = [
        {"prev_hash": "", "event_type": "created", "payload": {"count": 1}},
        {"event_type": "updated", "payload": {"count": 2}},
    ]
    events[0]["entry_hash"] = canonical_sha256(events[0])
    events[1]["prev_hash"] = events[0]["entry_hash"]
    events[1]["entry_hash"] = canonical_sha256(events[1])
    return events


def build_service_hash_chain():
    records = [
        {
            "event_id": "evt-2026-05-23-0001",
            "timestamp": "2026-05-23T17:00:00Z",
            "tenant": "flexinfer",
            "project_slug": "icc-project-workspaces",
            "actor": {"type": "agent", "id": "codex"},
            "operation": "capture.created",
            "resource": {"type": "capture", "id": "cap-001"},
            "payload": {
                "source": "email",
                "path": "projects/fi-accel/email/cap-001.md",
                "labels": ["release-gate", "hash-chain"],
            },
            "metadata": {"request_id": "req-001", "fallback": False},
        },
        {
            "event_id": "evt-2026-05-23-0002",
            "timestamp": "2026-05-23T17:03:12Z",
            "tenant": "flexinfer",
            "project_slug": "icc-project-workspaces",
            "actor": {"type": "agent", "id": "codex"},
            "operation": "capture.indexed",
            "resource": {"type": "capture", "id": "cap-001"},
            "payload": {
                "chunk_count": 7,
                "embedding_model": "text-embedding-3-small",
                "artifact_sha256": "9f7f4a934aa3c5684f1a0f77870d2fbb1f726c8ec0e0873dd3277197f45d4b2b",
            },
            "metadata": {"request_id": "req-002", "fallback": False},
        },
        {
            "event_id": "evt-2026-05-23-0003",
            "timestamp": "2026-05-23T17:06:29Z",
            "tenant": "flexinfer",
            "project_slug": "icc-project-workspaces",
            "actor": {"type": "service", "id": "icc-api"},
            "operation": "capture.linked",
            "resource": {"type": "capture", "id": "cap-001"},
            "payload": {
                "linked_issue": "FA-PLAN-04",
                "status": "verified",
                "references": [
                    "docs/benchmark-ledger.md",
                    ".loom/45-planning-audit-and-backlog-2026-05-20.md",
                ],
            },
            "metadata": {"request_id": "req-003", "fallback": False},
        },
    ]

    previous = ""
    events = []
    for record in records:
        event = {"prev_hash": previous, **record}
        event["entry_hash"] = canonical_sha256(event)
        previous = event["entry_hash"]
        events.append(event)
    return events


def multipart_email():
    return b"""Subject: Release gate
From: codex@example.test
Content-Type: multipart/mixed; boundary="BOUNDARY"

--BOUNDARY
Content-Type: text/plain; charset=utf-8

Hello ICC
--BOUNDARY
Content-Type: text/html; charset=utf-8

<p>Hello <b>ICC</b></p>
--BOUNDARY
Content-Type: text/plain; name="notes.txt"
Content-Disposition: attachment; filename="notes.txt"

attached note
--BOUNDARY--
"""


def test_fallback_submodule_exports_icc_release_helpers():
    expected = {
        "count_tokens_batch",
        "sha256_batch",
        "sha256_stream",
        "verify_hash_chain",
        "html_to_text",
        "detect_encoding",
        "NDJSONReader",
        "hybrid_rank",
        "rrf_fuse",
        "MultiPatternMatcher",
        "parse_eml",
        "parse_mbox",
    }

    assert expected.issubset(set(fallback.__all__))
    for name in expected:
        assert getattr(fallback, name)


def test_icc_hashing_native_matches_fallback():
    items = [b"", b"hello", b"audit" * 64]
    stream_payload = b"chunked payload" * 32
    events = build_hash_chain()

    assert NATIVE.sha256_batch(items) == fallback.sha256_batch(items)
    assert fallback.sha256_stream(io.BytesIO(stream_payload), chunk_size=13) == hashlib.sha256(
        stream_payload
    ).hexdigest()
    assert fi_accel.sha256_stream(io.BytesIO(stream_payload), chunk_size=13) == fallback.sha256_stream(
        io.BytesIO(stream_payload), chunk_size=13
    )
    assert NATIVE.verify_hash_chain(events) == fallback.verify_hash_chain(events)


def test_token_count_batch_matches_scalar_api():
    texts = ["hello world", "def f():\n    return 1\n", ""]

    assert NATIVE.count_tokens_batch(texts, model="gpt-4") == [
        NATIVE.count_tokens(text, model="gpt-4") for text in texts
    ]

    tokenizer = NATIVE.Tokenizer("gpt-4")
    assert tokenizer.count_batch(texts) == [tokenizer.count(text) for text in texts]


def test_icc_hash_chain_service_fixture_native_matches_fallback():
    events = build_service_hash_chain()

    native = NATIVE.verify_hash_chain(events)
    slow = fallback.verify_hash_chain(events)

    assert native == slow
    assert native["valid"] is True
    assert native["total_events"] == native["verified_events"] == 3


def test_icc_hash_chain_service_fixture_rejects_tampering():
    events = build_service_hash_chain()
    tampered = json.loads(json.dumps(events))
    tampered[1]["payload"]["chunk_count"] = 8

    native = NATIVE.verify_hash_chain(tampered)
    slow = fallback.verify_hash_chain(tampered)

    assert native == slow
    assert native["valid"] is False
    assert native["mismatch_index"] == 1
    assert native["error"] == "entry_hash mismatch"
    assert native["actual_hash"] == events[1]["entry_hash"]

    broken_link = json.loads(json.dumps(events))
    broken_link[2]["prev_hash"] = "0" * 64

    native = NATIVE.verify_hash_chain(broken_link)
    slow = fallback.verify_hash_chain(broken_link)

    assert native == slow
    assert native["valid"] is False
    assert native["mismatch_index"] == 2
    assert native["error"] == "prev_hash mismatch"
    assert native["expected_prev_hash"] == events[1]["entry_hash"]
    assert native["actual_prev_hash"] == "0" * 64


def test_icc_text_encoding_and_ndjson_native_matches_fallback():
    html = "<h1>Title &amp; More</h1><p>Read <a href='https://example.test'>docs</a>.</p>"
    assert NATIVE.html_to_text(html, preserve_links=True) == fallback.html_to_text(
        html, preserve_links=True
    )
    native_encoding = NATIVE.detect_encoding(b"\x93quoted\x94")
    fallback_encoding = fallback.detect_encoding(b"\x93quoted\x94")
    assert native_encoding["encoding"] == fallback_encoding["encoding"] == "windows-1252"
    assert native_encoding["confidence"] >= 0.7
    assert fallback_encoding["confidence"] >= 0.7

    native_reader = NATIVE.NDJSONReader(b'{"a": 1}\n{"partial"')
    fallback_reader = fallback.NDJSONReader(b'{"a": 1}\n{"partial"')
    assert len(native_reader) == len(fallback_reader) == 1
    assert native_reader.append(b': true}\n') == fallback_reader.append(b': true}\n')
    with pytest.raises(ValueError, match="line 3"):
        native_reader.append(b"not json\n")
    with pytest.raises(ValueError, match="line 3"):
        fallback_reader.append(b"not json\n")
    assert native_reader.append(b'{"after": "ok"}\n') == fallback_reader.append(
        b'{"after": "ok"}\n'
    )


def test_icc_ranking_native_matches_fallback():
    assert_scores_close(
        NATIVE.hybrid_rank([0, 2, 1], [0.2, 0.95, 0.5], alpha=0.5),
        fallback.hybrid_rank([0, 2, 1], [0.2, 0.95, 0.5], alpha=0.5),
    )
    assert_scores_close(
        NATIVE.hybrid_rank([2, 1, 0], [0.7, 0.7, 0.2], method="rrf", k=60),
        fallback.hybrid_rank([2, 1, 0], [0.7, 0.7, 0.2], method="rrf", k=60),
    )
    assert_scores_close(
        NATIVE.rrf_fuse([[10, 20, 30], [20, 10]], k=60),
        fallback.rrf_fuse([[10, 20, 30], [20, 10]], k=60),
    )


def test_icc_pattern_matcher_native_matches_fallback():
    text = "See FA-123, email ops@example.test, and FA-456."

    native_literal = NATIVE.MultiPatternMatcher(["FA-", "example"])
    fallback_literal = fallback.MultiPatternMatcher(["FA-", "example"])
    assert native_literal.find_all(text) == fallback_literal.find_all(text)

    native_regex = NATIVE.MultiPatternMatcher([r"FA-\d+", r"[\w.]+@example\.test"], regex=True)
    fallback_regex = fallback.MultiPatternMatcher([r"FA-\d+", r"[\w.]+@example\.test"], regex=True)
    assert native_regex.find_all(text) == fallback_regex.find_all(text)


def test_icc_mail_parser_native_matches_fallback_shape():
    payload = multipart_email()

    native = NATIVE.parse_eml(payload)
    slow = fallback.parse_eml(payload)

    assert native["headers"]["subject"] == slow["headers"]["subject"] == "Release gate"
    assert native["body_text"] == slow["body_text"] == "Hello ICC"
    assert native["body_html"] == slow["body_html"] == "<p>Hello <b>ICC</b></p>"
    assert native["attachments"] == slow["attachments"]

    mbox_payload = b"From one@example.test Thu Jan 01 00:00:00 2026\n" + payload
    assert NATIVE.parse_mbox(mbox_payload) == fallback.parse_mbox(mbox_payload)


def test_gemma_exactness_release_gate_is_documented_as_opt_in():
    if NATIVE_AVAILABLE:
        with pytest.raises(Exception, match="Gemma tokenization requires|local Hugging Face"):
            NATIVE.count_tokens("hello", model="gemma-26b")
    assert fallback.Tokenizer("gemma-26b").count("hello") > 0
