# Read-only `.stylx` fixed RGB ramps

The optional `stylx_fixed_rgb_v1` adapter reads a narrow subset of an ArcGIS
Pro `.stylx` file into crate-owned fixed `ColorRamp` values. It is a format
adapter, not an ArcGIS renderer or compatibility claim.

## Source-informed storage boundary

ArcGIS Pro documents styles as individual databases with a `.stylx`
extension. An Esri support procedure shows the `ITEMS` fields and identifies
`CONTENT` as CIM JSON. The CIM API documents `CIMFixedColorRamp` as an ordered
color list.

- <https://pro.arcgis.com/en/pro-app/latest/help/projects/styles.htm>
- <https://support.esri.com/en-us/knowledge-base/how-to-import-svg-files-into-a-style-file-using-python--000038814>
- <https://pro.arcgis.com/en/pro-app/3.5/sdk/api-reference/topic1375.html>
- <https://developers.arcgis.com/javascript/latest/references/core/symbols/cim/types/>

Version 1 requires an ordinary rollback-journal SQLite 3 file and exactly this
table field shape:

```sql
CREATE TABLE ITEMS (
    ID INTEGER PRIMARY KEY,
    CLASS INTEGER,
    CATEGORY TEXT,
    NAME TEXT,
    TAGS TEXT,
    CONTENT TEXT,
    KEY TEXT
);
```

Additional or missing fields are incompatible. A unique index on `KEY` is
normal but is not relied upon; the reader independently rejects duplicate
supported names or keys. Only `CLASS = 2` color-ramp rows are inspected, in
ascending `ID` order.

`CONTENT` must be UTF-8 SQLite TEXT containing one JSON object. A single
trailing NUL used by observed ArcGIS Pro style files is accepted as storage
termination; embedded or repeated trailing NULs are malformed. Binary or
compressed blobs are reported as unsupported and are never decoded.

## Supported CIM subset

The root object must be `CIMFixedColorRamp`. Its `colors` must be a non-empty
ordered list of `CIMRGBColor` objects. `arrangement` may be absent or
`"Default"`.

Each RGB color contains exactly four finite values:

```text
red, green, blue: 0 through 255
alpha:            0 through 100 percent
```

The RGB values must be within `1e-9` of an eight-bit integer. The alpha
percentage must map within `1e-9` of an exact eight-bit alpha. This accepts
observed floating serialization residue while rejecting a color that would
require lossy quantization. For example, alpha `80` maps exactly to `204`,
while alpha `50` would map to `127.5` and is rejected.

`colorSpace` may be absent or exactly
`{"type":"CIMICCColorSpace","url":"Default RGB"}`. Any other ICC URL or color
space is profile-dependent and unsupported.

The reader reports rather than approximates:

- continuous, multipart, algorithmic, random, or procedural ramp types;
- CMYK, HSV, HSL, LAB, ICC-profile-dependent, and other color models;
- non-default fixed-ramp arrangements;
- malformed, unknown-field, empty, binary, or compressed content;
- colors that cannot enter the crate's RGBA contract without quantization.

Recognized entries and typed unsupported entries are both returned so a
caller can build a complete inspection report.

## Read-only and resource contract

Before SQLite is opened, the reader rejects symlinks, non-regular files,
files above 16 MiB, non-SQLite headers, and WAL-mode headers. It then opens the
input with SQLite's read-only flag and uses only compile-time constant SQL.
It never executes SQL from `CONTENT`, performs network access, or writes the
database.

The fixed limits are:

```text
database bytes       16,777,216
color-ramp rows           4,096
name/key/category/tag     4,096 UTF-8 bytes each
CONTENT bytes         1,048,576 per row
fixed colors              4,096 per ramp
```

The synthetic tests set the file read-only, attest its bytes before and after
inspection, and verify that no journal, WAL, or shared-memory sidecar appears.
They also cover every resource limit and rejection family.

## External validation

On 2026-07-25, the ignored real-file test was run against the public
`Scientific Colour Maps 8.0.1` ArcGIS Pro style (item
`db36aff062ec4218a9bd384b74dc6e9d`) without committing or redistributing it:

```text
file bytes: 6,258,688
sha256: 366431c67b309ad127415d30262c02ce17765da5683baa0d36e2416ef1d9c06a
supported fixed RGB ramps: 160
unsupported multipart ramps: 40
unsupported lossy-channel fixed ramps: 22
input bytes changed: no
SQLite sidecars created: no
```

The command was:

```bash
STYLX_REAL_FIXTURE=/tmp/scientific-colour-maps-8.stylx \
  cargo test --features stylx --test stylx_contract \
  inspects_external_validation_style_without_mutating_it -- \
  --ignored --exact
```

The caller remains responsible for rights to any style file. No Esri asset is
included in this repository.
