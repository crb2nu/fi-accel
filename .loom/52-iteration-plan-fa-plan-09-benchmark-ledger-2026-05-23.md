# RALPH Iteration Plan

## Review

- Roadmap milestone: M5 close the loop with benchmarks, docs, and one downstream adopter.
- Spec section(s): benchmark-backed promotion requirements in `.loom/20-product-spec.md` and `.loom/30-implementation-plan.md`.
- Prior decisions to preserve:
  - Performance claims must be benchmark-backed before release positioning.
  - Exact code-memory token counting is promoted only for the measured local `PythonIndexer` token path.
  - Full `fi_accel.chunk_code` default adoption remains a separate downstream contract decision.

## Align

- Slice name: FA-PLAN-09 Benchmark Ledger Claim Discipline.
- Scope in:
  - Remove stale unbacked speed multipliers from README and Python package prose.
  - Make README performance text point to dated ledger evidence.
  - Mark FA-PLAN-09 complete in planning artifacts.
- Scope out:
  - New benchmark runs.
  - New performance claims for tokenizer, embedding, diff, SSE, TOML, or TOON surfaces.
  - Downstream code-memory default flip.
- Acceptance criteria:
  - Ledger records the claim policy and remains the source of truth.
  - README performance claims link to the ledger or use conservative wording.
  - Package prose no longer advertises broad unbacked multipliers.
- Dependencies/blockers: none for the docs-only claim-discipline slice.

## Land

- Planned file areas: `README.md`, `python/fi_accel/__init__.py`, `docs/benchmark-ledger.md`, `.loom/`.
- Implementation steps:
  1. Replace legacy multiplier language with conservative API descriptions.
  2. Rewrite the README performance section around ledger-backed highlights.
  3. Update plan/backlog/decision artifacts and add a handoff note.

## Prove

- Tests to run: docs/prose checks with `rg` for stale multipliers and ledger references.
- Lint/static checks: not applicable for documentation-only changes.
- CI checks: not run locally for this slice because no runtime behavior changed.

## Handoff/Harvest

- Docs to update: README, benchmark ledger, implementation plan, planning audit, decisions.
- Agent-context entries to add: finding for FA-PLAN-09 completion.
- Next-slice candidates:
  - Land the downstream exact-token default flip in `py-codebase-memory`.
  - Decide FA-PLAN-08 WASM tokenizer/code-chunking bundle kill-test.
  - Add fresh ledger entries before promoting unmeasured performance claims.
