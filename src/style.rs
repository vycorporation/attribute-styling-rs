use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

use serde::{Deserialize, Serialize};

use crate::{
    AttributeValue, ColorRamp, FeatureRecord, FilterExpression, FiniteF64, MAXIMUM_CLASSES, Rgba,
    StylingError, evaluate_filter, pretty_upper_bounds,
};

/// A numerical class-break algorithm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Classifier {
    /// Equal-width value ranges.
    EqualInterval {
        /// Requested class count.
        classes: usize,
    },
    /// Equal-count targets without splitting tied values.
    Quantile {
        /// Requested class count.
        classes: usize,
    },
    /// Round decimal intervals whose bounds cover the observed range.
    Pretty {
        /// Requested approximate class count.
        classes: usize,
    },
    /// Caller-supplied inclusive upper bounds.
    Manual {
        /// Strictly increasing finite inclusive upper bounds.
        upper_bounds: Vec<f64>,
    },
}

/// The attribute-to-style mapping family.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Classification {
    /// One style for every selected feature.
    Single,
    /// One class for each distinct non-null scalar.
    Categorical {
        /// Attribute name.
        attribute: String,
    },
    /// A classified numerical attribute.
    Numeric {
        /// Attribute name.
        attribute: String,
        /// Break algorithm.
        classifier: Classifier,
    },
    /// Unclassified interpolation across the observed numerical extent.
    Continuous {
        /// Attribute name.
        attribute: String,
    },
}

/// A complete renderer-neutral style request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StyleSpec {
    /// Optional feature filter.
    pub filter: Option<FilterExpression>,
    /// Classification family.
    pub classification: Classification,
    /// Color ramp.
    pub ramp: ColorRamp,
}

/// Whether one input feature passed the optional filter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterOutcome {
    /// Stable feature identity.
    feature_id: String,
    /// True when the feature is retained for style resolution.
    included: bool,
}

impl FilterOutcome {
    /// Returns the stable feature identity.
    #[must_use]
    pub fn feature_id(&self) -> &str {
        &self.feature_id
    }

    /// Returns whether the feature passed the optional filter.
    #[must_use]
    pub const fn included(&self) -> bool {
        self.included
    }
}

/// One resolved legend/class entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StyleClass {
    /// Zero-based deterministic class index.
    index: usize,
    /// Human-readable deterministic label.
    label: String,
    /// Inclusive lower extent of the first class and exclusive lower extent
    /// for subsequent numerical classes.
    lower_bound: Option<f64>,
    /// Inclusive numerical upper bound.
    upper_bound: Option<f64>,
    /// Resolved sRGB color.
    color: Rgba,
}

impl StyleClass {
    /// Returns the zero-based class index.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// Returns the deterministic legend label.
    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the numerical lower bound where applicable.
    #[must_use]
    pub const fn lower_bound(&self) -> Option<f64> {
        self.lower_bound
    }

    /// Returns the inclusive numerical upper bound where applicable.
    #[must_use]
    pub const fn upper_bound(&self) -> Option<f64> {
        self.upper_bound
    }

    /// Returns the resolved color.
    #[must_use]
    pub const fn color(&self) -> Rgba {
        self.color
    }
}

/// The resolved style for one selected feature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeatureStyleAssignment {
    /// Stable feature identity.
    feature_id: String,
    /// Class index, or none for null/continuous values.
    class_index: Option<usize>,
    /// Resolved color, or none for a null classification value.
    color: Option<Rgba>,
    /// Continuous ramp position, otherwise none.
    ramp_position: Option<f64>,
}

impl FeatureStyleAssignment {
    /// Returns the stable feature identity.
    #[must_use]
    pub fn feature_id(&self) -> &str {
        &self.feature_id
    }

    /// Returns the zero-based class index, if classified.
    #[must_use]
    pub const fn class_index(&self) -> Option<usize> {
        self.class_index
    }

    /// Returns the resolved color, or none for a null value.
    #[must_use]
    pub const fn color(&self) -> Option<Rgba> {
        self.color
    }

    /// Returns the continuous ramp position where applicable.
    #[must_use]
    pub const fn ramp_position(&self) -> Option<f64> {
        self.ramp_position
    }
}

/// An immutable deterministic style plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResolvedStylePlan {
    /// Filter results in input order.
    filter_outcomes: Vec<FilterOutcome>,
    /// Selected-feature assignments in input order.
    assignments: Vec<FeatureStyleAssignment>,
    /// Numerical or categorical classes in deterministic order.
    classes: Vec<StyleClass>,
    /// Legend entries in deterministic order.
    legend: Vec<StyleClass>,
    /// Requested numerical class count where applicable.
    requested_class_count: Option<usize>,
    /// Actual non-null class count.
    effective_class_count: usize,
}

