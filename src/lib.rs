//! Renderer-neutral attribute filtering, classification, color ramps, and
//! immutable style resolution.
//!
//! Consumers adapt their own storage into crate-owned records and apply the
//! resulting plan with their own renderer.

mod filter;
mod model;
mod pretty;
mod ramp;
mod style;
#[cfg(feature = "stylx")]
mod stylx;

pub(crate) const MAXIMUM_CLASSES: usize = 4096;

pub use filter::{Comparison, ComparisonOperator, FilterExpression, evaluate_filter};
pub use model::{AttributeValue, FeatureRecord, FiniteF64};
pub use pretty::{PRETTY_BREAKS_IDENTITY, pretty_upper_bounds};
pub use ramp::{
    BUILT_IN_RAMP_CATALOG_IDENTITY, BuiltInRamp, BuiltInRampKind,
    CUSTOM_RAMP_INTERPOLATION_IDENTITY, ColorRamp, ColorStop, Rgba, built_in_ramps,
};
pub use style::{
    Classification, Classifier, FeatureStyleAssignment, FilterOutcome, ResolvedStylePlan,
    StyleClass, StyleSpec, resolve_style,
};
#[cfg(feature = "stylx")]
pub use stylx::{
    STYLX_READER_IDENTITY, StylxCatalog, StylxError, StylxRamp, StylxUnsupportedEntry,
    StylxUnsupportedReason, read_stylx,
};

use thiserror::Error;

/// Failures while validating or resolving an attribute style.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum StylingError {
    /// A numerical attribute was NaN or infinite.
    #[error("numerical attributes must be finite")]
    NonFiniteNumber,
    /// A stable feature identity was empty.
    #[error("feature identities must not be empty")]
    EmptyFeatureId,
    /// Two input features had the same stable identity.
    #[error("duplicate feature identity: {0}")]
    DuplicateFeatureId(String),
    /// An attribute named by a specification was absent.
    #[error("unknown attribute: {0}")]
    UnknownAttribute(String),
    /// Two attribute values cannot be compared under the requested operator.
    #[error("attribute values have incompatible types")]
    IncompatibleTypes,
    /// A Boolean group contained no child expressions.
    #[error("and/or filter expressions must contain at least one child")]
    EmptyBooleanExpression,
    /// A signed or unsigned integer could not be represented exactly as f64.
    #[error("integer is outside the exact f64 range")]
    NumberOutsideExactF64Range,
    /// No feature records were supplied.
    #[error("style resolution requires at least one input feature")]
    EmptyInput,
    /// A filter excluded every supplied feature.
    #[error("style filter selected no features")]
    EmptySelection,
    /// A numerical classifier had no non-null values.
    #[error("numerical classification requires at least one non-null value")]
    EmptyNumericInput,
    /// A classifier requested zero classes.
    #[error("class count must be greater than zero")]
    ZeroClasses,
    /// A numerical classifier exceeded the bounded class-count contract.
    #[error("requested {requested} classes, but the maximum is {maximum}")]
    TooManyClasses {
        /// Requested class count.
        requested: usize,
        /// Maximum supported class count.
        maximum: usize,
    },
    /// Pretty-break bounds were supplied in decreasing order.
    #[error("pretty-break minimum must not exceed its maximum")]
    InvalidPrettyRange,
    /// Finite pretty-break inputs could not produce finite covering bounds.
    #[error("pretty-break range cannot be represented with finite covering bounds")]
    UnrepresentablePrettyRange,
    /// Manual upper bounds were empty, non-finite, or not strictly increasing.
    #[error("manual upper bounds must be finite and strictly increasing")]
    UnorderedManualBreaks,
    /// The last manual upper bound did not cover the selected maximum.
    #[error("manual upper bounds do not cover every selected value")]
    ManualBreaksDoNotCoverValues,
    /// A ramp sample or custom stop position was not finite and in [0, 1].
    #[error("color-ramp positions must be finite and in the closed interval [0, 1]")]
    InvalidRampPosition,
    /// A custom ramp had no stops or stops were not strictly increasing.
    #[error("custom color-ramp stops must be non-empty and strictly increasing")]
    UnorderedRampStops,
    /// A categorical palette was used as a continuous gradient.
    #[error("categorical palette requires discrete sampling: {0}")]
    CategoricalPaletteRequiresDiscreteSampling(String),
    /// A categorical palette was asked for more colors than it contains.
    #[error("palette {palette} contains {maximum} colors but {requested} colors were requested")]
    TooManyPaletteColors {
        /// Stable built-in palette name.
        palette: String,
        /// Requested color count.
        requested: usize,
        /// Maximum fixed-color capacity.
        maximum: usize,
    },
    /// A fixed ramp was used as a continuous gradient.
    #[error("fixed color ramps require discrete sampling")]
    FixedRampRequiresDiscreteSampling,
    /// A fixed ramp was asked for more colors than it contains.
    #[error("fixed ramp contains {maximum} colors but {requested} colors were requested")]
    TooManyFixedRampColors {
        /// Requested color count.
        requested: usize,
        /// Maximum fixed-color capacity.
        maximum: usize,
    },
    /// A discrete color index was outside its requested color count.
    #[error("palette index {index} is outside a color count of {count}")]
    InvalidPaletteIndex {
        /// Requested zero-based index.
        index: usize,
        /// Requested color count.
        count: usize,
    },
}
