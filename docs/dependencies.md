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

## GIS reference and palette policy

| Area | Current crate | Reference and direction |
| --- | --- | --- |
| Named catalog | All 48 `colorous` 1.0.16 presets, with stable names and kind metadata | RustGIS documents broader Matplotlib/cmocean coverage and GeoLibre-compatible names; consider additional scientifically useful palettes, not an arbitrary 48-palette ceiling. |
| Custom ramps | Caller-owned positioned stops, reversal, fixed RGBA lists | RustGIS supports custom ramps and QGIS/GDAL/GMT interchange; consider narrowly scoped importers when needed instead of adopting a GIS runtime. |
| Classifiers | Unsampled weighted Jenks and population-sigma classes alongside existing modes | RustGIS/GeoLibre are primary design references; algorithms with the same name can differ in sampling, tie and interval conventions. |
| Legends and UI | Immutable classes and assignments; UI owned by consumers | GeoLibre's preview-before-apply and explicit manual mode after editing breaks are useful interaction references. |

No palette is excluded merely because it is absent from these projects.
The existing versioned 48-preset catalog remains stable; future additions must define naming, provenance, interpolation, categorical capacity and reproducible colors.
Do not silently cycle an exhausted categorical palette or relabel an approximate classifier.

RustGIS repository code is Apache-2.0; GeoLibre repository code is MIT.
Those repository licenses do not automatically cover every palette, bundled image or third-party asset.
Check exact files and asset attribution before copying; retain required notices.
This implementation introduces no new dependency or copied palette data.
A larger precomputed ramp table generally costs binary/data size rather than per-feature classification time; avoid loading a GIS application merely to sample colors.
Resolve and reuse a style plan rather than rerunning classification during every redraw.
