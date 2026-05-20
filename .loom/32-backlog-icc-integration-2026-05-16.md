# Backlog — Utilities requested by the ICC integration (2026-05-16)

Sourced from the Integration Command Center (ICC) ↔ fi-accel integration spec at [`services/project-management/.loom/36-fi-accel-integration.md`](../../../services/project-management/.loom/36-fi-accel-integration.md). Each ticket is scoped tightly enough to land as one MR.

These are **additions to the implementation plan** ([30-implementation-plan.md](30-implementation-plan.md)). They sit roughly at M4-M5 in that plan's milestone ladder but are scoped from a real downstream consumer rather than from internal roadmap hypothesis. ICC is the first downstream adopter referenced in M5 of the plan.

Priority key: **P1** = unblocks a near-term ICC slice (M51-M52). **P2** = clear win, no immediate blocker. **P3** = nice-to-have, file for record.

---

## FA-ICC-01 — Batch SHA-256 hashing (P2)

**Surface:** `fi_accel.sha256_batch(items: list[bytes]) -> list[str]` (hex digests) and `fi_accel.sha256_stream(reader: BinaryIO, chunk_size: int = 64*1024) -> str`.

**Why:** ICC's `store.py:verify_audit_chain` walks a hash-linked audit chain that can grow to 100K+ entries (`services/project-management/src/integration_command_center/store.py:873-920`). Each entry is `sha256(prev_hash || serialized_event)`. Today this is a Python loop calling `hashlib.sha256` per entry. Evidence-vault backfill has the same shape for file-content hashing.

**Why fi-accel rather than stdlib:** `hashlib.sha256` already lands in OpenSSL, so the per-call speedup is small. The win is amortizing Python-loop overhead and (optionally) running multi-block hashing in Rust with explicit SIMD on AVX2/SHA-NI hosts. Order-of-magnitude target: 3-5x on 10K small entries; near-1x on a single large blob.

**Acceptance:**
- `sha256_batch([b"a", b"b", b"c"])` returns same hex digests as `[hashlib.sha256(x).hexdigest() for x in ...]`.
- Benchmark: 100K × 256-byte entries ≥ 3x faster than the Python loop.
- Streaming form works without loading the whole file.

**Estimate:** S (one Rust file wrapping `sha2` crate, PyO3 wrapper, parity test).

---

## FA-ICC-02 — HTML → plain text extraction (P2)

**Surface:** `fi_accel.html_to_text(html: str, *, preserve_links: bool = False) -> str`.

**Why:** ICC's email parser (`services/project-management/src/integration_command_center/parsers/email.py`) encounters `text/html` MIME parts and currently has no fallback better than handing the raw HTML to FTS. This degrades search quality (FTS indexes `<p>` and `&nbsp;` as tokens). The same need will hit a future HTML-clipper for Confluence / Jira descriptions.

**Why fi-accel rather than `html.parser`:** stdlib's `html.parser` is a SAX-style callback API — you write the visitor yourself. A library-grade plain-text extractor (block-vs-inline aware, entity-decoded, whitespace-normalized) is a small Rust port of the well-known algorithm and matches fi-accel's "high-perf utility belt" charter. Bonus: deterministic output for downstream embedding/FTS.

**Acceptance:**
- `html_to_text("<p>Hello <b>world</b>.</p>")` → `"Hello world."`.
- Entities decoded: `&amp;` → `&`, `&#x27;` → `'`.
- Block-level elements produce paragraph breaks; inline elements do not.
- `<script>` and `<style>` contents are dropped, not emitted.
- `preserve_links=True` emits `text (url)` for `<a href="url">text</a>`.

**Estimate:** M (existing crates like `html2text` or `scraper` can underpin it; need shape/normalization passes).

---

## FA-ICC-03 — Hybrid BM25 + vector rerank helper (P1, target M51)

**Surface:** `fi_accel.hybrid_rank(lex_ranks: list[int], cosine_scores: list[float], *, alpha: float = 0.5, method: Literal["weighted_sum", "rrf"] = "weighted_sum") -> list[float]` returning a `final_score` per candidate.

