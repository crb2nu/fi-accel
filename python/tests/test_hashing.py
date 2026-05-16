import hashlib
import io
import json

from fi_accel import sha256_batch, sha256_stream, verify_hash_chain


def canonical_sha256(value):
    payload = json.dumps(value, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(payload).hexdigest()


def build_chain():
    first = {
        "prev_hash": "",
        "event_type": "created",
        "actor": "codex",
        "payload": {"count": 1},
    }
    first["entry_hash"] = canonical_sha256(first)

    second = {
        "prev_hash": first["entry_hash"],
        "event_type": "updated",
        "actor": "codex",
        "payload": {"count": 2},
    }
    second["entry_hash"] = canonical_sha256(second)
    return [first, second]


def test_sha256_batch_matches_hashlib():
    items = [b"", b"hello", b"agent audit log"]

    assert sha256_batch(items) == [hashlib.sha256(item).hexdigest() for item in items]


def test_sha256_stream_matches_hashlib():
    data = b"chunked audit payload" * 128

    assert sha256_stream(io.BytesIO(data), chunk_size=17) == hashlib.sha256(data).hexdigest()


def test_verify_hash_chain_accepts_valid_chain():
    result = verify_hash_chain(build_chain())

    assert result["valid"] is True
    assert result["total_events"] == 2
    assert result["verified_events"] == 2
    assert "mismatch_index" not in result


def test_verify_hash_chain_rejects_tampered_payload():
    events = build_chain()
    events[1]["payload"]["count"] = 3

    result = verify_hash_chain(events)

    assert result["valid"] is False
    assert result["mismatch_index"] == 1
    assert result["verified_events"] == 1
    assert result["error"] == "entry_hash mismatch"
    assert result["actual_hash"] == events[1]["entry_hash"]
    assert result["expected_hash"] != events[1]["entry_hash"]


def test_verify_hash_chain_accepts_empty_chain():
    result = verify_hash_chain([])

    assert result["valid"] is True
    assert result["total_events"] == 0
    assert result["verified_events"] == 0


def test_verify_hash_chain_supports_payload_fields_and_seed():
    event = {"prev_hash": "root", "payload": {"ok": True}, "ignored": "outside-hash"}
    event["entry_hash"] = canonical_sha256(
        {"prev_hash": event["prev_hash"], "payload": event["payload"]}
    )

    result = verify_hash_chain(
        [event],
        payload_fields=["prev_hash", "payload"],
        seed="root",
    )

    assert result["valid"] is True
