# Classification contract

This document defines the first durable behavior implemented by
`attribute-styling`.

## Records and filters

Each input record has a unique, non-empty stable feature identity and an
ordered map of typed attributes. Floats are finite by construction. Integers
outside the exactly representable `f64` range are rejected when numerical
classification or mixed integer/float comparison requires conversion.

Filters support:

- `is null`;
- equality and ordering for compatible values;
- set membership;
- non-empty `and` and `or` groups; and
- `not`.

An unknown attribute or incompatible comparison fails the complete resolution.
Every child or membership literal is evaluated for validity even after the
Boolean result is determined; short circuit does not hide invalid operands.
Filter outcomes remain in input order. Excluded features do not participate in
classification or assignments.

## Numerical boundaries

Numerical classes use inclusive upper bounds. The first interval includes the
observed minimum; subsequent intervals exclude the prior upper bound:

```text
[minimum, break_0]
(break_0, break_1]
...
```

Every non-null selected value has exactly one class.

Equal interval divides the observed minimum-to-maximum extent into the
requested number of equal-width ranges. Degenerate input resolves to one
effective class. Finite endpoints produce finite bounds even when subtracting
them would overflow. Boundaries that coincide at local `f64` precision are merged,
so effective count can be smaller than requested. Generated bounds must remain
finite, strictly increasing, and covering; impossible bounds fail with a typed
error before any plan is returned.

Quantile targets equal population counts but never splits equal values. It
selects the cumulative tied-value boundary nearest each population target,
preferring the lower cumulative boundary on an exact distance tie. Duplicate
boundaries are removed, so the effective class count may be smaller than the
request.

Manual classification accepts strictly increasing, finite inclusive upper
bounds. The first bound must be at or above the observed minimum, and the final
bound must cover the observed maximum. A first bound below the minimum fails
with `ManualBreaksDoNotCoverValues` rather than creating an inverted interval.
The first bound may equal the minimum. Later intervals may be unpopulated;
their declared bounds and requested count are retained.

Pretty classification uses the crate-owned
`pretty_125_covering_v2` contract. Given a non-degenerate finite observed
range and an approximate requested interval count, it:

1. computes a target cell width from the observed span;
2. selects a step from `1`, `2`, or `5` times an integral power of ten using
   the R/QGIS high-unit bias `1.5` and five-unit bias `2.75`;
3. expands the implicit lower bound and returned upper bounds outward so they
   cover the complete observed range with exact numerical comparisons; and
4. reports the requested count separately from the number of effective
   intervals.

The first returned break is the inclusive upper bound of the class beginning
at the observed minimum. Subsequent classes are lower-exclusive and
upper-inclusive. The last round bound may be greater than the observed
maximum. A step that would create more than 4,096 effective classes is
deterministically coarsened to the next `1`/`2`/`5` step before allocation.

An exactly degenerate range produces one class at the observed value. A
nonzero span that cannot produce a positive cell or distinct decimal index at
the local `f64` precision also collapses to one class at the observed maximum.
Decreasing bounds are invalid. Finite endpoints whose subtraction overflows
fail as an unrepresentable pretty range.

The numerical and categorical class counts are limited to 4,096 before class
or legend allocation, including caller-supplied manual bounds and distinct
categorical values. Repeated categories do not consume additional capacity.

Numerical equality treats `-0.0` and `+0.0` as the same value. Classification
normalizes their bounds and labels to positive zero; quantile ties cannot split
them, and a manual boundary list containing both is not strictly increasing.

Version 2 corrects version 1's epsilon-based endpoint acceptance, which could
leave an observed maximum uncovered and panic during assignment. The original
July reference record remains version 1 evidence; its checked-in reference
matrix also passes the version 2 implementation.

## Other classifiers

Single classification assigns every selected feature to one class.

Categorical classification sorts distinct typed values deterministically.
Its keys retain exact integer and finite-float identities; adjacent distinct
binary64 values cannot merge through display rounding. Float signed zeros
share one category, consistent with numerical equality. Type distinctions
(signed, unsigned, float, Boolean, and text) remain part of category identity.
Nulls receive no class or color.