**Why:** ICC's `search_hybrid.hybrid_search` (`services/project-management/src/integration_command_center/synthesizers/search_hybrid.py:219-256`) hand-rolls weighted-sum fusion with reciprocal-rank normalization on the lexical side. The math is identical across every hybrid-search consumer in the workspace. Centralizing it in fi-accel removes ~30 lines of fiddly code from each adopter and gives one canonical place to add RRF, learned-to-rank, or alpha-tuning later.

**Why fi-accel:** matches the "embedding utility belt" charter; SIMD optional but not required (vector size is tiny — at most a few hundred candidates).

**Acceptance:**
- `weighted_sum`: `final[i] = alpha * (1 / (1 + lex_ranks[i])) + (1 - alpha) * cosine_scores[i]`.
- `rrf`: standard `1 / (k + lex_rank)` with `k=60` default.
- `alpha` validated to `[0, 1]`; `cosine_scores[i]==0` collapses to lexical-only contribution.
- Lengths must match; raise `PyValueError` otherwise.
- Property test: ordering is invariant to permutations of identical-score inputs (stable sort contract).

**Estimate:** S (pure-math; ~50 LoC Rust, 30 LoC PyO3).

---

## FA-ICC-04 — Gemma tokenizer support in `count_tokens` / `truncate_to_tokens` (P1, target M51)

**Surface:** Add `model="gemma"`, `model="gemma-26b"`, `model="gemma-26b-a4b"` to the existing tokenizer enum.

**Why:** ICC's LLM extractor (`services/project-management/src/integration_command_center/extractors/llm_qwen.py`, despite the legacy filename it now points at the **warm Gemma 26B-A4B lane** per the 2026-05-13 docstring update) ships artifact bodies to the model without token-budget enforcement. fi-accel's tokenizer currently supports `gpt-*`, `claude-3-*`, and embedding models, but **not Gemma**. ICC will use `gpt-4` as a conservative upper-bound estimator in M51 — this ticket replaces that workaround with an accurate count.

**Why fi-accel:** the tokenizer module already exists; adding one model family is the obvious next slice.

**Acceptance:**
- `count_tokens("hello world", model="gemma-26b")` returns a count that matches Gemma's SentencePiece tokenizer (±0 tokens on a 100-string fixture).
- `truncate_to_tokens(long_text, 1000, model="gemma-26b")` returns text whose `count_tokens(..., model="gemma-26b") <= 1000`.
- Existing GPT/Claude paths unchanged.
- README capability table updated.

**2026-05-20 status:** Gemma aliases now require the optional `gemma-tokenizer` feature and a local Hugging Face `tokenizer.json`; fi-accel does not approximate Gemma with GPT tokenizers. An opt-in Rust exactness fixture compares fi-accel counts against the local Hugging Face tokenizer on 100 strings when `FI_ACCEL_GEMMA_TOKENIZER_PATH` is set, and exits successfully without an asset when the variable is unset.

**Estimate:** M (depends on whether tiktoken-rs has the Gemma SentencePiece model bundled; if not, add the `sentencepiece` crate behind a feature flag and gate the binary-size hit).

---

## FA-ICC-05 — Charset / encoding detection (P2)

**Surface:** `fi_accel.detect_encoding(data: bytes, *, sample_bytes: int = 8192) -> tuple[str, float]` returning `(encoding_name, confidence)`.

**Why:** ICC's email and Slack-export parsers currently fall back to `bytes.decode("utf-8", errors="replace")` (e.g., `parsers/email.py`, `parsers/slack_export.py:320`). This corrupts content silently when the source is legacy Windows-1252 or ISO-8859-1. A `detect_encoding` utility would let the parser pick the right codec on the first try.

**Why fi-accel:** the `chardetng` crate is a high-quality Mozilla-derived detector with a small footprint. Drop-in.

**Acceptance:**
- `detect_encoding(windows_1252_bytes)` → `("windows-1252", confidence >= 0.7)`.
- `detect_encoding(b"plain ascii")` → `("utf-8", 1.0)` or `("ascii", 1.0)`.
- Sample size capped at `sample_bytes` to keep the call O(1) on large payloads.

**Estimate:** S (PyO3 wrapper around `chardetng`).

---

## FA-ICC-06 — Streaming NDJSON reader (LogReader for JSON) (P2, target M50-series)

