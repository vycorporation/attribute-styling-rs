# attribute-styling Context

## Repository role

`attribute-styling` owns deterministic attribute filtering, classification,
color-ramp sampling, and resolved visual style semantics. It accepts
dependency-neutral records and returns immutable renderer-neutral plans.

It does not read table formats, vectorize rasters, convert geometry, or render
pixels and GPU primitives.

## Vocabulary

- **Attribute value**: a null, Boolean, integer, finite float, or UTF-8 string.
- **Filter outcome**: whether a stable feature identity remains visible.
- **Classifier**: a named rule that assigns compatible values to classes.
- **Class break**: a declared boundary with explicit inclusivity semantics.
- **Color ramp**: ordered or qualitative colors sampled by a named policy.
- **Resolved style plan**: immutable feature assignments, colors, visual
  channels, breaks, and legend entries.
- **Consumer adapter**: code owned by a caller that translates its storage and
  renderer types at the crate boundary.

## Ownership boundaries

`vectorizer-rs` owns raster decoding, cubic geometry, canonical artifacts,
manifest contracts, and `preview.png`.

`spatial-io-rs` owns primitive conversion, coordinate meaning, CRS handling,
and portable spatial I/O.

Rerun owns graph state, interactive UI, selection, GPU instance data, and
drawing.

This crate owns styling semantics shared by those consumers. It must not gain
their native public types or runtime dependencies.

## Invariants

- Public construction is validated and fallible.
- Numerical classification never accepts NaN or infinity.
- Identical ordered input and a validated specification produce identical
  assignments, breaks, colors, and legend order.
- Null and incompatible-type behavior is explicit.
- The plan does not contain runtime observations.
- Renderers do not silently reinterpret class boundaries.
- Unsafe Rust is forbidden.

## Current classification contract

- Numerical intervals are lower-exclusive and upper-inclusive, except that the
  first class includes its observed lower extent.
- Quantile targets never split equal values. Ties may reduce the effective
  class count.
- Null classification values remain selected but receive no class or color.
- Filters remove features before classification while retaining one outcome
  per input identity.
- Continuous classification records normalized ramp positions and no
  artificial classes.
- Class counts are bounded to prevent specifications from forcing unbounded
  allocations.

## Delivery order

1. Bootstrap the crate and dependency-neutral model. Complete.
2. Implement the first filter, classification, ramp, and plan contracts.
   Complete.
3. Add independently validated pretty-break, Jenks, and standard-deviation
   slices.
4. Add an independent `vectorizer-rs render` adapter.
5. Add Rerun integration against shared fixtures.
6. Add skills only after an operational workflow stabilizes.