impl ResolvedStylePlan {
    /// Returns filter results in input order.
    #[must_use]
    pub fn filter_outcomes(&self) -> &[FilterOutcome] {
        &self.filter_outcomes
    }

    /// Returns selected-feature assignments in input order.
    #[must_use]
    pub fn assignments(&self) -> &[FeatureStyleAssignment] {
        &self.assignments
    }

    /// Returns classes in deterministic order.
    #[must_use]
    pub fn classes(&self) -> &[StyleClass] {
        &self.classes
    }

    /// Returns legend entries in deterministic order.
    #[must_use]
    pub fn legend(&self) -> &[StyleClass] {
        &self.legend
    }

    /// Returns the requested class count where applicable.
    #[must_use]
    pub const fn requested_class_count(&self) -> Option<usize> {
        self.requested_class_count
    }

    /// Returns the effective non-null class count.
    #[must_use]
    pub const fn effective_class_count(&self) -> usize {
        self.effective_class_count
    }
}

/// Resolves a complete immutable style plan.
///
/// # Errors
///
/// Returns a typed error for empty input/selection, unknown or incompatible
/// attributes, invalid filters, impossible classifiers, or invalid ramps.
pub fn resolve_style(
    features: &[FeatureRecord],
    spec: &StyleSpec,
) -> Result<ResolvedStylePlan, StylingError> {
    if features.is_empty() {
        return Err(StylingError::EmptyInput);
    }
    let mut identities = BTreeSet::new();
    for feature in features {
        if !identities.insert(feature.feature_id()) {
            return Err(StylingError::DuplicateFeatureId(
                feature.feature_id().to_owned(),
            ));
        }
    }

    let mut filter_outcomes = Vec::with_capacity(features.len());
    let mut selected = Vec::with_capacity(features.len());
    for feature in features {
        let included = match &spec.filter {
            Some(filter) => evaluate_filter(feature, filter)?,
            None => true,
        };
        filter_outcomes.push(FilterOutcome {
            feature_id: feature.feature_id().to_owned(),
            included,
        });
        if included {
            selected.push(feature);
        }
    }
    if selected.is_empty() {
        return Err(StylingError::EmptySelection);
    }

    let partial = match &spec.classification {
        Classification::Single => resolve_single(&selected, &spec.ramp)?,
        Classification::Categorical { attribute } => {
            resolve_categorical(&selected, attribute, &spec.ramp)?
        }
        Classification::Numeric {
            attribute,
            classifier,
        } => resolve_numeric(&selected, attribute, classifier, &spec.ramp)?,
        Classification::Continuous { attribute } => {
            resolve_continuous(&selected, attribute, &spec.ramp)?
        }
    };

    Ok(ResolvedStylePlan {
        filter_outcomes,
        assignments: partial.assignments,
        legend: partial.classes.clone(),
        classes: partial.classes,
        requested_class_count: partial.requested_class_count,
        effective_class_count: partial.effective_class_count,
    })
}

struct PartialPlan {
    assignments: Vec<FeatureStyleAssignment>,
    classes: Vec<StyleClass>,
    requested_class_count: Option<usize>,
    effective_class_count: usize,
}

fn resolve_single(
    selected: &[&FeatureRecord],
    ramp: &ColorRamp,
) -> Result<PartialPlan, StylingError> {
    let color = ramp.sample_discrete(0, 1)?;
    let classes = vec![StyleClass {
        index: 0,
        label: "All features".to_owned(),
        lower_bound: None,
        upper_bound: None,
        color,
    }];
    let assignments = selected
        .iter()
        .map(|feature| FeatureStyleAssignment {
            feature_id: feature.feature_id().to_owned(),
            class_index: Some(0),
            color: Some(color),
            ramp_position: None,
        })
        .collect();
    Ok(PartialPlan {
        assignments,
        classes,
        requested_class_count: None,
        effective_class_count: 1,
    })
}