**Surface:** `fi_accel.NDJSONReader(initial_content: bytes = b"")` — a class mirroring `LogReader`'s incremental-append contract, but emitting parsed JSON objects per line. Methods: `append(bytes) -> list[dict]`, `__len__`, `tail(n) -> list[dict]`.

**Why:** ICC's transcript-display path (planned, see `store.py:254` `TRANSCRIPT_FORMATS = {"codex_jsonl", "claude_jsonl", "generic_jsonl"}`) will need to read NDJSON files that can run to 50MB+ on long sessions. Today `parse_ndjson(bytes) -> list[dict]` (existing) requires loading the whole file. A streaming reader avoids OOM and enables tail-following for live transcripts.

**Why fi-accel:** `LogReader` already exists with this exact incremental shape; this is a JSON-aware sibling.

**Acceptance:**
- `r = NDJSONReader(); r.append(b'{"a":1}\n{"b":2}\n')` returns `[{"a":1},{"b":2}]`.
- Partial trailing line (no `\n`) is buffered and emitted on the next `append`.
- Malformed line raises `PyValueError` with line number; reader stays usable for subsequent appends (skip-bad-line mode optional).
- `tail(n)` returns the last n parsed objects.

**Estimate:** M (line-buffer state machine; PyO3 class wrapping the existing NDJSON parser).

---

## FA-ICC-07 — SIMD regex / multi-pattern matcher (P3)

**Surface:** `fi_accel.MultiPatternMatcher(patterns: list[str])` — class with `find_all(text: str) -> list[tuple[int, int, int]]` returning `(pattern_index, start, end)` per match.

**Why:** ICC's entity linker (`synthesizers/entity_linker.py:39-176`) runs ~6 regex patterns (Jira keys, emails, URLs, vendor substrings) against every artifact body on extract. Python `re` is C-backed and fine at MVP scale, but a SIMD multi-pattern matcher (Hyperscan-style) would let us add 100+ patterns (per-vendor aliases, per-project codes) without per-pattern linear cost.

**Why fi-accel:** the `aho-corasick` and `regex-automata` crates make a small-footprint version straightforward; full Hyperscan integration is heavier and probably out of scope.

**Acceptance:**
- `MultiPatternMatcher(["foo", "bar"]).find_all("foo bar baz foo")` → `[(0,0,3),(1,4,7),(0,12,15)]`.
- Patterns can be literal substrings OR `regex`-crate regexes (declare via a constructor flag).
- One-pass scan; cost sub-linear in pattern count.

**Estimate:** M-L (API design needs care; defer until ICC actually feels pattern-count pressure).

---

## FA-ICC-08 — EML / mbox parser (P3)

**Surface:** `fi_accel.parse_eml(data: bytes) -> dict` returning `{headers: dict, body_text: str, body_html: str | None, attachments: list[{filename, content_type, sha256, size}]}`.

**Why:** ICC's `parsers/email.py` (207 LoC) hand-rolls MIME walking on top of `email.message_from_string`. It works but is a known pain point for multipart and base64-encoded HTML parts. A fi-accel parser using the `mailparse` Rust crate would replace it wholesale and give a stable shape across consumers.

**Why fi-accel:** matches the "document parsing utility belt" charter alongside the chunkers.

**Acceptance:**
- Parses a multipart/alternative message with both text/plain and text/html parts.
- Decodes quoted-printable and base64 transfer encodings.
- Attachments are reported with size and sha256, but content is NOT loaded into memory by default (returned as a handle / offset, or a `read_attachment(index)` method).
- mbox variant (`parse_mbox`) emits an iterator of parsed messages.

