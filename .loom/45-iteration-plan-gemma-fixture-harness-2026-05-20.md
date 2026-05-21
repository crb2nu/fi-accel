# RALPH Iteration Plan

## Review

- Roadmap milestone: M5 release proof and ICC utility hardening.
- Spec section(s): `.loom/30-implementation-plan.md` active backlog FA-PLAN-05; `.loom/45-planning-audit-and-backlog-2026-05-20.md` ICC backlog FA-ICC-04.
- Prior decisions to preserve: Gemma aliases must not approximate with GPT tokenizers; Gemma assets are local-only and are not downloaded at runtime.

## Align

- Slice name: FA-PLAN-05 Gemma Fixture Harness.
- Scope in: opt-in exactness test for Gemma token counts, fixed 100-string fixture, README documentation for `FI_ACCEL_GEMMA_TOKENIZER_PATH`.
- Scope out: vendoring tokenizer assets, changing tokenizer runtime behavior, adding downstream ICC fixtures.
- Acceptance criteria:
  - Test confirms fi-accel Gemma token counts match a local Hugging Face tokenizer on a 100-string fixture.
  - Test passes without requiring the asset when `FI_ACCEL_GEMMA_TOKENIZER_PATH` is unset.
  - README documents the asset path and opt-in command.
- Dependencies/blockers: A local Hugging Face `tokenizer.json` is required only to exercise the exactness path.

## Land

- Planned file areas: `crates/fi-accel-core/src/tokenizer.rs`, `README.md`.
- Implementation steps:
  1. Add a feature-gated Gemma fixture exactness test.
  2. Compare fi-accel counts to the Hugging Face tokenizer loaded directly from the configured asset.
  3. Document the opt-in test command and skip behavior.

## Prove

- Tests to run:
  - `cargo test -p fi-accel-core tokenizer::tests::test_gemma_model_names_are_recognized`
  - `cargo test -p fi-accel-core --features gemma-tokenizer gemma_fixture`
  - `cargo test -p fi-accel-core`
- Lint/static checks: `cargo fmt --check`
- CI checks: Rust core test job should cover the default feature path; optional Gemma fixture remains environment-gated.

## Handoff/Harvest

- Docs to update: README Gemma tokenizer note.
- Agent-context entries to add: fixture harness decision/finding and next release-gate follow-up.
- Next-slice candidates: FA-PLAN-04 ICC release checklist or FA-PLAN-09 benchmark ledger.