Continuous classification maps the observed minimum to ramp position `0`, the
maximum to `1`, and intermediate values linearly between them. Degenerate
values use position `0.5`. Opposite-sign extreme finite endpoints use scaled
normalization to avoid overflowing intermediate differences. It creates no
artificial classes.

## Serialized plans

Feature-record deserialization enforces the same non-empty identity rule as
`FeatureRecord::new`. Resolved results retain their existing serialized field
shape and `Deserialize` support. Deserialization rejects inconsistent counts,
class indices, legends, bounds, selected feature identities
or order, class colors, and null/classified/continuous assignment shapes.
Numerical bounds must be finite, paired, ordered, and contiguous. Continuous
positions must be finite and in `[0, 1]`. Class and legend collections are
limited to 4,096 entries during deserialization, before an unbounded collection
can be allocated.

Stored labels are preserved and must agree between classes and the legend.
They are not regenerated from decoded bounds, because a serialization format
may round floating-point values differently from the original display text.

These are structural checks on the stored plan. Original attributes and the
style specification are not present in the plan, so deserialization cannot
recompute classification decisions or attest the source data.

## Color ramps

Public plans contain only crate-owned eight-bit sRGB plus straight-alpha
colors.

The stable built-in catalog identity is `colorous_1_0_16_catalog_v1`. It
exposes all 48 presets supported by `colorous` 1.0.16 through unique,
case-sensitive, lowercase kebab-case names:

- sequential: `blue-green`, `blue-purple`, `blues`, `cividis`, `cool`,
  `cubehelix`, `green-blue`, `greens`, `greys`, `inferno`, `magma`,
  `orange-red`, `oranges`, `plasma`, `purple-blue`, `purple-blue-green`,
  `purple-red`, `purples`, `red-purple`, `reds`, `turbo`, `viridis`, `warm`,
  `yellow-green`, `yellow-green-blue`, `yellow-orange-brown`, and
  `yellow-orange-red`;
- diverging: `brown-green`, `pink-green`, `purple-green`, `purple-orange`,
  `red-blue`, `red-grey`, `red-yellow-blue`, `red-yellow-green`, and
  `spectral`;
- cyclical: `rainbow` and `sinebow`;
- categorical: `accent` (8), `category10` (10), `dark2` (8), `paired` (12),
  `pastel1` (9), `pastel2` (8), `set1` (9), `set2` (8), `set3` (12), and
  `tableau10` (10).

The number in parentheses is the categorical capacity and recommended
category count. A categorical request returns the first requested fixed
colors in source order and fails when the request exceeds capacity. Reversal
reverses that selected fixed sequence. Categorical presets reject continuous
sampling instead of being interpolated into gradients.

Continuous named sampling delegates to the pinned preset at a normalized
position. Discrete sampling delegates to the preset's rational sampling,
which keeps cyclical endpoints from being duplicated; a one-color request
uses the ramp midpoint. The existing
`ColorRamp::Viridis` variant remains a source-compatible convenience, but
catalog discovery and string lookup expose only the single name `viridis`.

Custom ramps retain `srgb_linear_channel_round_v1`: each straight-alpha
sRGB/alpha channel is interpolated linearly between strictly increasing stops
and rounded to the nearest eight-bit value. Reversal maps `t` to `1 - t`.

## Reference evidence

The checked-in `pretty_break_reference_fixture_v1` matrix records positive,
negative, crossing-zero, tiny, huge, and degenerate cases from QGIS 4.2.0
`QgsSymbolLayerUtils.prettyBreaks` and R 4.6.0 `base::pretty`. See
`docs/validation/2026-07-25-pretty-breaks.md` for exact commands and the
semantic comparison.

The crate follows R's round covering-bound behavior for non-degenerate
representable ranges. QGIS uses the same unit-selection family but clamps its
first or final returned class break to the observed range and imposes a
larger fixed minimum cell. Degenerate behavior also differs. These references
inform the crate-owned contract; the crate does not claim QGIS or R parity.

Fixed ramps contain an ordered, bounded set of exact RGBA colors. They support
only discrete sampling, return the first requested colors in source order,
and reject requests above their capacity. Reversal reverses the selected
fixed sequence. The optional `.stylx` adapter returns this ordinary crate-owned
variant without exposing SQLite or CIM types.

