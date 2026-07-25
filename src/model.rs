use std::{cmp::Ordering, collections::BTreeMap};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use crate::StylingError;

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

    #[allow(clippy::cast_precision_loss)]
    pub(crate) fn numeric_value(&self) -> Result<Option<f64>, StylingError> {
        const MAX_EXACT_INTEGER: u64 = 1_u64 << 53;
        match self {
            Self::Null => Ok(None),
            Self::Signed(value) => {
                let magnitude = value.unsigned_abs();
                if magnitude <= MAX_EXACT_INTEGER {
                    Ok(Some(*value as f64))
                } else {
                    Err(StylingError::NumberOutsideExactF64Range)
                }
            }
            Self::Unsigned(value) if *value <= MAX_EXACT_INTEGER => Ok(Some(*value as f64)),
            Self::Unsigned(_) => Err(StylingError::NumberOutsideExactF64Range),
            Self::Float(value) => Ok(Some(value.get())),
            Self::Boolean(_) | Self::Text(_) => Err(StylingError::IncompatibleTypes),
        }
    }

    pub(crate) fn compare(&self, other: &Self) -> Result<Ordering, StylingError> {
        match (self, other) {
            (Self::Null, Self::Null) => Ok(Ordering::Equal),
            (Self::Boolean(left), Self::Boolean(right)) => Ok(left.cmp(right)),
            (Self::Text(left), Self::Text(right)) => Ok(left.cmp(right)),
            (Self::Signed(left), Self::Signed(right)) => Ok(left.cmp(right)),
            (Self::Unsigned(left), Self::Unsigned(right)) => Ok(left.cmp(right)),
            (Self::Signed(left), Self::Unsigned(right)) => {
                if *left < 0 {
                    Ok(Ordering::Less)
                } else {
                    Ok(left.unsigned_abs().cmp(right))
                }
            }
            (Self::Unsigned(left), Self::Signed(right)) => {
                if *right < 0 {
                    Ok(Ordering::Greater)
                } else {
                    Ok(left.cmp(&right.unsigned_abs()))
                }
            }
            (
                Self::Signed(_) | Self::Unsigned(_) | Self::Float(_),
                Self::Signed(_) | Self::Unsigned(_) | Self::Float(_),
            ) => {
                let left = self
                    .numeric_value()?
                    .ok_or(StylingError::IncompatibleTypes)?;
                let right = other
                    .numeric_value()?
                    .ok_or(StylingError::IncompatibleTypes)?;
                left.partial_cmp(&right)
                    .ok_or(StylingError::NonFiniteNumber)
            }
            _ => Err(StylingError::IncompatibleTypes),
        }
    }

    pub(crate) fn category_sort_key(&self) -> Result<(u8, String), StylingError> {
        match self {
            Self::Null => Err(StylingError::IncompatibleTypes),
            Self::Boolean(value) => Ok((0, value.to_string())),
            Self::Signed(value) => Ok((1, format!("{value:+020}"))),
            Self::Unsigned(value) => Ok((2, format!("{value:020}"))),
            Self::Float(value) => Ok((3, format!("{:024.12e}", value.get()))),
            Self::Text(value) => Ok((4, value.clone())),
        }
    }

    pub(crate) fn category_label(&self) -> Result<String, StylingError> {
        match self {
            Self::Null => Err(StylingError::IncompatibleTypes),
            Self::Boolean(value) => Ok(value.to_string()),
            Self::Signed(value) => Ok(format!("signed:{value}")),
            Self::Unsigned(value) => Ok(format!("unsigned:{value}")),
            Self::Float(value) => Ok(format!("float:{}", value.get())),
            Self::Text(value) => Ok(value.clone()),
        }
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

/// One stable feature identity and its named attributes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureRecord {
    /// Caller-owned stable identity.
    feature_id: String,
    /// Deterministically ordered named attributes.
    attributes: BTreeMap<String, AttributeValue>,
}

impl FeatureRecord {
    /// Constructs a feature record with a non-empty identity.
    ///
    /// # Errors
    ///
    /// Returns [`StylingError::EmptyFeatureId`] for an empty identity.
    pub fn new(
        feature_id: impl Into<String>,
        attributes: BTreeMap<String, AttributeValue>,
    ) -> Result<Self, StylingError> {
        let feature_id = feature_id.into();
        if feature_id.is_empty() {
            return Err(StylingError::EmptyFeatureId);
        }
        Ok(Self {
            feature_id,
            attributes,
        })
    }

    /// Returns the stable caller-owned identity.
    #[must_use]
    pub fn feature_id(&self) -> &str {
        &self.feature_id
    }

    /// Returns the deterministically ordered attributes.
    #[must_use]
    pub const fn attributes(&self) -> &BTreeMap<String, AttributeValue> {
        &self.attributes
    }

    pub(crate) fn attribute(&self, name: &str) -> Result<&AttributeValue, StylingError> {
        self.attributes
            .get(name)
            .ok_or_else(|| StylingError::UnknownAttribute(name.to_owned()))
    }
}
