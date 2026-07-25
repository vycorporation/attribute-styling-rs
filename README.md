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

## Status

The bootstrap establishes the crate boundary and dependency-neutral scalar
model. Classification, filters, ramps, and resolved style plans will land in
independently reviewed slices.

`vectorizer-rs` keeps its canonical `preview.png` and artifact-v5 bundle
unchanged. Its planned `render` subcommand will style existing output and write
separate caller-selected artifacts.

## Example

```rust
use attribute_styling::AttributeValue;

let score = AttributeValue::try_f64(0.75)?;
let label = AttributeValue::Text("component-12".to_owned());

# Ok::<(), attribute_styling::StylingError>(())
```

## Validation

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
git diff --check
```

The crate uses Rust 2024, has MSRV 1.89, forbids unsafe Rust, and is licensed
under either Apache-2.0 or MIT.
