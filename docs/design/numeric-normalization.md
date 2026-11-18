# Normalized continuous styling

Objective: the shared numeric part of `aries-render-rs` #1/#3 Java renderer
replacement, as delegated November 18, 2026. This is not renderer integration or
full Java parity. Parent design review accepted the additive domain/transform
API and required sample standard deviation to reject fewer than two values.

## Contract

`Classification::ContinuousNormalized` uses a `NumericNormalization` with a
`NumericDomain` and `NumericTransform`. Existing `Continuous` and all discrete
classifiers retain their behavior and serialized results. Adding an enum variant
requires downstream exhaustive Rust matches to adopt it when updating the pin.

Domains are computed **after filtering**, using only non-null numeric values.
Null assignments remain uncolored. Empty numeric selection is an error. Existing
exact-integer conversion and nonfinite-attribute rejection still apply.

- Observed: selected minimum/maximum.
- Explicit: finite minimum <= maximum in original attribute units.
- PercentileTrim: fractions 0 <= lower < upper <= 1; sorted type-7 linear
  interpolation with rank `(n-1)*p`. Ties are not split; repeated values contribute
  their multiplicity. Singleton input and equal percentile endpoints are valid.
- StandardDeviation: mean ± `deviations` times **sample** standard deviation
  (n−1), positive finite multiplier. Fewer than two numeric values is a typed
  refusal. Constant samples resolve to a degenerate domain. A sorted, centered-and-scaled
  Welford calculation avoids intermediate square overflow and cancellation of
  adjacent large values; unrepresentable final bounds or precision-collapsed
  nonconstant domains fail instead of becoming infinity or invented ties. This is continuous normalization,
  not a standard-deviation discrete classifier.

Linear mapping uses the effective domain; log uses natural logarithms and
requires **all** selected non-null values and domain endpoints to be positive.
There is no implicit shift or silent omission of nonpositive values. Values
outside nondegenerate domains clip to 0/1. All values in a degenerate domain map
to 0.5, including values outside a caller-specified constant domain. Signed zero
bounds canonicalize to positive zero. Finite extreme linear ranges use scaled
arithmetic; log ratios use `ln_1p` where possible to preserve close endpoints.

## Frozen legend evidence

The immutable plan has optional `normalization()` metadata only for the new
variant. Its privately held policy, minimum, maximum and ramp are available via
read-only accessors; `position`, `value_at` and `sample_color` provide the same
mapping and ramp as assignments. Tick values are in original units; a log
midpoint is geometric, not arithmetic. Colors are sampled from the frozen ramp,
not reconstructed from observed assignments. No quantization/classes are added.

Deserialization validates finite ordered bounds, policy parameters, explicit
bounds agreement, log positivity, continuous ramp validity, absence of discrete
classes, and agreement of each colored assignment with its stored ramp position.
It does not attest original observations or reconstruct a statistical domain
from absent source data. Old plans omit the optional field entirely.

## Reference and intentional differences

Reference: `aries-toolset` Java `BezierToImageJb.java` at
`9dd307cf140fc11daf0c16d8c71d412db92ba46c`, normalization lines 495–558 and domain
discovery 4609–4690. The Java implementation is a useful established reference,
not ground truth. Its log path maps nonpositive values to zero while shifting
positive values when min<=0; this shared contract deliberately refuses that
ambiguous combination. Java percentile-trim discovery uses SQL `PERCENTILE_CONT(0.02/0.98)`; its
separate quantile branch uses Commons Math. This contract explicitly chooses
type 7 for percentile trim; no unmeasured difference or equivalence is claimed.
Java sigma breaks are clipped to observed range; here the effective domain is
unambiguously mean±k sample σ. These differences require explicit consumer
translation/documentation, not a misleading alias for Java options.

## Delivery boundary

This slice is a renderer-neutral library change, with no new dependencies.
Renderer pin adoption, CLI/config translation, smooth legend rendering and
end-to-end Java reference fixtures remain consumer work. Strict repo validation
and independent parent review are required before publication.