fn resolve_categorical(
    selected: &[&FeatureRecord],
    attribute: &str,
    ramp: &ColorRamp,
) -> Result<PartialPlan, StylingError> {
    let mut categories = BTreeMap::<(u8, String), (AttributeValue, String)>::new();
    for feature in selected {
        let value = feature.attribute(attribute)?;
        if !matches!(value, AttributeValue::Null) {
            categories
                .entry(value.category_sort_key()?)
                .or_insert((value.clone(), value.category_label()?));
        }
    }
    if categories.is_empty() {
        return Err(StylingError::EmptyNumericInput);
    }
    let effective = categories.len();
    let mut class_by_key = BTreeMap::new();
    let mut classes = Vec::with_capacity(effective);
    for (index, (key, (_, label))) in categories.into_iter().enumerate() {
        let color = ramp.sample_discrete(index, effective)?;
        class_by_key.insert(key, index);
        classes.push(StyleClass {
            index,
            label,
            lower_bound: None,
            upper_bound: None,
            color,
        });
    }
    let assignments = selected
        .iter()
        .map(|feature| {
            let value = feature.attribute(attribute)?;
            if matches!(value, AttributeValue::Null) {
                return Ok(FeatureStyleAssignment {
                    feature_id: feature.feature_id().to_owned(),
                    class_index: None,
                    color: None,
                    ramp_position: None,
                });
            }
            let index = class_by_key[&value.category_sort_key()?];
            Ok(FeatureStyleAssignment {
                feature_id: feature.feature_id().to_owned(),
                class_index: Some(index),
                color: Some(classes[index].color),
                ramp_position: None,
            })
        })
        .collect::<Result<Vec<_>, StylingError>>()?;
    Ok(PartialPlan {
        assignments,
        classes,
        requested_class_count: None,
        effective_class_count: effective,
    })
}

fn resolve_numeric(
    selected: &[&FeatureRecord],
    attribute: &str,
    classifier: &Classifier,
    ramp: &ColorRamp,
) -> Result<PartialPlan, StylingError> {
    let values = selected_numeric_values(selected, attribute)?;
    let mut sorted = values
        .iter()
        .filter_map(|(_, value)| *value)
        .collect::<Vec<_>>();
    if sorted.is_empty() {
        return Err(StylingError::EmptyNumericInput);
    }
    sorted.sort_by(f64::total_cmp);

    let (requested, upper_bounds) = match classifier {
        Classifier::EqualInterval { classes } => {
            require_class_count(*classes)?;
            (Some(*classes), equal_interval_breaks(&sorted, *classes))
        }
        Classifier::Quantile { classes } => {
            require_class_count(*classes)?;
            (Some(*classes), quantile_breaks(&sorted, *classes))
        }
        Classifier::Pretty { classes } => {
            require_class_count(*classes)?;
            let minimum = FiniteF64::new(sorted[0])?;
            let maximum = FiniteF64::new(*sorted.last().expect("non-empty"))?;
            (
                Some(*classes),
                pretty_upper_bounds(minimum, maximum, *classes)?
                    .into_iter()
                    .map(FiniteF64::get)
                    .collect(),
            )
        }
        Classifier::Manual { upper_bounds } => {
            validate_manual_breaks(upper_bounds, *sorted.last().expect("non-empty"))?;
            (Some(upper_bounds.len()), upper_bounds.clone())
        }
    };
    let effective = upper_bounds.len();
    let minimum = sorted[0];
    let classes = upper_bounds
        .iter()
        .enumerate()
        .map(|(index, upper)| {
            let lower = if index == 0 {
                minimum
            } else {
                upper_bounds[index - 1]
            };
            Ok(StyleClass {
                index,
                label: format_numeric_label(lower, *upper, index == 0),
                lower_bound: Some(lower),
                upper_bound: Some(*upper),
                color: ramp.sample_discrete(index, effective)?,
            })
        })
        .collect::<Result<Vec<_>, StylingError>>()?;
    let assignments = values
        .into_iter()
        .map(|(feature, value)| match value {
            None => FeatureStyleAssignment {
                feature_id: feature.feature_id().to_owned(),
                class_index: None,
                color: None,
                ramp_position: None,
            },
            Some(value) => {
                let index = upper_bounds
                    .iter()
                    .position(|upper| value <= *upper)
                    .expect("validated breaks cover selected values");
                FeatureStyleAssignment {
                    feature_id: feature.feature_id().to_owned(),
                    class_index: Some(index),
                    color: Some(classes[index].color),
                    ramp_position: None,
                }
            }
        })
        .collect();
    Ok(PartialPlan {
        assignments,
        classes,
        requested_class_count: requested,
        effective_class_count: effective,
    })
}

