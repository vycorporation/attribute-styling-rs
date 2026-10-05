//! Validate serialized results at their public construction boundary.

use std::{collections::BTreeSet, fmt};

use serde::{
    Deserialize, Deserializer,
    de::{self, SeqAccess, Visitor},
};

use super::{FeatureStyleAssignment, FilterOutcome, ResolvedStylePlan, StyleClass};
use crate::{MAXIMUM_CLASSES, Rgba};

#[derive(Deserialize)]
#[serde(rename = "StyleClass")]
pub(super) struct UncheckedClass {
    index: usize,
    label: String,
    lower_bound: Option<f64>,
    upper_bound: Option<f64>,
    color: Rgba,
}

impl TryFrom<UncheckedClass> for StyleClass {
    type Error = &'static str;

    fn try_from(value: UncheckedClass) -> Result<Self, Self::Error> {
        if value.index >= MAXIMUM_CLASSES {
            return Err("class index must be below 4096");
        }
        match (value.lower_bound, value.upper_bound) {
            (None, None) => {}
            (Some(lower), Some(upper))
                if lower.is_finite() && upper.is_finite() && lower <= upper => {}
            _ => return Err("class bounds must be paired, finite, and ordered"),
        }
        Ok(Self {
            index: value.index,
            label: value.label,
            lower_bound: value.lower_bound,
            upper_bound: value.upper_bound,
            color: value.color,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename = "FeatureStyleAssignment")]
pub(super) struct UncheckedAssignment {
    #[serde(deserialize_with = "crate::model::deserialize_feature_id")]
    feature_id: String,
    class_index: Option<usize>,
    color: Option<Rgba>,
    ramp_position: Option<f64>,
}

impl TryFrom<UncheckedAssignment> for FeatureStyleAssignment {
    type Error = &'static str;

    fn try_from(value: UncheckedAssignment) -> Result<Self, Self::Error> {
        match (value.class_index, value.color, value.ramp_position) {
            (None, None, None) => {}
            (Some(index), Some(_), None) if index < MAXIMUM_CLASSES => {}
            (None, Some(_), Some(position))
                if position.is_finite() && (0.0..=1.0).contains(&position) => {}
            _ => {
                return Err(
                    "assignment must be null, classified, or continuous with a position in [0, 1]",
                );
            }
        }
        Ok(Self {
            feature_id: value.feature_id,
            class_index: value.class_index,
            color: value.color,
            ramp_position: value.ramp_position,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename = "ResolvedStylePlan")]
pub(super) struct UncheckedPlan {
    filter_outcomes: Vec<FilterOutcome>,
    assignments: Vec<FeatureStyleAssignment>,
    #[serde(deserialize_with = "deserialize_classes")]
    classes: Vec<StyleClass>,
    #[serde(deserialize_with = "deserialize_classes")]
    legend: Vec<StyleClass>,
    requested_class_count: Option<usize>,
    effective_class_count: usize,
}

impl TryFrom<UncheckedPlan> for ResolvedStylePlan {
    type Error = &'static str;

    fn try_from(value: UncheckedPlan) -> Result<Self, Self::Error> {
        let plan = Self {
            filter_outcomes: value.filter_outcomes,
            assignments: value.assignments,
            classes: value.classes,
            legend: value.legend,
            requested_class_count: value.requested_class_count,
            effective_class_count: value.effective_class_count,
        };
        validate_plan(&plan)?;
        Ok(plan)
    }
}

fn validate_plan(plan: &ResolvedStylePlan) -> Result<(), &'static str> {
    if plan.filter_outcomes.is_empty() || plan.assignments.is_empty() {
        return Err("resolved plans require input outcomes and selected assignments");
    }
    if plan.effective_class_count != plan.classes.len() || plan.legend != plan.classes {
        return Err("effective class count and legend must match classes");
    }
    if plan
        .requested_class_count
        .is_some_and(|count| !(1..=MAXIMUM_CLASSES).contains(&count))
    {
        return Err("requested class count must be in 1..=4096");
    }
    let numeric = plan
        .classes
        .first()
        .is_some_and(|class| class.lower_bound.is_some());
    if numeric != plan.requested_class_count.is_some() {
        return Err("only numerical classes have a requested class count");
    }
    for (index, class) in plan.classes.iter().enumerate() {
        if class.index != index || class.lower_bound.is_some() != numeric {
            return Err("classes must have sequential indices and a consistent kind");
        }
        if let (Some(lower), Some(upper)) = (class.lower_bound, class.upper_bound)
            && index > 0
            && (Some(lower) != plan.classes[index - 1].upper_bound || lower >= upper)
        {
            return Err("numerical classes must have contiguous, increasing bounds");
        }
    }
    let mut identities = BTreeSet::new();
    for outcome in &plan.filter_outcomes {
        if !identities.insert(outcome.feature_id.as_str()) {
            return Err("filter outcomes must have unique feature identities");
        }
    }
    let selected = plan
        .filter_outcomes
        .iter()
        .filter(|outcome| outcome.included);
    if selected.clone().count() != plan.assignments.len() {
        return Err("assignments must cover exactly the selected features");
    }
    for (outcome, assignment) in selected.zip(&plan.assignments) {
        if outcome.feature_id != assignment.feature_id {
            return Err("assignments must preserve selected feature identity and order");
        }
        if assignment.color.is_none() {
            continue;
        }
        if plan.classes.is_empty() {
            if assignment.ramp_position.is_none() {
                return Err("unclassified colored assignments require continuous positions");
            }
        } else {
            let class = assignment
                .class_index
                .and_then(|index| plan.classes.get(index));
            if class.is_none_or(|class| Some(class.color) != assignment.color) {
                return Err("classified assignments must reference their class and color");
            }
        }
    }
    if !plan
        .assignments
        .iter()
        .any(|assignment| assignment.color.is_some())
    {
        return Err("resolved plans require at least one styled value");
    }
    Ok(())
}

fn deserialize_classes<'de, D>(deserializer: D) -> Result<Vec<StyleClass>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ClassesVisitor;

    impl<'de> Visitor<'de> for ClassesVisitor {
        type Value = Vec<StyleClass>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("at most 4096 class entries")
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            // Power-of-two capacity keeps Vec's growth within the class limit.
            let capacity = sequence
                .size_hint()
                .unwrap_or(0)
                .min(MAXIMUM_CLASSES)
                .next_power_of_two();
            let mut classes = Vec::with_capacity(capacity);
            while let Some(class) = sequence.next_element()? {
                if classes.len() == MAXIMUM_CLASSES {
                    return Err(de::Error::custom(
                        "class collections cannot exceed 4096 entries",
                    ));
                }
                classes.push(class);
            }
            Ok(classes)
        }
    }

    deserializer.deserialize_seq(ClassesVisitor)
}
