# AGENTS.md - vycorporation/attribute-styling-rs

Read this file before changing attribute, filtering, classification, ramp,
visual-channel, resolved-plan, or consumer-boundary behavior.

<!-- vy-agentic-change-workflow:v1:start -->
## Agentic change workflow

This repository follows version 1 of the Vy agentic change workflow.

- Read the root and nearest applicable `AGENTS.md` files and authoritative contract documents before changing files.
- Bind substantive or decision-bearing work to a GitHub issue or another durable reviewed objective before implementation.
- Use a short-lived `codex/` branch and a pull request for repository changes; preserve any stricter repository-specific Git or review rules.
- Use conventional commits and include an explanatory commit body for every non-documentation change.
- Run the repository's declared validation before merge and report the exact commands and results.
- Obtain explicit authority for external writes and destructive operations, resolve exact targets first, and preserve unrelated user work.
- Treat Beast as a placement requirement: run `hostname -s` before SSH and never open a nested SSH connection when already on Beast.
- Complete work only after delivery, verification, review, merge, and an explicit handoff of any remaining blockers.
- Keep automatic deletion of merged remote branches separate from guarded cleanup of local branches and worktrees.

Repository-specific instructions may strengthen or specialize this block but may not weaken its safety or verification requirements.
<!-- vy-agentic-change-workflow:v1:end -->

## Repository role

`attribute-styling` is the reusable, renderer-neutral Rust library for
deterministic attribute styling. Keep it independent of vectorization, geometry
I/O, table engines, GUI runtimes, and renderers.

### Vectorizer product naming

**ARIES Vectorizer** is Vy's patented C++ CLI application. Always use its full
proper name. **vectorizer-rs** is Vy's separate work-in-progress Rust CLI
application; always write it as `vectorizer-rs`. Both are Vy applications.
Never shorten either product to "the vectorizer," "our vectorizer," "native
vectorizer," "native ARIES," or similar shorthand. When comparing them, name
**ARIES Vectorizer** and `vectorizer-rs` explicitly. Use lowercase
"vectorizer" only for the generic software category.

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
- Keep resolved-plan fields private and expose read-only accessors.
- Bound class counts before allocating class or legend collections.
- Do not use unsafe Rust.

## Dependency policy

Do not expose Arrow, Parquet, DataFusion, Rerun, image, wgpu, vectorizer,
spatial-io, QGIS, ArcGIS, DuckDB, or Sedona types in the public API.
Third-party palette types must remain private.

Prefer small, maintained Rust dependencies. Review license and maintenance
evidence before adding one.

`colorous` 1.0.16 is the private Apache-2.0 implementation behind the initial
Viridis ramp. Do not expose its types.

## Consumer boundaries

- `vectorizer-rs` retains canonical cubic output, `preview.png`, and its
  artifact contracts. Its implemented `render` subcommand consumes this crate
  and publishes separate styling output; consult its `docs/attribute-rendering.md`.
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
