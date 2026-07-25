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
effective class.

Quantile targets equal population counts but never splits equal values. It
selects the cumulative tied-value boundary nearest each population target,
preferring the lower cumulative boundary on an exact distance tie. Duplicate
boundaries are removed, so the effective class count may be smaller than the
request.

Manual classification accepts strictly increasing, finite inclusive upper
bounds. The final bound must cover the observed maximum.

Pretty classification uses the crate-owned
`pretty_125_covering_v1` contract. Given a non-degenerate finite observed
range and an approximate requested interval count, it:

1. computes a target cell width from the observed span;
2. selects a step from `1`, `2`, or `5` times an integral power of ten using
   the R/QGIS high-unit bias `1.5` and five-unit bias `2.75`;
3. expands the implicit lower bound and returned upper bounds outward so they
   cover the complete observed range; and
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

The numerical class count is limited to 4,096 before class allocation.

## Other classifiers

Single classification assigns every selected feature to one class.

Categorical classification sorts distinct typed values deterministically.
Nulls receive no class or color.

Continuous classification maps the observed minimum to ramp position `0`, the
maximum to `1`, and intermediate values linearly between them. Degenerate
values use position `0.5`. It creates no artificial classes.

## Color ramps

Public plans contain only crate-owned eight-bit sRGB plus straight-alpha
colors.

The first built-in ramp is Viridis, privately sampled through `colorous`
1.0.16. Custom ramps interpolate each sRGB/alpha channel linearly between
strictly increasing stops. Reversal maps `t` to `1 - t`.

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

## Deferred reference classifiers

Jenks natural breaks and standard-deviation classification are not aliases for
the implemented algorithms. Each requires its own issue, reference fixtures,
and performance bounds before becoming public behavior.
