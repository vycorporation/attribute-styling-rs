use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::Path,
};

use rusqlite::{Connection, OpenFlags, types::Value as SqlValue};
use serde::Deserialize;
use thiserror::Error;

use crate::{ColorRamp, Rgba};

/// Stable identity for the narrow read-only `.stylx` contract.
pub const STYLX_READER_IDENTITY: &str = "stylx_fixed_rgb_v1";

const SQLITE_HEADER: &[u8; 16] = b"SQLite format 3\0";
const MAX_DATABASE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_RAMP_ITEMS: usize = 4096;
const MAX_ITEM_STRING_BYTES: usize = 4096;
const MAX_CONTENT_BYTES: usize = 1024 * 1024;
const MAX_STOPS: usize = 4096;
const CHANNEL_INTEGER_EPSILON: f64 = 1e-9;

const EXPECTED_ITEMS_COLUMNS: [(&str, &str, i64); 7] = [
    ("ID", "INTEGER", 1),
    ("CLASS", "INTEGER", 0),
    ("CATEGORY", "TEXT", 0),
    ("NAME", "TEXT", 0),
    ("TAGS", "TEXT", 0),
    ("CONTENT", "TEXT", 0),
    ("KEY", "TEXT", 0),
];

/// One supported fixed RGB ramp extracted from a caller-provided style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StylxRamp {
    name: String,
    key: String,
    category: String,
    colors: Vec<Rgba>,
}

impl StylxRamp {
    /// Returns the display name stored by the style.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the stable item key stored by the style.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Returns the style category.
    #[must_use]
    pub fn category(&self) -> &str {
        &self.category
    }

    /// Returns the exact ordered fixed colors.
    #[must_use]
    pub fn colors(&self) -> &[Rgba] {
        &self.colors
    }

    /// Creates an ordinary crate-owned fixed ramp.
    #[must_use]
    pub fn color_ramp(&self, reversed: bool) -> ColorRamp {
        ColorRamp::Fixed {
            colors: self.colors.clone(),
            reversed,
        }
    }
}

/// Why a color-ramp row was not imported.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum StylxUnsupportedReason {
    /// The CIM ramp kind is outside the fixed-ramp subset.
    #[error("unsupported CIM ramp type: {0}")]
    UnsupportedRampType(String),
    /// The fixed-ramp arrangement is not the default ordered arrangement.
    #[error("unsupported fixed-ramp arrangement: {0}")]
    UnsupportedArrangement(String),
    /// A color uses a non-RGB CIM color model.
    #[error("unsupported CIM color type: {0}")]
    UnsupportedColorType(String),
    /// A ramp or color names an external or non-default color profile.
    #[error("unsupported profile-dependent color space")]
    UnsupportedColorSpace,
    /// A channel cannot be represented exactly by the crate RGBA contract.
    #[error("color channel cannot be represented exactly as eight-bit RGBA")]
    InvalidColorChannel,
    /// The fixed ramp contains no colors.
    #[error("fixed ramp contains no colors")]
    EmptyFixedRamp,
    /// The fixed ramp exceeds the bounded stop count.
    #[error("fixed ramp contains {observed} colors; maximum is {maximum}")]
    TooManyStops {
        /// Observed stop count.
        observed: usize,
        /// Maximum supported stop count.
        maximum: usize,
    },
    /// The `CONTENT` value is not `SQLite` text.
    #[error("CONTENT is binary or otherwise not SQLite text")]
    NonTextContent,
    /// Required item fields are absent, non-text, or empty.
    #[error("required item fields are missing, non-text, or empty")]
    MalformedItem,
    /// The CIM JSON is malformed or contains fields outside the supported subset.
    #[error("unsupported or malformed CIM JSON: {0}")]
    MalformedCim(String),
}

/// One color-ramp row rejected without aborting catalog inspection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StylxUnsupportedEntry {
    name: Option<String>,
    key: Option<String>,
    reason: StylxUnsupportedReason,
}

impl StylxUnsupportedEntry {
    /// Returns the display name when it was a valid bounded string.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the item key when it was a valid bounded string.
    #[must_use]
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }

    /// Returns the typed incompatibility reason.
    #[must_use]
    pub const fn reason(&self) -> &StylxUnsupportedReason {
        &self.reason
    }
}

/// Deterministically ordered supported and rejected color-ramp entries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StylxCatalog {
    ramps: Vec<StylxRamp>,
    unsupported_entries: Vec<StylxUnsupportedEntry>,
}

