# Pretty-break reference validation

This evidence supports `pretty_125_covering_v1`. QGIS and R are independent
references, not runtime dependencies or compatibility specifications.

## Versions

```text
QGIS 4.2.0-Belém do Pará, revision ec9a7f91
R 4.6.0 (2026-04-24)
attribute-styling pretty_125_covering_v1
```

The fixture is
`tests/fixtures/pretty_breaks/qgis-r-v1.json`.

## QGIS

The installed QGIS Python binding was invoked with
`QgsSymbolLayerUtils.prettyBreaks(minimum, maximum, classes)` for every fixture
case. The QGIS implementation delegates the graduated `Pretty` classifier to
this helper:

- <https://github.com/qgis/QGIS/blob/master/src/core/classification/qgsclassificationprettybreaks.cpp>
- <https://github.com/qgis/QGIS/blob/master/src/core/symbology/qgssymbollayerutils.cpp>

The exact case matrix was:

```python
cases = [
    (1.0, 15.0, 5),
    (-9.0, -1.0, 5),
    (-3.2, 8.1, 5),
    (1e-9, 6e-9, 4),
    (1e90, 6e90, 4),
    (4.0, 4.0, 5),
]
for case in cases:
    print(case, list(QgsSymbolLayerUtils.prettyBreaks(*case)))
```

QGIS uses the R-derived 1/2/5 unit choice, then clamps an outer first or final
break to the observed minimum/maximum. Its fixed minimum cell collapses the
tiny fixture to one break.

## R

R was run from the pinned Nix package without adding a project dependency:

```bash
nix-shell -p R --run 'Rscript pretty-reference.R'
```

`pretty-reference.R` contained:

```r
cases <- list(
  c(1, 15),
  c(-9, -1),
  c(-3.2, 8.1),
  c(1e-9, 6e-9),
  c(1e90, 6e90),
  c(4, 4)
)
classes <- c(5, 5, 5, 4, 4, 5)
for (i in seq_along(cases)) {
  print(format(pretty(cases[[i]], n = classes[[i]]), digits = 17))
}
```

R's official contract chooses approximately the requested number of round
breakpoints with a 1/2/5 decimal unit and, with default `bounds = TRUE`,
covers the complete range:

- <https://stat.ethz.ch/R-manual/R-devel/library/base/html/pretty.html>
- <https://github.com/wch/r-source/blob/trunk/src/appl/pretty.c>

## Crate decision

For non-degenerate representable ranges, the crate emits the R-style round
covering upper bounds and does not clamp the last class to the observed
maximum. For `[-3.2, 8.1]` with five requested intervals this means:

```text
QGIS upper bounds:  -2, 0, 2, 4, 6, 8, 8.1
R boundaries:       -4, -2, 0, 2, 4, 6, 8, 10
crate upper bounds: -2, 0, 2, 4, 6, 8, 10
```

The crate deliberately collapses exact degenerate and unresolvable
floating-point spans to one class at the observed maximum. It returns typed
errors for reversed ranges, excessive requests, and finite endpoints whose
span is not representable.
