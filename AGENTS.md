# AGENTS.md - vycorporation/attribute-styling-rs

Read this file before changing attribute, filtering, classification, ramp,
visual-channel, resolved-plan, or consumer-boundary behavior.

## Repository role

`attribute-styling` is the reusable, renderer-neutral Rust library for
deterministic attribute styling. Keep it independent of vectorization, geometry
I/O, table engines, GUI runtimes, and renderers.

## Contract rules

- Keep public types crate-owned.
- Support null, Boolean, signed and unsigned integer, finite float, and UTF-8
  text attributes.
- Reject NaN and infinity rather than silently assigning them.
- Define nulls, ties, ordering, boundary inclusivity, requested/effective class
  count, and empty-input behavior for every classifier.
- Preserve deterministic feature, class, and legend order.
- Keep filter expression parsing outside the crate until a shared grammar is
  separately approved.
- Keep ramp kind and visual-channel meaning explicit.
- Return immutable plans; never render inside the core crate.
- Do not use unsafe Rust.

## Dependency policy

Do not expose Arrow, Parquet, DataFusion, Rerun, image, wgpu, vectorizer,
spatial-io, QGIS, ArcGIS, DuckDB, or Sedona types in the public API.
Third-party palette types must remain private.

Prefer small, maintained Rust dependencies. Review license and maintenance
evidence before adding one.

## Consumer boundaries

- `vectorizer-rs` retains canonical cubic output, `preview.png`, and artifact
  v5. Styling output is separate.
- `spatial-io-rs` retains geometry conversion, coordinates, CRS, and formats.
- Rerun retains graph, UI, interaction, and rendering behavior.
- Consumers own translation to and from `attribute-styling` types.

## Issue and GitHub workflow

Use `codex/` branch names for Codex-authored changes. Perform repository and
GitHub work as `vy-matt-davis`.

Treat issue acceptance checkboxes as the execution ledger: check only
evidence-backed criteria, re-fetch before PR-ready/merge/close, and never merge
or close while applicable criteria remain unchecked.

## Validation

Before reporting code complete, run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
git diff --check
```

Do not claim QGIS, ArcGIS, DuckDB, Sedona, or renderer parity without
fixture-backed independent evidence.
