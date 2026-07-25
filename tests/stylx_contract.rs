#![cfg(feature = "stylx")]

use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use attribute_styling::{
    ColorRamp, Rgba, StylingError, StylxError, StylxUnsupportedReason, read_stylx,
};
use rusqlite::{Connection, params};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct TempStylx {
    path: PathBuf,
}

impl TempStylx {
    fn empty() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "attribute-styling-stylx-{}-{sequence}.stylx",
            std::process::id()
        ));
        let connection = Connection::open(&path).expect("create synthetic stylx");
        connection
            .execute_batch(include_str!("fixtures/stylx/items-schema.sql"))
            .expect("create supported schema");
        drop(connection);
        Self { path }
    }

    fn new() -> Self {
        let fixture = Self::empty();
        let connection = Connection::open(fixture.path()).expect("open synthetic stylx");
        connection
            .execute(
                "INSERT INTO ITEMS(CLASS, CATEGORY, NAME, TAGS, CONTENT, KEY)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    2,
                    "Qualitative",
                    "Synthetic Fixed RGB",
                    "synthetic,test",
                    include_str!("fixtures/stylx/fixed-rgb.json"),
                    "synthetic-fixed-rgb"
                ],
            )
            .expect("insert supported ramp");
        connection
            .execute(
                "INSERT INTO ITEMS(CLASS, CATEGORY, NAME, TAGS, CONTENT, KEY)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    2,
                    "Sequential",
                    "Unsupported Multipart",
                    "synthetic,test",
                    r#"{"type":"CIMMultipartColorRamp","colorRamps":[],"weights":[]}"#,
                    "unsupported-multipart"
                ],
            )
            .expect("insert unsupported ramp");
        drop(connection);
        fixture
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn insert(&self, id: i64, name: &str, content: &str, key: &str) {
        Connection::open(self.path())
            .expect("open fixture")
            .execute(
                "INSERT INTO ITEMS(ID, CLASS, CATEGORY, NAME, TAGS, CONTENT, KEY)
                 VALUES (?1, 2, 'Synthetic', ?2, 'test', ?3, ?4)",
                params![id, name, content, key],
            )
            .expect("insert fixture row");
    }
}

impl Drop for TempStylx {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        for suffix in ["-journal", "-wal", "-shm"] {
            let _ = fs::remove_file(format!("{}{}", self.path.display(), suffix));
        }
    }
}

#[test]
fn reads_supported_fixed_rgb_and_reports_unsupported_ramps_without_writing() {
    let fixture = TempStylx::new();
    let original = fs::read(fixture.path()).expect("fixture bytes");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(fixture.path(), fs::Permissions::from_mode(0o444))
            .expect("read-only fixture");
    }

    let catalog = read_stylx(fixture.path()).expect("read synthetic stylx");

    assert_eq!(
        fs::read(fixture.path()).expect("bytes after read"),
        original
    );
    for suffix in ["-journal", "-wal", "-shm"] {
        assert!(
            !PathBuf::from(format!("{}{}", fixture.path().display(), suffix)).exists(),
            "unexpected SQLite sidecar {suffix}"
        );
    }
    assert_eq!(catalog.ramps().len(), 1);
    let ramp = &catalog.ramps()[0];
    assert_eq!(ramp.name(), "Synthetic Fixed RGB");
    assert_eq!(ramp.key(), "synthetic-fixed-rgb");
    assert_eq!(ramp.category(), "Qualitative");
    assert_eq!(
        ramp.colors(),
        &[
            Rgba::new(38, 13, 64, 255),
            Rgba::new(128, 64, 32, 204),
            Rgba::new(255, 255, 255, 0),
        ]
    );

    assert_eq!(catalog.unsupported_entries().len(), 1);
    assert_eq!(
        catalog.unsupported_entries()[0].reason(),
        &StylxUnsupportedReason::UnsupportedRampType("CIMMultipartColorRamp".to_owned())
    );
}

#[test]
fn extracted_fixed_ramp_uses_exact_bounded_discrete_colors() {
    let fixture = TempStylx::new();
    let catalog = read_stylx(fixture.path()).expect("read synthetic stylx");
    let forward = catalog.ramps()[0].color_ramp(false);
    assert_eq!(
        forward.sample_discrete(0, 3).expect("first"),
        Rgba::new(38, 13, 64, 255)
    );
    assert_eq!(
        forward.sample_discrete(0, 4),
        Err(StylingError::TooManyFixedRampColors {
            requested: 4,
            maximum: 3,
        })
    );
    assert_eq!(
        forward.sample(0.5),
        Err(StylingError::FixedRampRequiresDiscreteSampling)
    );

    let reversed = catalog.ramps()[0].color_ramp(true);
    assert_eq!(
        reversed.sample_discrete(0, 3).expect("reversed first"),
        Rgba::new(255, 255, 255, 0)
    );
    assert!(matches!(forward, ColorRamp::Fixed { .. }));
}