impl StylxCatalog {
    /// Returns supported ramps in ascending `ITEMS.ID` order.
    #[must_use]
    pub fn ramps(&self) -> &[StylxRamp] {
        &self.ramps
    }

    /// Returns rejected color-ramp rows in ascending `ITEMS.ID` order.
    #[must_use]
    pub fn unsupported_entries(&self) -> &[StylxUnsupportedEntry] {
        &self.unsupported_entries
    }
}

/// Fatal failures that prevent safe style inspection.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum StylxError {
    /// The input is missing, unreadable, or not a regular file.
    #[error("cannot read .stylx input: {0}")]
    Input(String),
    /// The input exceeds the complete database byte limit.
    #[error(".stylx database is {observed} bytes; maximum is {maximum}")]
    DatabaseTooLarge {
        /// Observed file byte length.
        observed: u64,
        /// Maximum accepted file byte length.
        maximum: u64,
    },
    /// The input is not an ordinary rollback-journal `SQLite` 3 database.
    #[error("unsupported .stylx SQLite header or journal mode")]
    UnsupportedDatabaseEncoding,
    /// The `ITEMS` table does not exactly match the supported field shape.
    #[error("unsupported .stylx ITEMS table shape")]
    IncompatibleItemsSchema,
    /// `SQLite` could not inspect the bounded read-only database.
    #[error("cannot inspect .stylx database: {0}")]
    Database(String),
    /// The style contains more color-ramp rows than the bounded contract.
    #[error(".stylx contains {observed} ramp rows; maximum is {maximum}")]
    TooManyRampItems {
        /// Observed color-ramp row count.
        observed: usize,
        /// Maximum accepted color-ramp row count.
        maximum: usize,
    },
    /// A stored field exceeds its byte limit.
    #[error(".stylx row {row_id} field {field} is {observed} bytes; maximum is {maximum}")]
    ItemFieldTooLarge {
        /// Stable `SQLite` row identity.
        row_id: i64,
        /// Supported field name.
        field: &'static str,
        /// Observed byte length.
        observed: usize,
        /// Maximum accepted byte length.
        maximum: usize,
    },
    /// Two supported ramps have the same name or key.
    #[error("duplicate supported .stylx ramp {field}: {value}")]
    DuplicateRampIdentity {
        /// Conflicting identity field.
        field: &'static str,
        /// Repeated value.
        value: String,
    },
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CimFixedColorRamp {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "colorSpace")]
    color_space: Option<CimDefaultRgbColorSpace>,
    colors: Vec<CimRgbColor>,
    arrangement: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CimRgbColor {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "colorSpace")]
    color_space: Option<CimDefaultRgbColorSpace>,
    values: Vec<f64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CimDefaultRgbColorSpace {
    #[serde(rename = "type")]
    kind: String,
    url: String,
}

/// Reads the supported fixed RGB ramp subset from a caller-provided `.stylx`.
///
/// The input is opened with `SQLite`'s read-only flag after its size, header,
/// rollback-journal mode, and exact `ITEMS` field shape pass validation.
/// Queries are compile-time constants and no input-supplied SQL is executed.
///
/// # Errors
///
/// Returns a typed fatal error when the input or table is incompatible, a
/// database or item resource bound is exceeded, `SQLite` inspection fails, or
/// supported ramps have ambiguous duplicate identities. Unsupported CIM ramp
/// entries are retained in [`StylxCatalog::unsupported_entries`] instead.
pub fn read_stylx(path: impl AsRef<Path>) -> Result<StylxCatalog, StylxError> {
    let path = path.as_ref();
    validate_file(path)?;
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(database_error)?;
    validate_items_schema(&connection)?;
    validate_resource_bounds(&connection)?;
    read_catalog(&connection)
}

fn validate_file(path: &Path) -> Result<(), StylxError> {
    let symlink_metadata = fs::symlink_metadata(path)
        .map_err(|error| StylxError::Input(format!("{}: {error}", path.display())))?;
    if symlink_metadata.file_type().is_symlink() || !symlink_metadata.is_file() {
        return Err(StylxError::Input(format!(
            "{} is not a regular non-symlink file",
            path.display()
        )));
    }
    if symlink_metadata.len() > MAX_DATABASE_BYTES {
        return Err(StylxError::DatabaseTooLarge {
            observed: symlink_metadata.len(),
            maximum: MAX_DATABASE_BYTES,
        });
    }
    let mut header = [0_u8; 20];
    File::open(path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|error| StylxError::Input(format!("{}: {error}", path.display())))?;
    if &header[..16] != SQLITE_HEADER || header[18] != 1 || header[19] != 1 {
        return Err(StylxError::UnsupportedDatabaseEncoding);
    }
    Ok(())
}

