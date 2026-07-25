# attribute-styling-rs Bootstrap Design

**Status:** Proposed for written review  
**Date:** 2026-07-25  
**Repository:** `vycorporation/attribute-styling-rs`  
**Visibility:** Public

## Purpose

`attribute-styling-rs` will be the reusable Rust library for filtering records,
classifying attribute values, sampling color ramps, and resolving deterministic
visual styles. It will provide the shared behavior used by
`vycorporation/vectorizer-rs`, the `vycorporation/rerun` fork, and future Vy
tools without making any one renderer authoritative for the others.

The crate will not vectorize rasters and will not render pixels or GPU
primitives. It will turn typed attribute data plus a style specification into a
resolved style plan. Each consuming tool will adapt that plan to its own
renderer.

## Repository and Package Identity

- GitHub repository: `vycorporation/attribute-styling-rs`
- Cargo package: `attribute-styling`
- Rust crate path: `attribute_styling`
- License: dual MIT or Apache-2.0
- Rust edition: 2024
- Minimum supported Rust version: 1.89, matching the lower requirement of the
  initial consumers

The repository uses the `-rs` suffix to identify its implementation language.
The Cargo package omits the suffix so downstream imports remain concise.

## Architectural Seam

The library interface will accept:

1. typed attribute values exposed by a consumer adapter;
2. a validated filter and style specification; and
3. stable feature identities supplied by the caller.

It will return an immutable resolved plan containing:

- selected feature identities;
- filter outcomes;
- class assignments;
- class breaks and labels;
- resolved RGBA colors;
- resolved optional opacity, size, and stroke-width channels; and
- legend entries in deterministic order.

The public interface will not expose Arrow, Parquet, DataFusion, Rerun, image,
wgpu, or vectorizer-specific types. `vectorizer-rs` currently uses Arrow 54,
while Rerun uses Arrow 58.3 and DataFusion 53.1. Keeping those types behind
consumer adapters prevents dependency-version coupling and keeps the module
usable with ordinary Rust structs and future table engines.

## Attribute and Filter Model

The first durable scalar model will support:

- null;
- Boolean;
- signed and unsigned integers;
- finite floating-point values; and
- UTF-8 strings.

NaN and infinity will be rejected as numerical classification inputs rather
than silently assigned to a class. Filters may test nullness and may compare
compatible values with equality, ordering, set membership, and Boolean
composition.

The crate will own the validated filter expression tree and evaluation
semantics. Text parsing for a CLI expression is an adapter concern until two
consumers need the same textual grammar. This avoids freezing a query language
before the Rerun and command-line use cases are both understood.

## Classification Model

The planned classification families are:

- single style;
- categorical;
- equal interval;
- quantile;
- natural breaks (Jenks);
- pretty breaks;
- standard deviation;
- manual breaks; and
- continuous, unclassified interpolation.

Implementation will be incremental. The first implementation slice will
establish single, categorical, equal-interval, quantile, manual, and continuous
behavior. Jenks, pretty breaks, and standard deviation will follow with
independent reference fixtures and performance bounds.

`Quantile` is the contract name for the GIS mode sometimes called equal count.
Exact equality of class populations is not promised when tied values must
remain together. The resolved plan will record the effective class count and
breaks so callers never have to reconstruct the tie policy.

Every classifier will define:

- empty-input behavior;
- null handling;
- comparison and boundary inclusivity;
- tie behavior;
- requested versus effective class count;
- deterministic ordering; and
- failure behavior for incompatible values.

## Color Ramps and Visual Channels

The public model will own stable RGBA values and serializable ramp
specifications rather than exposing a third-party palette crate's types.
Initial built-in ramps may be implemented with `colorous`, subject to license
verification during implementation. Custom stop-based ramps and reversal will
be part of the stable model.

Ramp kinds remain explicit:

- sequential for ordered low-to-high values;
- diverging with a caller-specified meaningful center; and
- qualitative for categories.

The first resolved visual channels are color, opacity, size, stroke width, and
visibility. The library does not decide how a renderer interprets pixels,
points, lines, curves, or GPU instances.

## vectorizer-rs Integration

`vectorizer-rs` will retain its existing `preview.png` byte and meaning
unchanged. Attribute styling must not alter the current five-artifact run
bundle, manifest v5, Parquet metadata v2, or
`vectorizer_per_image_artifact_contract_v5`.

