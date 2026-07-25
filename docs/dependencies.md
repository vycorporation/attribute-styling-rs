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