fn validate_items_schema(connection: &Connection) -> Result<(), StylxError> {
    let mut statement = connection
        .prepare("PRAGMA table_info('ITEMS')")
        .map_err(database_error)?;
    let columns = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    let expected = EXPECTED_ITEMS_COLUMNS
        .iter()
        .map(|(name, data_type, primary_key)| {
            ((*name).to_owned(), (*data_type).to_owned(), *primary_key)
        })
        .collect::<Vec<_>>();
    if columns != expected {
        return Err(StylxError::IncompatibleItemsSchema);
    }
    Ok(())
}

fn validate_resource_bounds(connection: &Connection) -> Result<(), StylxError> {
    let count = connection
        .query_row("SELECT COUNT(*) FROM ITEMS WHERE CLASS = 2", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(database_error)?;
    let count = usize::try_from(count).map_err(|error| StylxError::Database(error.to_string()))?;
    if count > MAX_RAMP_ITEMS {
        return Err(StylxError::TooManyRampItems {
            observed: count,
            maximum: MAX_RAMP_ITEMS,
        });
    }

    let mut statement = connection
        .prepare(
            "SELECT ID, COALESCE(length(CAST(CATEGORY AS BLOB)), 0),
                    COALESCE(length(CAST(NAME AS BLOB)), 0),
                    COALESCE(length(CAST(TAGS AS BLOB)), 0),
                    COALESCE(length(CAST(CONTENT AS BLOB)), 0),
                    COALESCE(length(CAST(KEY AS BLOB)), 0)
             FROM ITEMS WHERE CLASS = 2 ORDER BY ID",
        )
        .map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    while let Some(row) = rows.next().map_err(database_error)? {
        let row_id = row.get::<_, i64>(0).map_err(database_error)?;
        for (index, field, maximum) in [
            (1, "CATEGORY", MAX_ITEM_STRING_BYTES),
            (2, "NAME", MAX_ITEM_STRING_BYTES),
            (3, "TAGS", MAX_ITEM_STRING_BYTES),
            (4, "CONTENT", MAX_CONTENT_BYTES),
            (5, "KEY", MAX_ITEM_STRING_BYTES),
        ] {
            let observed = row.get::<_, i64>(index).map_err(database_error)?;
            let observed = usize::try_from(observed)
                .map_err(|error| StylxError::Database(error.to_string()))?;
            if observed > maximum {
                return Err(StylxError::ItemFieldTooLarge {
                    row_id,
                    field,
                    observed,
                    maximum,
                });
            }
        }
    }
    Ok(())
}

fn read_catalog(connection: &Connection) -> Result<StylxCatalog, StylxError> {
    let mut statement = connection
        .prepare(
            "SELECT CATEGORY, NAME, CONTENT, KEY
             FROM ITEMS WHERE CLASS = 2 ORDER BY ID",
        )
        .map_err(database_error)?;
    let mut rows = statement.query([]).map_err(database_error)?;
    let mut ramps = Vec::new();
    let mut unsupported_entries = Vec::new();
    let mut names = BTreeSet::new();
    let mut keys = BTreeSet::new();
    while let Some(row) = rows.next().map_err(database_error)? {
        let category = row.get::<_, SqlValue>(0).map_err(database_error)?;
        let name = row.get::<_, SqlValue>(1).map_err(database_error)?;
        let content = row.get::<_, SqlValue>(2).map_err(database_error)?;
        let key = row.get::<_, SqlValue>(3).map_err(database_error)?;
        let name = nonempty_text(name);
        let key = nonempty_text(key);
        let category = nonempty_text(category);
        let content = match content {
            SqlValue::Text(content) => Some(content),
            _ => None,
        };
        let Some(content) = content else {
            unsupported_entries.push(unsupported(
                name,
                key,
                StylxUnsupportedReason::NonTextContent,
            ));
            continue;
        };
        let (name, key, category) = match (name, key, category) {
            (Some(name), Some(key), Some(category)) => (name, key, category),
            (name, key, _) => {
                unsupported_entries.push(unsupported(
                    name,
                    key,
                    StylxUnsupportedReason::MalformedItem,
                ));
                continue;
            }
        };
        match parse_fixed_ramp(&content) {
            Ok(colors) => {
                if !names.insert(name.clone()) {
                    return Err(StylxError::DuplicateRampIdentity {
                        field: "NAME",
                        value: name,
                    });
                }
                if !keys.insert(key.clone()) {
                    return Err(StylxError::DuplicateRampIdentity {
                        field: "KEY",
                        value: key,
                    });
                }
                ramps.push(StylxRamp {
                    name,
                    key,
                    category,
                    colors,
                });
            }
            Err(reason) => unsupported_entries.push(unsupported(Some(name), Some(key), reason)),
        }
    }
    Ok(StylxCatalog {
        ramps,
        unsupported_entries,
    })
}

fn parse_fixed_ramp(content: &str) -> Result<Vec<Rgba>, StylxUnsupportedReason> {
    let content = content.strip_suffix('\0').unwrap_or(content);
    if content.contains('\0') {
        return Err(StylxUnsupportedReason::MalformedCim(
            "embedded NUL in CIM JSON".to_owned(),
        ));
    }
    let value = serde_json::from_str::<serde_json::Value>(content)
        .map_err(|error| StylxUnsupportedReason::MalformedCim(error.to_string()))?;
    let kind = value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| StylxUnsupportedReason::MalformedCim("missing string type".to_owned()))?;
    if kind != "CIMFixedColorRamp" {
        return Err(StylxUnsupportedReason::UnsupportedRampType(kind.to_owned()));
    }
    let ramp = serde_json::from_value::<CimFixedColorRamp>(value)
        .map_err(|error| StylxUnsupportedReason::MalformedCim(error.to_string()))?;
    if ramp.kind != "CIMFixedColorRamp" {
        return Err(StylxUnsupportedReason::MalformedCim(
            "inconsistent fixed ramp type".to_owned(),
        ));
    }
    validate_color_space(ramp.color_space.as_ref())?;
    if ramp
        .arrangement
        .as_deref()
        .is_some_and(|value| value != "Default")
    {
        return Err(StylxUnsupportedReason::UnsupportedArrangement(
            ramp.arrangement.unwrap_or_default(),
        ));
    }
    if ramp.colors.is_empty() {
        return Err(StylxUnsupportedReason::EmptyFixedRamp);
    }
    if ramp.colors.len() > MAX_STOPS {
        return Err(StylxUnsupportedReason::TooManyStops {
            observed: ramp.colors.len(),
            maximum: MAX_STOPS,
        });
    }
    ramp.colors.into_iter().map(convert_color).collect()
}

