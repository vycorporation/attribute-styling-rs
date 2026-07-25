# Dependency decisions

## `colorous` 1.0.16

`colorous` is an Apache-2.0 Rust crate that ports the established
`d3-scale-chromatic` schemes. Version 1.0.16 supplies all 48 presets exposed
by `colorous_1_0_16_catalog_v1` and supports Rust versions older than this
repository's MSRV.

Its types remain private. Public specifications and resolved plans use
crate-owned `BuiltInRamp`, `BuiltInRampKind`, `ColorRamp`, and `Rgba` types,
allowing the implementation to change without coupling consumers to the
palette dependency. Categorical arrays are copied as exact fixed RGBA colors;
the library does not interpolate or extend them.

The core crate does not depend on Arrow, Parquet, DataFusion, Rerun, image,
wgpu, vectorizer-rs, spatial-io, QGIS, ArcGIS, DuckDB, or SedonaDB.

## Optional `.stylx` dependencies

The `stylx` feature enables `rusqlite` 0.37 with its bundled SQLite source and
`serde_json` 1.0. `rusqlite` is MIT-licensed, SQLite is public domain, and
`serde_json` is MIT or Apache-2.0. Version 0.37 was selected because it passes
this crate's Rust 1.89 MSRV gate; its types remain private.

Default features are empty. A default build therefore has no SQLite, JSON, C
compiler, ArcGIS, network, or style-file dependency. The optional adapter
opens only a caller-provided local database in read-only mode.
