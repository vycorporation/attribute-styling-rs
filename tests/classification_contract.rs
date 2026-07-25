use std::collections::BTreeMap;

use attribute_styling::{
    AttributeValue, Classification, Classifier, ColorRamp, FeatureRecord, Rgba, StyleSpec,
    StylingError, resolve_style,
};

fn records(values: &[f64]) -> Vec<FeatureRecord> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            FeatureRecord::new(
                format!("curve-{index}"),
                BTreeMap::from([(
                    "length".to_owned(),
                    AttributeValue::try_f64(*value).expect("finite"),
                )]),
            )
            .expect("valid feature")
        })
        .collect()
}

fn viridis(classification: Classification) -> StyleSpec {
    StyleSpec {
        filter: None,
        classification,
        ramp: ColorRamp::Viridis { reversed: false },
    }
}

#[test]
fn equal_interval_uses_inclusive_upper_bounds_and_complete_assignment() {
    let plan = resolve_style(
        &records(&[0.0, 2.5, 5.0, 7.5, 10.0]),
        &viridis(Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::EqualInterval { classes: 4 },
        }),
    )
    .expect("equal interval plan");

    assert_eq!(plan.requested_class_count(), Some(4));
    assert_eq!(plan.effective_class_count(), 4);
    assert_eq!(
        plan.classes()
            .iter()
            .map(attribute_styling::StyleClass::upper_bound)
            .collect::<Vec<_>>(),
        vec![Some(2.5), Some(5.0), Some(7.5), Some(10.0)]
    );
    assert_eq!(
        plan.assignments()
            .iter()
            .map(attribute_styling::FeatureStyleAssignment::class_index)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(0), Some(1), Some(2), Some(3)]
    );
}

#[test]
fn quantile_keeps_ties_together_and_records_effective_classes() {
    let plan = resolve_style(
        &records(&[1.0, 1.0, 1.0, 2.0, 3.0, 4.0]),
        &viridis(Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::Quantile { classes: 4 },
        }),
    )
    .expect("quantile plan");

    assert_eq!(plan.requested_class_count(), Some(4));
    assert_eq!(plan.effective_class_count(), 3);
    assert_eq!(
        plan.classes()
            .iter()
            .map(attribute_styling::StyleClass::upper_bound)
            .collect::<Vec<_>>(),
        vec![Some(1.0), Some(2.0), Some(4.0)]
    );
    assert_eq!(
        plan.assignments()
            .iter()
            .map(attribute_styling::FeatureStyleAssignment::class_index)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(0), Some(0), Some(1), Some(2), Some(2)]
    );
}

#[test]
fn manual_breaks_are_strict_and_must_cover_selected_values() {
    let invalid = resolve_style(
        &records(&[1.0, 2.0]),
        &viridis(Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::Manual {
                upper_bounds: vec![1.0, 1.0, 2.0],
            },
        }),
    );
    assert_eq!(invalid, Err(StylingError::UnorderedManualBreaks));

    let uncovered = resolve_style(
        &records(&[1.0, 3.0]),
        &viridis(Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::Manual {
                upper_bounds: vec![1.0, 2.0],
            },
        }),
    );
    assert_eq!(uncovered, Err(StylingError::ManualBreaksDoNotCoverValues));
}

#[test]
fn degenerate_numeric_data_resolves_to_one_effective_class() {
    let plan = resolve_style(
        &records(&[4.0, 4.0, 4.0]),
        &viridis(Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::EqualInterval { classes: 5 },
        }),
    )
    .expect("degenerate plan");

    assert_eq!(plan.requested_class_count(), Some(5));
    assert_eq!(plan.effective_class_count(), 1);
    assert_eq!(plan.classes()[0].upper_bound(), Some(4.0));
}

#[test]
fn nulls_are_excluded_and_recorded_without_a_class_assignment() {
    let mut input = records(&[1.0, 2.0]);
    input.push(
        FeatureRecord::new(
            "curve-null",
            BTreeMap::from([("length".to_owned(), AttributeValue::Null)]),
        )
        .expect("valid feature"),
    );

    let plan = resolve_style(
        &input,
        &viridis(Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::Quantile { classes: 2 },
        }),
    )
    .expect("null-aware plan");

    assert_eq!(plan.filter_outcomes().len(), 3);
    assert!(plan.filter_outcomes()[2].included());
    assert_eq!(plan.assignments()[2].class_index(), None);
    assert_eq!(plan.assignments()[2].color(), None);
}

#[test]
fn empty_selected_input_and_zero_classes_fail_explicitly() {
    assert_eq!(
        resolve_style(
            &[],
            &viridis(Classification::Numeric {
                attribute: "length".to_owned(),
                classifier: Classifier::EqualInterval { classes: 4 },
            }),
        ),
        Err(StylingError::EmptyInput)
    );
    assert_eq!(
        resolve_style(
            &records(&[1.0]),
            &viridis(Classification::Numeric {
                attribute: "length".to_owned(),
                classifier: Classifier::Quantile { classes: 0 },
            }),
        ),
        Err(StylingError::ZeroClasses)
    );
    assert_eq!(
        resolve_style(
            &records(&[1.0]),
            &viridis(Classification::Numeric {
                attribute: "length".to_owned(),
                classifier: Classifier::EqualInterval { classes: 4097 },
            }),
        ),
        Err(StylingError::TooManyClasses {
            requested: 4097,
            maximum: 4096,
        })
    );
}

#[test]
fn duplicate_stable_feature_identities_are_rejected() {
    let input = vec![records(&[1.0])[0].clone(), records(&[2.0])[0].clone()];
    assert_eq!(
        resolve_style(&input, &viridis(Classification::Single)),
        Err(StylingError::DuplicateFeatureId("curve-0".to_owned()))
    );
}

#[test]
fn categorical_and_single_classification_are_deterministic() {
    let input = vec![
        FeatureRecord::new(
            "b",
            BTreeMap::from([("kind".to_owned(), AttributeValue::Text("ridge".to_owned()))]),
        )
        .expect("valid feature"),
        FeatureRecord::new(
            "a",
            BTreeMap::from([("kind".to_owned(), AttributeValue::Text("edge".to_owned()))]),
        )
        .expect("valid feature"),
    ];

    let categorical = resolve_style(
        &input,
        &viridis(Classification::Categorical {
            attribute: "kind".to_owned(),
        }),
    )
    .expect("categorical");
    assert_eq!(
        categorical
            .classes()
            .iter()
            .map(attribute_styling::StyleClass::label)
            .collect::<Vec<_>>(),
        vec!["edge", "ridge"]
    );
    assert_eq!(
        categorical
            .assignments()
            .iter()
            .map(attribute_styling::FeatureStyleAssignment::class_index)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(0)]
    );

    let single = resolve_style(
        &input,
        &StyleSpec {
            filter: None,
            classification: Classification::Single,
            ramp: ColorRamp::Custom {
                stops: vec![
                    attribute_styling::ColorStop::new(0.0, Rgba::new(12, 34, 56, 255))
                        .expect("stop"),
                ],
                reversed: false,
            },
        },
    )
    .expect("single");
    assert_eq!(single.effective_class_count(), 1);
    assert_eq!(
        single.assignments()[0].color(),
        Some(Rgba::new(12, 34, 56, 255))
    );
}
