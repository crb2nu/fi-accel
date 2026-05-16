import importlib
import math
import sys

from fi_accel import fallback


def test_fallback_imports_without_native_extension(monkeypatch):
    monkeypatch.setitem(sys.modules, "fi_accel.fi_accel", None)
    module = importlib.reload(importlib.import_module("fi_accel"))
    assert module.capabilities()["fallback"] is True


def test_embedding_helpers():
    vectors = fallback.normalize_l2([[3.0, 4.0], [0.0, 0.0]])
    assert math.isclose(sum(value * value for value in vectors[0]), 1.0)
    assert fallback.cosine_similarity([1.0, 0.0], [0.5, 0.5]) == 0.5
    assert fallback.cosine_similarity_matrix([[1.0]], [[2.0], [3.0]]) == [[2.0, 3.0]]
    assert fallback.compute_centroid([[1.0, 3.0], [3.0, 5.0]]) == [2.0, 4.0]


def test_json_and_log_helpers():
    assert fallback.parse_ndjson(b'{"a":1}\nnot json\n{"b":2}\n') == [
        {"a": 1},
        {"b": 2},
    ]
    reader = fallback.LogReader("a\nb\n")
    assert reader.append("c\n") == ["c"]
    assert reader.tail(2) == ["b", "c"]
    assert reader.search("c") == [(3, "c")]


def test_chunk_helpers_have_native_shape():
    chunks = fallback.chunk_auto("def f():\n    return g()\n", "x.py")
    assert chunks[0]["kind"] == "code"
    assert "g" in chunks[0]["calls"]


def test_tokenizer_roundtrip():
    tokenizer = fallback.Tokenizer("gemma-26b")
    encoded = tokenizer.encode("abc")
    assert tokenizer.decode(encoded) == "abc"
    assert tokenizer.count("hello world") > 0