The first integration will be a `vectorizer-rs render` subcommand that:

1. reads an existing `curves.parquet`;
2. adapts its curve rows and attributes to `attribute-styling`;
3. resolves a style plan; and
4. writes a caller-selected output outside the canonical vectorization bundle.

The existing flat vectorization invocation remains valid:

```bash
vectorizer-rs -i image.png -o out -m 2,3 -b 24,32
```

An optional future one-invocation `--style` path may render directly from the
in-memory `VectorizeResult`, but it requires a separate approved
artifact/publication contract. It must not be smuggled into the v5 bundle or
replace `preview.png`.

Filtering individual cubic rows may fragment contours. The renderer adapter
must therefore make the styling unit explicit as `segment` or `contour`; a
contour-level adapter must define attribute aggregation before classification.

## Rerun Integration

Rerun will consume the same filter, classifier, ramp, and style-plan semantics
through a Rerun-owned adapter. Rerun remains responsible for:

- extracting attributes from its current chunk/table representation;
- interactive UI state;
- selection and hover behavior;
- GPU instance data; and
- drawing.

The shared crate must not depend on Rerun crates or encode Rerun-specific UI
behavior.

## Repository Documentation

The bootstrap will include:

- `README.md` describing the library role, consumer relationships, and status;
- `AGENTS.md` as the canonical repository guidance;
- `CLAUDE.md` as a symlink to `AGENTS.md`;
- `CONTEXT.md` with the domain glossary and repository boundaries;
- dual `LICENSE-MIT` and `LICENSE-APACHE`;
- a library-first Cargo scaffold; and
- local validation commands.

`vectorizer-rs` documentation will add a short relationship section linking to
the new repository and explicitly preserving `preview.png`. It will describe
the future render-subcommand boundary as planned until that integration lands.

## Skills Repository Decision

No new agent skill is justified by an empty library scaffold. The
`vycorporation/skills` repository should not duplicate evolving crate
documentation.

Once a real `vectorizer-rs render` workflow exists:

- update the existing `vectorizer-rs` skill to discover and follow the source
  repository's current render contract; and
- create a dedicated attribute-styling skill only if the workflow becomes
  repeated, subtle, cross-repository, or machine-specific.

This follows the skills repository rule that product repositories own evolving
implementation contracts while skills teach stable workflows and routing.

## Errors and Determinism

Public construction is validated and fallible. Typed errors will distinguish:

- unknown attributes;
- incompatible scalar types;
- invalid filter expressions;
- invalid or unordered manual breaks;
- unsupported null/NaN/infinite inputs;
- impossible classifier requests; and
- invalid color-ramp or visual-channel specifications.

For identical ordered inputs and a validated specification, the resolved style
plan must be deterministic. Runtime observations do not belong in the plan.

## Testing and Validation

The implementation plan will require:

- unit tests for each filter operator and scalar compatibility rule;
- table-driven boundary, tie, null, and degenerate-input classifier tests;
- deterministic snapshot fixtures for resolved breaks, colors, and legends;
- reference fixtures for QGIS/R pretty breaks and established Jenks behavior;
- property tests for monotonic breaks and complete single assignment;
- large-input performance and allocation bounds before enabling Jenks on
  unbounded datasets;
- consumer contract tests in `vectorizer-rs` and Rerun adapters; and
- standard Rust formatting, Clippy, test, documentation, and diff-hygiene
  gates.

The bootstrap itself will validate with:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
git diff --check
```

## Explicit Non-Goals for the Bootstrap

- changing `vectorizer-rs` preview rendering;
- changing vectorizer artifact or Parquet schemas;
- implementing a standalone rendering executable;
- depending on QGIS or ArcGIS runtime code;
- embedding Arrow or Rerun types in the public interface;
- implementing every classifier in the initial commit;
- adding a skills package before an operational workflow exists; or
- claiming visual parity with QGIS or ArcGIS without reference evidence.

## Delivery Sequence

1. Bootstrap and validate the public library repository.
2. Define and implement the first classification/filter/style contracts.
3. Add the independent `vectorizer-rs render` adapter and command.
4. Add the Rerun adapter against the same resolved-plan fixtures.
5. Update shared skills only after the operational workflow stabilizes.
