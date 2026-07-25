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

Fixed ramps contain an ordered, bounded set of exact RGBA colors. They support
only discrete sampling, return the first requested colors in source order,
and reject requests above their capacity. Reversal reverses the selected
fixed sequence. The optional `.stylx` adapter returns this ordinary crate-owned
variant without exposing SQLite or CIM types.

## Deferred reference classifiers

Pretty breaks, Jenks natural breaks, and standard-deviation classification are
not aliases for the initial algorithms. Each requires its own issue, reference
fixtures, and performance bounds before becoming public behavior.