fn resolve_continuous(
    selected: &[&FeatureRecord],
    attribute: &str,
    ramp: &ColorRamp,
) -> Result<PartialPlan, StylingError> {
    let values = selected_numeric_values(selected, attribute)?;
    let observed = values
        .iter()
        .filter_map(|(_, value)| *value)
        .collect::<Vec<_>>();
    if observed.is_empty() {
        return Err(StylingError::EmptyNumericInput);
    }
    let minimum = observed.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = observed.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let assignments = values
        .into_iter()
        .map(|(feature, value)| match value {
            None => Ok(FeatureStyleAssignment {
                feature_id: feature.feature_id().to_owned(),
                class_index: None,
                color: None,
                ramp_position: None,
            }),
            Some(value) => {
                let position = if minimum.total_cmp(&maximum) == Ordering::Equal {
                    0.5
                } else {
                    (value - minimum) / (maximum - minimum)
                };
                Ok(FeatureStyleAssignment {
                    feature_id: feature.feature_id().to_owned(),
                    class_index: None,
                    color: Some(ramp.sample(position)?),
                    ramp_position: Some(position),
                })
            }
        })
        .collect::<Result<Vec<_>, StylingError>>()?;
    Ok(PartialPlan {
        assignments,
        classes: Vec::new(),
        requested_class_count: None,
        effective_class_count: 0,
    })
}

fn selected_numeric_values<'a>(
    selected: &[&'a FeatureRecord],
    attribute: &str,
) -> Result<Vec<(&'a FeatureRecord, Option<f64>)>, StylingError> {
    selected
        .iter()
        .map(|feature| Ok((*feature, feature.attribute(attribute)?.numeric_value()?)))
        .collect()
}

fn require_class_count(classes: usize) -> Result<(), StylingError> {
    if classes == 0 {
        Err(StylingError::ZeroClasses)
    } else if classes > MAXIMUM_CLASSES {
        Err(StylingError::TooManyClasses {
            requested: classes,
            maximum: MAXIMUM_CLASSES,
        })
    } else {
        Ok(())
    }
}

#[allow(clippy::cast_precision_loss)]
fn equal_interval_breaks(sorted: &[f64], classes: usize) -> Vec<f64> {
    let minimum = sorted[0];
    let maximum = *sorted.last().expect("non-empty");
    if minimum.total_cmp(&maximum) == Ordering::Equal {
        return vec![maximum];
    }
    let width = (maximum - minimum) / classes as f64;
    (1..=classes)
        .map(|index| {
            if index == classes {
                maximum
            } else {
                minimum + width * index as f64
            }
        })
        .collect()
}

#[allow(clippy::cast_precision_loss)]
fn quantile_breaks(sorted: &[f64], classes: usize) -> Vec<f64> {
    if sorted[0].total_cmp(sorted.last().expect("non-empty")) == Ordering::Equal {
        return vec![sorted[0]];
    }
    let mut groups = Vec::<(f64, usize)>::new();
    for value in sorted {
        match groups.last_mut() {
            Some((last, count)) if last.total_cmp(value) == Ordering::Equal => *count += 1,
            _ => groups.push((*value, 1)),
        }
    }
    let cumulative = groups
        .iter()
        .scan(0_usize, |total, (value, count)| {
            *total += count;
            Some((*value, *total))
        })
        .collect::<Vec<_>>();
    let mut breaks = Vec::<f64>::new();
    for index in 1..classes {
        let target = index as f64 * sorted.len() as f64 / classes as f64;
        let closest = cumulative
            .iter()
            .min_by(|(_, left), (_, right)| {
                let left_distance = (*left as f64 - target).abs();
                let right_distance = (*right as f64 - target).abs();
                left_distance
                    .total_cmp(&right_distance)
                    .then_with(|| left.cmp(right))
            })
            .expect("at least one value")
            .0;
        if breaks
            .last()
            .is_none_or(|last| last.total_cmp(&closest) != Ordering::Equal)
        {
            breaks.push(closest);
        }
    }
    let maximum = *sorted.last().expect("non-empty");
    if breaks
        .last()
        .is_none_or(|last| last.total_cmp(&maximum) != Ordering::Equal)
    {
        breaks.push(maximum);
    }
    breaks
}

fn validate_manual_breaks(upper_bounds: &[f64], maximum: f64) -> Result<(), StylingError> {
    if upper_bounds.is_empty()
        || upper_bounds.iter().any(|value| !value.is_finite())
        || upper_bounds
            .windows(2)
            .any(|pair| pair[0].total_cmp(&pair[1]) != Ordering::Less)
    {
        return Err(StylingError::UnorderedManualBreaks);
    }
    if *upper_bounds.last().expect("non-empty") < maximum {
        return Err(StylingError::ManualBreaksDoNotCoverValues);
    }
    Ok(())
}

fn format_numeric_label(lower: f64, upper: f64, first: bool) -> String {
    if first {
        format!("[{lower}, {upper}]")
    } else {
        format!("({lower}, {upper}]")
    }
}
