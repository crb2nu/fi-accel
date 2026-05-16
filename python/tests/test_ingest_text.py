import pytest

from fi_accel import NDJSONReader, detect_encoding, html_to_text


def test_html_to_text_preserves_links_and_decodes_entities():
    html = """
    <style>body { color: red; }</style>
    <h1>Title &amp; More</h1>
    <p>Read <a href="https://example.test?a=1&amp;b=2"><span>the docs</span></a>.</p>
    <script>alert(1)</script>
    """

    assert (
        html_to_text(html, preserve_links=True)
        == "Title & More\n\nRead the docs (https://example.test?a=1&b=2)."
    )
    assert html_to_text(html) == "Title & More\n\nRead the docs."


def test_detect_encoding_reports_ascii_utf8_and_windows_1252():
    assert detect_encoding(b"plain text")["encoding"] == "ascii"

    utf8 = detect_encoding("snowman: \u2603".encode())
    assert utf8["encoding"] == "utf-8"
    assert utf8["confidence"] > 0.9

    cp1252 = detect_encoding(b"\x93quoted\x94")
    assert cp1252["encoding"] == "windows-1252"
    assert cp1252["confidence"] >= 0.7


def test_ndjson_reader_buffers_partial_lines_and_recovers_after_error():
    reader = NDJSONReader(initial_content=b'{"a": 1}\n{"partial"')
    assert len(reader) == 1
    assert reader.buffered_len() > 0

    assert reader.append(b': true}\n') == [{"partial": True}]
    assert reader.tail(2) == [{"a": 1}, {"partial": True}]

    with pytest.raises(ValueError, match="line 3"):
        reader.append(b"not json\n")

    assert reader.append(b'{"after": "ok"}\n') == [{"after": "ok"}]
    assert len(reader) == 3