fn convert_color(color: CimRgbColor) -> Result<Rgba, StylxUnsupportedReason> {
    if color.kind != "CIMRGBColor" {
        return Err(StylxUnsupportedReason::UnsupportedColorType(color.kind));
    }
    validate_color_space(color.color_space.as_ref())?;
    if color.values.len() != 4 {
        return Err(StylxUnsupportedReason::MalformedCim(
            "CIMRGBColor values must contain exactly four channels".to_owned(),
        ));
    }
    let red = rgb_channel(color.values[0])?;
    let green = rgb_channel(color.values[1])?;
    let blue = rgb_channel(color.values[2])?;
    let alpha = alpha_channel(color.values[3])?;
    Ok(Rgba::new(red, green, blue, alpha))
}

fn validate_color_space(
    color_space: Option<&CimDefaultRgbColorSpace>,
) -> Result<(), StylxUnsupportedReason> {
    if color_space.is_none_or(|color_space| {
        color_space.kind == "CIMICCColorSpace" && color_space.url == "Default RGB"
    }) {
        Ok(())
    } else {
        Err(StylxUnsupportedReason::UnsupportedColorSpace)
    }
}

fn rgb_channel(value: f64) -> Result<u8, StylxUnsupportedReason> {
    exact_byte(value, 1.0)
}

fn alpha_channel(value: f64) -> Result<u8, StylxUnsupportedReason> {
    exact_byte(value, 255.0 / 100.0)
}

fn exact_byte(value: f64, scale: f64) -> Result<u8, StylxUnsupportedReason> {
    let scaled = value * scale;
    let rounded = scaled.round();
    if scaled.is_finite()
        && (0.0..=255.0).contains(&scaled)
        && (scaled - rounded).abs() <= CHANNEL_INTEGER_EPSILON
    {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(rounded as u8)
    } else {
        Err(StylxUnsupportedReason::InvalidColorChannel)
    }
}

fn nonempty_text(value: SqlValue) -> Option<String> {
    match value {
        SqlValue::Text(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}

fn unsupported(
    name: Option<String>,
    key: Option<String>,
    reason: StylxUnsupportedReason,
) -> StylxUnsupportedEntry {
    StylxUnsupportedEntry { name, key, reason }
}

#[allow(clippy::needless_pass_by_value)]
fn database_error(error: rusqlite::Error) -> StylxError {
    StylxError::Database(error.to_string())
}
