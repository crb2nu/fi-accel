# FlexDeck Surface Helpers

This helper set was shaped around three FlexDeck surfaces that currently do similar work in plain TypeScript:

## Logging

- FlexDeck files:
  - `web/src/lib/logUtils.ts`
  - `web/src/components/Logs/LogStream.tsx`
- `fi-accel` helpers:
  - `detect_log_level`
  - `analyze_log_lines`
  - WASM exports: `surface_detect_log_level`, `surface_analyze_logs`
- Intended use:
  - centralize log level heuristics
  - batch-compute `matches_filter` and `matches_search` for visual log renderers
  - keep regex fallback behavior consistent

## Topology / HoloDeck

- FlexDeck file:
  - `web/src/components/Dashboard/topology/layoutEngine.ts`
- `fi-accel` helpers:
  - `matches_label_selector`
  - `filter_label_selector_matches`
  - WASM exports: `surface_matches_label_selector`, `surface_filter_label_selector`
- Intended use:
  - reuse selector matching for service-to-pod relationships
  - support batched selector checks when topology graphs grow large

## Pipelines

- FlexDeck file:
  - `web/src/components/Pipeline/utils.ts`
- `fi-accel` helpers:
  - `normalize_ci_job_status`
  - `normalize_ci_pipeline_status`
  - WASM exports: `surface_normalize_ci_job_status`, `surface_normalize_ci_pipeline_status`
- Intended use:
  - collapse raw GitLab status tokens into a stable UI vocabulary
  - keep pipeline overview and detail surfaces aligned

## Notes

- These helpers are generic on purpose even though they were derived from FlexDeck.
- This round adds the core Rust API and WASM bindings first, which creates a low-risk adoption path for the frontend.
- A follow-up round can wire FlexDeck to the WASM package and remove duplicated TS logic.
