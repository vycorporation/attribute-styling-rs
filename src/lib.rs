//! Renderer-neutral attribute filtering, classification, and style resolution.
//!
//! The crate owns typed attribute and styling contracts. Consumers retain
//! responsibility for adapting storage and applying resolved styles to a
//! renderer.

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use thiserror::Error;

/// A dependency-neutral scalar attribute value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "value")]
pub enum AttributeValue {
    /// A missing value.
    Null,
    /// A Boolean value.
    Boolean(bool),
    /// A signed integer value.
    Signed(i64),
    /// An unsigned integer value.
    Unsigned(u64),
    /// A finite floating-point value.
    Float(FiniteF64),
    /// A UTF-8 string value.
    Text(String),
}

impl AttributeValue {
    /// Constructs a floating-point attribute after rejecting NaN and infinity.
    ///
    /// # Errors
    ///
    /// Returns [`StylingError::NonFiniteNumber`] when `value` is NaN or
    /// infinite.
    pub fn try_f64(value: f64) -> Result<Self, StylingError> {
        FiniteF64::new(value).map(Self::Float)
    }
}

/// A finite floating-point attribute value.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    /// Validates a finite floating-point value.
    ///
    /// # Errors
    ///
    /// Returns [`StylingError::NonFiniteNumber`] for NaN or infinity.
    pub fn new(value: f64) -> Result<Self, StylingError> {
        value
            .is_finite()
            .then_some(Self(value))
            .ok_or(StylingError::NonFiniteNumber)
    }

    /// Returns the validated finite value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Serialize for FiniteF64 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for FiniteF64 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Failures while validating or resolving an attribute style.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum StylingError {
    /// A numerical attribute was NaN or infinite.
    #[error("numerical attributes must be finite")]
    NonFiniteNumber,
}
