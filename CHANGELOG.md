# Changelog

## 0.1.1 — 2026-10-04

Fixes the four findings from the Sol 6.1 High review of commit `bbe77cf`.
Tracked in [issue #20](https://github.com/vycorporation/attribute-styling-rs/issues/20).

- Return a typed error for invalid UTF-8 in `.stylx` text fields, preserving
  read-only inspection.
- Reject empty feature identities during `FeatureRecord` deserialization.
- Validate resolved-plan and result-entry deserialization while retaining the
  public types, `Deserialize` support, and serialized field shape. Bound class
  and legend collections to 4,096 entries during decoding.
- Reject manual upper bounds below the observed minimum instead of returning
  inverted legend intervals.

Previously accepted malformed serialized inputs and inverted manual intervals
now return errors. Valid classification behavior and wire formats are retained.

## 0.1.0

Initial development version with typed attribute filters, deterministic
classification, color ramps, immutable resolved plans, and the optional
read-only `.stylx` fixed RGB adapter. No release tag or GitHub release existed
for this version when the 0.1.1 corrections began.
