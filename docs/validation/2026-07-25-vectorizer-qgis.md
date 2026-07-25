# Real vectorizer-rs to QGIS validation

**Date:** 2026-07-25  
**Attribute:** `length_estimate`  
**Classifier:** quantile/equal count, five requested classes  
**Ramp:** Viridis

## Sources

- `vectorizer-rs`: `5ff289174c8b0e0ed3a5158b4cc160d7ec5d51ef`
- `spatial-io-rs`: `d2183395262bba770e4e3ed4830645990c9bcf89`
- source fixture: `vectorizer-rs/tests/fixtures/pcb_roi/roi.png`
- vectorizer run: `m1_b24_fixed`
- vectorizer artifact contract: manifest v5

The run produced 119 cubic rows across 56 contours in a 170 by 170
top-left-origin, y-down working raster.

The validation harness intentionally remained outside all three repositories.
It:

1. read the canonical `curves.parquet` by the documented Arrow-54 column names;
2. adapted `length_estimate`, `edge_strength`, and stable
   `(contour_id, segment_id)` identities into `attribute-styling`;
3. resolved five quantile classes and Viridis colors;
4. adapted each cubic independently into `spatial-io`;
5. flattened with `recursive_convex_hull_bound_v1` at 0.25 pixel;
6. wrote pixel-coordinate GeoParquet 1.1 with explicit null CRS; and
7. loaded and rendered the result with QGIS.

This is an independent pre-integration harness. It changes neither
`vectorizer-rs` nor `spatial-io-rs`.

## Style result

The five class populations were `24, 24, 23, 24, 24`. QGIS received the class
index, label, RGBA channels, and hexadecimal color as ordinary attributes.

The resolved inclusive upper bounds were:

```text
2.2018231150048706
2.9730243835200123
3.622486331371381
4.609820397077103
8.754189046407188
```

## Independent QGIS evidence

QGIS `4.2.0-Belém do Pará` loaded the output through its OGR provider and
reported:

```text
storage: Parquet
geometry: LineString
features: 119
extent: (0.0, 0.0) - (166.10676274578123, 169.0)
CRS valid: false
```

The invalid QGIS CRS is expected and correct for explicit pixel coordinates;
the GeoParquet metadata must not imply a geographic CRS.

A saved QGIS project contains a categorized renderer over `class_index` using
the exact five resolved Viridis colors. A second QGIS render used one-pixel
black lines for geometry-only comparison with canonical `preview.png`.

QGIS renders Cartesian y-up while the vectorizer contract is top-left/y-down.
The QGIS images were therefore vertically flipped before image-space
comparison; no stored geometry was changed.

At a non-white threshold of 250:

```text
exact mask IoU: 0.8068880688806888
one-pixel precision: 1.0
one-pixel recall: 1.0
one-pixel F1: 1.0
```

At the darker core threshold of 128:

```text
exact mask IoU: 0.5754276827371695
one-pixel precision: 1.0
one-pixel recall: 0.9299065420560748
one-pixel F1: 0.963680387409201
```

The output is not expected to be pixel-identical because QGIS and
`preview.png` use different line sampling, antialiasing, and stroke behavior.
The one-pixel comparison shows that the derived QGIS geometry follows the
canonical preview support after coordinate normalization.

## Artifact attestations

```text
curves.parquet
811c877b4a85095a3913a8d59c5c9f3528dc9714d2812b60786f666e2cc57457

styled-lines.parquet
73363a919b3dd42bca4f8e91f5aabef99f147c44d7afe799a431ae480185bd0a

preview.png
bf6620dc0c3b69aeffe9d369d7a643470b8e3e159b6c82eeb175ebefbcfdb23a

QGIS geometry render, y-down comparison orientation
9b7cadefa6fcc1b4231423ac95ef3e4e0a350c01709d9c198822cdaf0b75d069

QGIS styled render, y-down comparison orientation
14bb1a0331ce6c12934ef97c1ab5ce91fdcf8b5d1653847255c6981b528d7924
```
