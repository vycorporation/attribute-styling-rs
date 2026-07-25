use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{AttributeValue, FeatureRecord, StylingError};

/// A comparison operator for compatible scalar values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonOperator {
    /// Equal.
    Equal,
    /// Not equal.
    NotEqual,
    /// Less than.
    LessThan,
    /// Less than or equal.
    LessThanOrEqual,
    /// Greater than.
    GreaterThan,
    /// Greater than or equal.
    GreaterThanOrEqual,
}

/// A comparison between one named attribute and one literal value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Comparison {
    /// Attribute name.
    pub attribute: String,
    /// Requested comparison.
    pub operator: ComparisonOperator,
    /// Literal right-hand value.
    pub value: AttributeValue,
}

impl Comparison {
    /// Constructs a comparison.
    #[must_use]
    pub fn new(
        attribute: impl Into<String>,
        operator: ComparisonOperator,
        value: AttributeValue,
    ) -> Self {
        Self {
            attribute: attribute.into(),
            operator,
            value,
        }
    }
}

/// A validated-on-evaluation renderer-neutral filter tree.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "operator", content = "arguments")]
pub enum FilterExpression {
    /// True when the named attribute is null.
    IsNull {
        /// Attribute name.
        attribute: String,
    },
    /// Compare one attribute with a literal.
    Compare(Comparison),
    /// True when the attribute equals any compatible literal.
    In {
        /// Attribute name.
        attribute: String,
        /// Candidate values.
        values: Vec<AttributeValue>,
    },
    /// True when every child is true.
    And(Vec<Self>),
    /// True when at least one child is true.
    Or(Vec<Self>),
    /// Negate a child expression.
    Not(Box<Self>),
}

/// Evaluates a filter against one feature.
///
/// # Errors
///
/// Returns a typed error for unknown attributes, empty Boolean groups, or
/// incompatible comparison types.
pub fn evaluate_filter(
    feature: &FeatureRecord,
    expression: &FilterExpression,
) -> Result<bool, StylingError> {
    match expression {
        FilterExpression::IsNull { attribute } => Ok(matches!(
            feature.attribute(attribute)?,
            AttributeValue::Null
        )),
        FilterExpression::Compare(comparison) => {
            let observed = feature.attribute(&comparison.attribute)?;
            let ordering = observed.compare(&comparison.value)?;
            Ok(match comparison.operator {
                ComparisonOperator::Equal => ordering == Ordering::Equal,
                ComparisonOperator::NotEqual => ordering != Ordering::Equal,
                ComparisonOperator::LessThan => ordering == Ordering::Less,
                ComparisonOperator::LessThanOrEqual => ordering != Ordering::Greater,
                ComparisonOperator::GreaterThan => ordering == Ordering::Greater,
                ComparisonOperator::GreaterThanOrEqual => ordering != Ordering::Less,
            })
        }
        FilterExpression::In { attribute, values } => {
            let observed = feature.attribute(attribute)?;
            for value in values {
                if observed.compare(value)? == Ordering::Equal {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        FilterExpression::And(children) => {
            require_children(children)?;
            for child in children {
                if !evaluate_filter(feature, child)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        FilterExpression::Or(children) => {
            require_children(children)?;
            for child in children {
                if evaluate_filter(feature, child)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        FilterExpression::Not(child) => Ok(!evaluate_filter(feature, child)?),
    }
}

fn require_children(children: &[FilterExpression]) -> Result<(), StylingError> {
    if children.is_empty() {
        Err(StylingError::EmptyBooleanExpression)
    } else {
        Ok(())
    }
}