## Statistical classifiers

Both new classifiers use the existing finite numeric input conversion, null exclusion, inclusive upper bounds, stable feature order, color assignment and immutable-plan contracts.
The requested count must be in `1..=4096`; constant inputs and a request for one class yield one class.
Nulls receive no class/color; all-null inputs return `EmptyNumericInput`.

### Jenks natural breaks

Use `Classifier::Jenks { classes: 5 }`, or `{"kind":"jenks","classes":5}` in serialized specifications.
The implementation minimizes within-class squared error over sorted distinct values weighted by their observation frequency.
It does not sample, split equal values, or silently substitute quantiles.
The effective count is the smaller of the requested count and the number of distinct values.
Every bound is an observed value, and the final bound is the observed maximum.
Equal computed candidate costs choose the earliest split.

The implementation uses divide-and-conquer dynamic programming with balanced weighted-moment range queries, avoiding unstable subtraction of raw prefix squared sums.
For `n` observations, `m` distinct values and `k` effective classes, sorting costs `O(n log n)`, optimization costs `O(k m log² m)`, and backpointer storage costs `O(k m)`.
The DP is unsampled but still uses `f64` arithmetic: it does not promise exact-real arithmetic or identity with other software's sampling and tie conventions.
Small cases are checked against an independent exhaustive partition oracle.

Before allocating backpointers, `(m+1)*(k+1)` is checked against 16,777,216 cells (128 MiB on a 64-bit host).
Exceeding that bound or failing the checked backpointer allocation returns `ClassificationWorkspaceExceeded`, not a sampled or reduced-class result.
One-class and one-class-per-distinct-value requests bypass the DP allocation.
This workspace bound is separate from observation count; repeated U8/U16 values have at most 256/65,536 distinct values.
Other input/plan arrays still scale with the observations.

### Standard deviation

Use `Classifier::StandardDeviation { classes: 5 }`, or `{"kind":"standard_deviation","classes":5}`.
For requested count `k`, interior cuts are `mean + (i - k/2) * sigma`, for `i=1..k-1`, using **population** variance (division by `n`, not `n-1`).
Only distinct representable cuts strictly inside the observed minimum/maximum are retained; the observed maximum is appended.
Consequently the effective count can be smaller, and interior classes can be empty.
Even `k` places a cut at the mean; odd `k` gives a central interval spanning the mean.
In particular, `k=2` divides at the mean independently of sigma.
This differs deliberately from the sample-sigma domain used for normalized continuous styling.
Moments are accumulated over all non-null observations in sorted order, making results invariant to feature permutation.

### Numerical limits and references

Both algorithms normalize the observed range before moment calculations to avoid overflowing squared values.
Finite opposite-sign endpoints whose direct subtraction overflows are handled with half-scaled arithmetic.
If normalization collapses distinct observations to the same floating-point value, resolution returns `UnrepresentableNumericRange` instead of silently merging them.
Jenks also rejects normalized adjacent gaps whose square would be zero or subnormal, preventing underflow from changing the optimization objective.
Bounds are returned in original units; rounded duplicate standard-deviation cuts are removed.

Primary design references are [RustGIS cartography](https://github.com/opengeos/RustGIS/blob/main/docs/user-guide/cartography.md) and [GeoLibre styling](https://github.com/opengeos/GeoLibre/blob/main/docs/user-guide/styling.md).
RustGIS's inspected Jenks implementation samples at most 1,000 values; this crate intentionally preserves all observations.
These references guide methods and explanation, not a claim of QGIS/ArcGIS/renderer or screenshot parity.
The tests include cluster fixtures, weighted ties, an exhaustive partition oracle, population-sigma fixtures, nulls, round trips, subnormals, extreme finite ranges and a repeated-byte distribution.

## Normalized continuous extension

`ContinuousNormalized` adds typed observed/explicit/percentile-trim/sample-sigma
numeric domains and linear/log mapping, with immutable original-unit legend
evidence. It does not change legacy `Continuous` plans or add discrete classes.
See [the normalization contract](design/numeric-normalization.md) for exact
clipping, nulls, percentile interpolation, sample-size, overflow, serialized
validation, and deliberate Java-reference differences.
