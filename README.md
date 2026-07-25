# attribute-styling

`attribute-styling` is a renderer-neutral Rust library for filtering typed
attributes, classifying values, sampling color ramps, and resolving immutable
visual style plans.

The Cargo package is `attribute-styling`; Rust code imports it as
`attribute_styling`.

## Why a separate crate?

[`vectorizer-rs`](https://github.com/vycorporation/vectorizer-rs), the
[`vycorporation/rerun`](https://github.com/vycorporation/rerun) graph
workbench, rendering CLIs, and future tools need the same GIS-style
classification semantics. This repository makes those rules reusable without
making any renderer, storage engine, or GUI authoritative.

The public interface contains crate-owned types. It does not expose Arrow,
Parquet, DataFusion, Rerun, image, wgpu, or vectorizer-specific types.

## Current capabilities

The first functional release provides:

- dependency-neutral feature records and typed scalar attributes;
- null, comparison, membership, and Boolean-composition filters;
- single, categorical, equal-interval, quantile/equal-count, pretty,
  manual-break, and continuous classification;
- a crate-owned RGBA contract, all 48 supported named `colorous` presets,
  custom stops, and reversal;
- deterministic class boundaries, assignments, colors, filter outcomes, and
  legends; and
- explicit null, tie, boundary, degenerate-input, and requested/effective class
  semantics.

Natural breaks (Jenks) and standard-deviation classification remain separate
follow-up slices requiring reference fixtures.

The optional `stylx` feature adds a narrow read-only adapter for
caller-provided ArcGIS Pro style databases. It imports only fixed ramps made
from losslessly representable RGB colors; see
[`docs/stylx.md`](docs/stylx.md) for the exact compatibility and security
contract.

`vectorizer-rs` keeps its canonical `preview.png` and artifact-v5 bundle
unchanged. Its planned `render` subcommand will style existing output and write
separate caller-selected artifacts.

## Example

```rust
use std::collections::BTreeMap;

use attribute_styling::{
    AttributeValue, Classification, Classifier, ColorRamp, FeatureRecord,
    StyleSpec, resolve_style,
};

let features = [1.0, 4.0, 9.0]
    .into_iter()
    .enumerate()
    .map(|(index, length)| {
        FeatureRecord::new(
            format!("curve-{index}"),
            BTreeMap::from([(
                "length".to_owned(),
                AttributeValue::try_f64(length)?,
            )]),
        )
    })
    .collect::<Result<Vec<_>, _>>()?;

let plan = resolve_style(
    &features,
    &StyleSpec {
        filter: None,
        classification: Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::Quantile { classes: 3 },
        },
        ramp: ColorRamp::Viridis { reversed: false },
    },
)?;

assert_eq!(plan.effective_class_count(), 3);

# Ok::<(), attribute_styling::StylingError>(())
```

See [`docs/classification.md`](docs/classification.md) for exact boundaries,
ties, nulls, ramp catalog semantics, and determinism.
The first independent real-artifact check is recorded in
[`docs/validation/2026-07-25-vectorizer-qgis.md`](docs/validation/2026-07-25-vectorizer-qgis.md).

## Validation

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
git diff --check
```

Run `cargo test --no-default-features` to prove the SQLite adapter remains
absent from the dependency-neutral default build.

The crate uses Rust 2024, has MSRV 1.89, forbids unsafe Rust, and is licensed
under either Apache-2.0 or MIT.