#[test]
fn reports_each_unsupported_cim_case_without_approximating() {
    let fixture = TempStylx::empty();
    for (id, name, content) in [
        (
            1,
            "algorithmic",
            r#"{"type":"CIMLinearContinuousColorRamp","fromColor":{},"toColor":{}}"#,
        ),
        (
            2,
            "cmyk",
            r#"{"type":"CIMFixedColorRamp","colors":[{"type":"CIMCMYKColor","values":[0,0,0,0,100]}]}"#,
        ),
        (
            3,
            "profile",
            r#"{"type":"CIMFixedColorRamp","colorSpace":{"type":"CIMICCColorSpace","url":"Company.icc"},"colors":[{"type":"CIMRGBColor","values":[1,2,3,100]}]}"#,
        ),
        (
            4,
            "arrangement",
            r#"{"type":"CIMFixedColorRamp","colors":[{"type":"CIMRGBColor","values":[1,2,3,100]}],"arrangement":"Random"}"#,
        ),
        (
            5,
            "malformed",
            r#"{"type":"CIMFixedColorRamp","colors":"bad"}"#,
        ),
        (6, "empty", r#"{"type":"CIMFixedColorRamp","colors":[]}"#),
        (
            7,
            "lossy-alpha",
            r#"{"type":"CIMFixedColorRamp","colors":[{"type":"CIMRGBColor","values":[1,2,3,50]}]}"#,
        ),
    ] {
        fixture.insert(id, name, content, name);
    }
    let many_stops = format!(
        r#"{{"type":"CIMFixedColorRamp","colors":[{}]}}"#,
        std::iter::repeat_n(r#"{"type":"CIMRGBColor","values":[1,2,3,100]}"#, 4097)
            .collect::<Vec<_>>()
            .join(",")
    );
    fixture.insert(8, "too-many-stops", &many_stops, "too-many-stops");

    let catalog = read_stylx(fixture.path()).expect("inspect unsupported matrix");
    assert!(catalog.ramps().is_empty());
    let reasons = catalog
        .unsupported_entries()
        .iter()
        .map(attribute_styling::StylxUnsupportedEntry::reason)
        .collect::<Vec<_>>();
    assert_eq!(reasons.len(), 8);
    assert_eq!(
        reasons[0],
        &StylxUnsupportedReason::UnsupportedRampType("CIMLinearContinuousColorRamp".to_owned())
    );
    assert_eq!(
        reasons[1],
        &StylxUnsupportedReason::UnsupportedColorType("CIMCMYKColor".to_owned())
    );
    assert_eq!(reasons[2], &StylxUnsupportedReason::UnsupportedColorSpace);
    assert_eq!(
        reasons[3],
        &StylxUnsupportedReason::UnsupportedArrangement("Random".to_owned())
    );
    assert!(matches!(
        reasons[4],
        StylxUnsupportedReason::MalformedCim(_)
    ));
    assert_eq!(reasons[5], &StylxUnsupportedReason::EmptyFixedRamp);
    assert_eq!(reasons[6], &StylxUnsupportedReason::InvalidColorChannel);
    assert_eq!(
        reasons[7],
        &StylxUnsupportedReason::TooManyStops {
            observed: 4097,
            maximum: 4096,
        }
    );
}

#[test]
fn binary_content_is_reported_and_never_decoded_as_cim() {
    let fixture = TempStylx::empty();
    Connection::open(fixture.path())
        .expect("open fixture")
        .execute(
            "INSERT INTO ITEMS(ID, CLASS, CATEGORY, NAME, TAGS, CONTENT, KEY)
             VALUES (1, 2, 'Synthetic', 'Compressed', 'test', ?1, 'compressed')",
            params![vec![0x78_u8, 0x9c, 0x03, 0x00]],
        )
        .expect("insert binary content");

    let catalog = read_stylx(fixture.path()).expect("inspect binary row");
    assert_eq!(
        catalog.unsupported_entries()[0].reason(),
        &StylxUnsupportedReason::NonTextContent
    );
}

#[test]
fn rejects_incompatible_schema_and_ambiguous_supported_identities() {
    let bad_schema = TempStylx::empty();
    {
        let connection = Connection::open(bad_schema.path()).expect("open schema fixture");
        connection
            .execute_batch("ALTER TABLE ITEMS ADD COLUMN EXTRA TEXT;")
            .expect("alter schema");
    }
    assert_eq!(
        read_stylx(bad_schema.path()),
        Err(StylxError::IncompatibleItemsSchema)
    );

    let duplicates = TempStylx::empty();
    duplicates.insert(
        1,
        "Duplicate",
        include_str!("fixtures/stylx/fixed-rgb.json"),
        "first",
    );
    duplicates.insert(
        2,
        "Duplicate",
        include_str!("fixtures/stylx/fixed-rgb.json"),
        "second",
    );
    assert_eq!(
        read_stylx(duplicates.path()),
        Err(StylxError::DuplicateRampIdentity {
            field: "NAME",
            value: "Duplicate".to_owned(),
        })
    );
}

#[test]
fn rejects_database_row_and_field_resource_overruns_before_decoding() {
    let too_many_rows = TempStylx::empty();
    {
        let mut connection = Connection::open(too_many_rows.path()).expect("open rows fixture");
        let transaction = connection.transaction().expect("transaction");
        for id in 1_i64..=4097 {
            transaction
                .execute(
                    "INSERT INTO ITEMS(ID, CLASS, CATEGORY, NAME, TAGS, CONTENT, KEY)
                     VALUES (?1, 2, 'x', ?2, '', '{}', ?2)",
                    params![id, format!("ramp-{id}")],
                )
                .expect("insert bounded row matrix");
        }
        transaction.commit().expect("commit rows");
    }
    assert!(matches!(
        read_stylx(too_many_rows.path()),
        Err(StylxError::TooManyRampItems {
            observed: 4097,
            maximum: 4096,
        })
    ));

    let long_name = TempStylx::empty();
    long_name.insert(
        1,
        &"n".repeat(4097),
        include_str!("fixtures/stylx/fixed-rgb.json"),
        "long-name",
    );
    assert!(matches!(
        read_stylx(long_name.path()),
        Err(StylxError::ItemFieldTooLarge {
            field: "NAME",
            observed: 4097,
            maximum: 4096,
            ..
        })
    ));

    let large_content = TempStylx::empty();
    large_content.insert(1, "large", &" ".repeat(1024 * 1024 + 1), "large");
    assert!(matches!(
        read_stylx(large_content.path()),
        Err(StylxError::ItemFieldTooLarge {
            field: "CONTENT",
            observed: 1_048_577,
            maximum: 1_048_576,
            ..
        })
    ));
}

#[test]
fn rejects_oversized_and_wal_mode_files_before_sqlite_open() {
    let oversized = TempStylx::empty();
    OpenOptions::new()
        .write(true)
        .open(oversized.path())
        .expect("open oversized fixture")
        .set_len(16 * 1024 * 1024 + 1)
        .expect("extend sparse fixture");
    assert!(matches!(
        read_stylx(oversized.path()),
        Err(StylxError::DatabaseTooLarge {
            observed: 16_777_217,
            maximum: 16_777_216,
        })
    ));

    let wal = TempStylx::empty();
    let mut file = OpenOptions::new()
        .write(true)
        .open(wal.path())
        .expect("open header fixture");
    file.seek(SeekFrom::Start(18)).expect("seek header");
    file.write_all(&[2]).expect("mark WAL write version");
    drop(file);
    assert_eq!(
        read_stylx(wal.path()),
        Err(StylxError::UnsupportedDatabaseEncoding)
    );
}

#[test]
#[ignore = "requires STYLX_REAL_FIXTURE pointing to the uncommitted validation style"]
fn inspects_external_validation_style_without_mutating_it() {
    let path = PathBuf::from(
        std::env::var_os("STYLX_REAL_FIXTURE").expect("set STYLX_REAL_FIXTURE for this test"),
    );
    let original = fs::read(&path).expect("real fixture bytes");
    let catalog = read_stylx(&path).expect("read real fixture");

    assert_eq!(catalog.ramps().len(), 160);
    assert_eq!(catalog.unsupported_entries().len(), 62);
    assert_eq!(catalog.ramps()[0].name(), "acton10");
    assert_eq!(catalog.ramps()[0].colors().len(), 10);
    assert_eq!(
        catalog
            .unsupported_entries()
            .iter()
            .filter(|entry| matches!(
                entry.reason(),
                StylxUnsupportedReason::UnsupportedRampType(_)
            ))
            .count(),
        40
    );
    assert_eq!(
        catalog
            .unsupported_entries()
            .iter()
            .filter(|entry| matches!(entry.reason(), StylxUnsupportedReason::InvalidColorChannel))
            .count(),
        22
    );
    assert_eq!(fs::read(&path).expect("bytes after read"), original);
    for suffix in ["-journal", "-wal", "-shm"] {
        assert!(
            !PathBuf::from(format!("{}{suffix}", path.display())).exists(),
            "unexpected SQLite sidecar {suffix}"
        );
    }
}
