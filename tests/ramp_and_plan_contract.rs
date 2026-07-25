use std::collections::BTreeMap;

use attribute_styling::{
    AttributeValue, Classification, Classifier, ColorRamp, ColorStop, Comparison,
    ComparisonOperator, FeatureRecord, FilterExpression, Rgba, StyleSpec, StylingError,
    resolve_style,
};

#[test]
fn custom_ramp_interpolates_endpoints_and_reversal() {
    let forward = ColorRamp::Custom {
        stops: vec![
            ColorStop::new(0.0, Rgba::new(0, 10, 20, 255)).expect("stop"),
            ColorStop::new(1.0, Rgba::new(100, 110, 120, 155)).expect("stop"),
        ],
        reversed: false,
    };
    assert_eq!(
        forward.sample(0.0).expect("start"),
        Rgba::new(0, 10, 20, 255)
    );
    assert_eq!(
        forward.sample(0.5).expect("middle"),
        Rgba::new(50, 60, 70, 205)
    );
    assert_eq!(
        forward.sample(1.0).expect("end"),
        Rgba::new(100, 110, 120, 155)
    );

    let reversed = ColorRamp::Custom {
        stops: match forward {
            ColorRamp::Custom { stops, .. } => stops,
            ColorRamp::Viridis { .. } | ColorRamp::BuiltIn { .. } | ColorRamp::Fixed { .. } => {
                unreachable!()
            }
        },
        reversed: true,
    };
    assert_eq!(
        reversed.sample(0.0).expect("reversed start"),
        Rgba::new(100, 110, 120, 155)
    );
}

#[test]
fn viridis_endpoints_are_pinned_behind_crate_owned_rgba() {
    let ramp = ColorRamp::Viridis { reversed: false };
    assert_eq!(ramp.sample(0.0).expect("start"), Rgba::new(68, 1, 84, 255));
    assert_eq!(ramp.sample(1.0).expect("end"), Rgba::new(253, 231, 37, 255));
}

#[test]
fn custom_ramp_validation_rejects_bad_positions_and_order() {
    assert_eq!(
        ColorStop::new(-0.1, Rgba::new(0, 0, 0, 255)),
        Err(StylingError::InvalidRampPosition)
    );
    let ramp = ColorRamp::Custom {
        stops: vec![
            ColorStop::new(0.5, Rgba::new(0, 0, 0, 255)).expect("stop"),
            ColorStop::new(0.5, Rgba::new(255, 255, 255, 255)).expect("stop"),
        ],
        reversed: false,
    };
    assert_eq!(ramp.sample(0.5), Err(StylingError::UnorderedRampStops));
}

#[test]
fn filtered_features_remain_in_outcomes_but_not_assignments() {
    let input = [1.0, 2.0, 3.0]
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            FeatureRecord::new(
                format!("curve-{index}"),
                BTreeMap::from([(
                    "length".to_owned(),
                    AttributeValue::try_f64(value).expect("finite"),
                )]),
            )
            .expect("feature")
        })
        .collect::<Vec<_>>();
    let spec = StyleSpec {
        filter: Some(FilterExpression::Compare(Comparison::new(
            "length",
            ComparisonOperator::GreaterThan,
            AttributeValue::try_f64(1.0).expect("finite"),
        ))),
        classification: Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::Quantile { classes: 2 },
        },
        ramp: ColorRamp::Viridis { reversed: false },
    };

    let first = resolve_style(&input, &spec).expect("plan");
    let second = resolve_style(&input, &spec).expect("same plan");

    assert_eq!(first, second);
    assert_eq!(
        first
            .filter_outcomes()
            .iter()
            .map(attribute_styling::FilterOutcome::included)
            .collect::<Vec<_>>(),
        vec![false, true, true]
    );
    assert_eq!(
        first
            .assignments()
            .iter()
            .map(attribute_styling::FeatureStyleAssignment::feature_id)
            .collect::<Vec<_>>(),
        vec!["curve-1", "curve-2"]
    );
    assert_eq!(first.legend(), first.classes());
}

#[test]
fn continuous_classification_samples_numeric_extent_without_classes() {
    let input = [0.0, 5.0, 10.0]
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            FeatureRecord::new(
                format!("curve-{index}"),
                BTreeMap::from([(
                    "score".to_owned(),
                    AttributeValue::try_f64(value).expect("finite"),
                )]),
            )
            .expect("feature")
        })
        .collect::<Vec<_>>();
    let plan = resolve_style(
        &input,
        &StyleSpec {
            filter: None,
            classification: Classification::Continuous {
                attribute: "score".to_owned(),
            },
            ramp: ColorRamp::Viridis { reversed: false },
        },
    )
    .expect("continuous plan");

    assert_eq!(plan.requested_class_count(), None);
    assert_eq!(plan.effective_class_count(), 0);
    assert!(plan.classes().is_empty());
    assert_eq!(plan.assignments()[0].ramp_position(), Some(0.0));
    assert_eq!(plan.assignments()[1].ramp_position(), Some(0.5));
    assert_eq!(plan.assignments()[2].ramp_position(), Some(1.0));
}