**Estimate:** L (parser surface is broad; pair with FA-ICC-02 HTML-to-text since both serve ICC's email path).

---

## FA-ICC-09 — Reciprocal rank fusion (RRF) standalone helper (P2)

**Surface:** `fi_accel.rrf_fuse(ranked_lists: list[list[Any]], *, k: int = 60) -> list[tuple[Any, float]]` returning a single fused ranking with RRF scores.

**Why:** Same motivation as FA-ICC-03 but the orthogonal use case — fusing **two independent candidate sets** (e.g., FTS hits ∪ semantic-only hits) into one ranking. ICC's `search_hybrid` docstring (`search_hybrid.py:32-37`) explicitly calls out that this is "deferred as a follow-up" and would unblock the "semantic-only matches the lexical-side missed" case.

**Why fi-accel:** RRF is the standard recipe; centralizing it avoids each consumer reinventing the `k=60` constant.

**Acceptance:**
- `rrf_fuse([["a","b","c"], ["b","a","d"]])` returns a list with "a" and "b" ahead of "c"/"d" (both appear in both lists).
- Items not in a given list contribute 0 to the score (not `1/(k+inf)`).
- `k` configurable, default 60 (the standard).

**Estimate:** S (pure-Python implementation is ~10 lines; the win is making it canonical and consumable from Rust+Go+WASM bindings too).

---

## FA-ICC-10 — Hash-linked event chain verifier (P3)

**Surface:** `fi_accel.verify_hash_chain(events: list[dict], *, hash_field: str = "entry_hash", prev_field: str = "prev_hash", payload_fields: list[str] | None = None) -> dict` returning `{valid: bool, first_invalid_index: int | None, expected: str | None, actual: str | None}`.

**Why:** ICC's `store.py:verify_audit_chain` (`store.py:873-920`) implements a generic linked-hash-list integrity check: each event has `entry_hash = sha256(prev_hash || serialized_payload)`. This pattern recurs in any audit/blockchain-lite design. Lifting it into fi-accel makes it reusable and lets us benchmark the SIMD-hash path (FA-ICC-01) in the same call.

**Why fi-accel:** a small algorithmic utility; pairs naturally with FA-ICC-01 (batch SHA-256). Could compose internally.

**Acceptance:**
- A valid chain returns `{valid: True, first_invalid_index: None, ...}`.
- A tampered entry returns `{valid: False, first_invalid_index: i, expected: <hex>, actual: <hex>}`.
- Empty chain returns `{valid: True, ...}`.
- Genesis entry's `prev_hash` is the empty string (or configurable seed).

**Estimate:** S (composes on FA-ICC-01 internally; minimal extra API surface).

---

## FA-ICC-11 — Pure-Python fallback shipped alongside the wheel (P2)

**Surface:** Add a `fi_accel.fallback` submodule that re-implements every public function in pure Python (no Rust). Importable independently: `from fi_accel.fallback import cosine_similarity_matrix`.

**Why:** Every downstream adopter (ICC included) writes a `try: import fi_accel / except ImportError: <hand-rolled fallback>` guard. If fi-accel shipped its own slow-but-correct fallback in the same package, adopters could drop the guard entirely and just import. The native wheel still installs when available; the fallback is only loaded when explicitly requested.

The bigger value is **parity testing**: adopters can run `assert fast(x) == fallback(x)` in their own test suites instead of writing the fallback themselves — exactly what ICC's `test_accel_parity` will do.

**Why fi-accel:** removes one entire category of bugs (adopters' hand-rolled fallbacks drift from the Rust behavior over time).

**Acceptance:**
- `from fi_accel.fallback import cosine_similarity_matrix` works in an environment with no native wheel installed (pure-Python install path).
- Each fallback returns bit-identical (within float tolerance for SIMD floats) results to its native counterpart.
- A parity test suite ships in fi-accel itself, runnable as `pytest fi_accel/tests/test_parity.py`.
- Packaging: the fallback is a `.py` source file; native wheel installs both.

**Estimate:** L (need a Python reimplementation of every promoted function; lots of small surface area). High leverage though — directly removes boilerplate from every adopter.

---

## Cross-cutting note on testing

If FA-ICC-11 lands first, FA-ICC-01 through FA-ICC-10 each get a "free" parity test (run the native impl against the fallback impl). Suggest sequencing: FA-ICC-11 → FA-ICC-03/09 (small algorithmic) → FA-ICC-04 (unblocks ICC M51) → FA-ICC-01/02 (batch hashing + html-to-text) → the rest as need surfaces.

ICC's M51 needs only FA-ICC-04 to upgrade from "conservative upper-bound estimator" to "accurate count." Everything else is M52+ or quality-of-life.

## References

- ICC integration spec: [`services/project-management/.loom/36-fi-accel-integration.md`](../../../services/project-management/.loom/36-fi-accel-integration.md)
- fi-accel implementation plan: [30-implementation-plan.md](30-implementation-plan.md)
- fi-accel README capability matrix: [README.md](../README.md)
