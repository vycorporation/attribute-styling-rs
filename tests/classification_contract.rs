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
fn manual_breaks_reject_inverted_first_interval() {
    let result = resolve_style(
        &records(&[10.0, 20.0]),
        &viridis(Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::Manual {
                upper_bounds: vec![0.0, 20.0],
            },
        }),
    );
    assert_eq!(result, Err(StylingError::ManualBreaksDoNotCoverValues));

    let plan = resolve_style(
        &records(&[10.0, 20.0]),
        &viridis(Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::Manual {
                upper_bounds: vec![10.0, 20.0],
            },
        }),
    )
    .expect("the first bound may equal the minimum");
    assert_eq!(plan.classes()[0].label(), "[10, 10]");
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

#[test]
fn categorical_float_identity_is_lossless_and_independent_of_input_order() {
    let values = [
        1.0,
        1.0 + f64::EPSILON,
        -1.0,
        -1.0 - f64::EPSILON,
        f64::from_bits(1),
        f64::from_bits(2),
        f64::MAX.next_down(),
        f64::MAX,
    ];
    let mut input = records(&values);
    let style = viridis(Classification::Categorical {
        attribute: "length".to_owned(),
    });
    let first = resolve_style(&input, &style).expect("distinct float categories");
    assert_eq!(first.effective_class_count(), 8);
    let indices = first
        .assignments()
        .iter()
        .map(|a| a.class_index().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(indices.len(), 8);
    input.reverse();
    let reversed = resolve_style(&input, &style).expect("permuted float categories");
    assert_eq!(first.classes(), reversed.classes());
    for assignment in first.assignments() {
        assert_eq!(
            Some(assignment),
            reversed
                .assignments()
                .iter()
                .find(|a| a.feature_id() == assignment.feature_id())
        );
    }
}

#[test]
fn categorical_and_numeric_classifiers_share_numerical_signed_zero_equality() {
    for classification in [
        Classification::Categorical {
            attribute: "length".to_owned(),
        },
        Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::EqualInterval { classes: 3 },
        },
        Classification::Numeric {
            attribute: "length".to_owned(),
            classifier: Classifier::Quantile { classes: 3 },
        },
    ] {
        let style = viridis(classification);
        let input = records(&[-0.0, 0.0]);
        let first = resolve_style(&input, &style).expect("one zero value");
        assert_eq!(first.effective_class_count(), 1);
        assert!(
            first
                .assignments()
                .iter()
                .all(|a| a.class_index() == Some(0))
        );
        let reverse =
            resolve_style(&[input[1].clone(), input[0].clone()], &style).expect("same zero value");
        assert_eq!(first.classes(), reverse.classes());
    }
}

#[test]
fn manual_signed_zeros_are_duplicate_bounds() {
    for upper_bounds in [vec![-0.0, 0.0], vec![0.0, -0.0]] {
        assert_eq!(
            resolve_style(
                &records(&[0.0]),
                &viridis(Classification::Numeric {
                    attribute: "length".to_owned(),
                    classifier: Classifier::Manual { upper_bounds },
                })
            ),
            Err(StylingError::UnorderedManualBreaks)
        );
    }
}

#[test]
fn manual_and_categorical_class_counts_are_bounded() {
    for count in [4096, 4097] {
        let values = (0..count).map(f64::from).collect::<Vec<_>>();
        let manual = resolve_style(
            &records(&[0.0, 1.0]),
            &viridis(Classification::Numeric {
                attribute: "length".to_owned(),
                classifier: Classifier::Manual {
                    upper_bounds: values.clone(),
                },
            }),
        );
        let categorical = resolve_style(
            &records(&values),
            &viridis(Classification::Categorical {
                attribute: "length".to_owned(),
            }),
        );
        for result in [manual, categorical] {
            if count == 4096 {
                assert_eq!(
                    result
                        .expect("maximum accepted count")
                        .effective_class_count(),
                    4096
                );
            } else {
                assert_eq!(
                    result.err(),
                    Some(StylingError::TooManyClasses {
                        requested: 4097,
                        maximum: 4096
                    })
                );
            }
        }
    }
}

#[test]
fn equal_interval_extreme_finite_values_have_finite_ordered_bounds_and_assignments() {
    for values in [
        vec![-f64::MAX, 0.0, f64::MAX],
        vec![0.0, f64::from_bits(1)],
        vec![1.0, 1.0 + f64::EPSILON],
        vec![f64::MAX.next_down(), f64::MAX],
    ] {
        let plan = resolve_style(
            &records(&values),
            &viridis(Classification::Numeric {
                attribute: "length".to_owned(),
                classifier: Classifier::EqualInterval { classes: 4 },
            }),
        )
        .expect("finite equal-interval plan");
        assert_eq!(plan.requested_class_count(), Some(4));
        assert!(plan.effective_class_count() <= 4);
        assert_eq!(plan.legend(), plan.classes());
        let bounds = plan
            .classes()
            .iter()
            .map(|c| c.upper_bound().unwrap())
            .collect::<Vec<_>>();
        assert!(bounds.iter().all(|bound| bound.is_finite()));
        assert!(bounds.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(bounds.last(), values.last());
        for (value, assignment) in values.iter().zip(plan.assignments()) {
            let index = assignment.class_index().expect("class for each value");
            assert!(*value <= bounds[index]);
            if index > 0 {
                assert!(*value > bounds[index - 1]);
            }
            assert_eq!(assignment.color(), Some(plan.classes()[index].color()));
        }
    }
}

#[test]
fn large_integer_categories_remain_exact_while_numeric_projection_is_rejected() {
    let input = [
        AttributeValue::Unsigned(u64::MAX),
        AttributeValue::Unsigned(u64::MAX - 1),
        AttributeValue::Signed(i64::MIN),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, value)| {
        FeatureRecord::new(
            index.to_string(),
            BTreeMap::from([("length".to_owned(), value)]),
        )
        .expect("feature")
    })
    .collect::<Vec<_>>();
    let categorical = resolve_style(
        &input,
        &viridis(Classification::Categorical {
            attribute: "length".to_owned(),
        }),
    )
    .expect("exact integer categories");
    assert_eq!(categorical.effective_class_count(), 3);
    assert_eq!(
        resolve_style(
            &input,
            &viridis(Classification::Numeric {
                attribute: "length".to_owned(),
                classifier: Classifier::Quantile { classes: 2 }
            })
        ),
        Err(StylingError::NumberOutsideExactF64Range)
    );
}

#[test]
fn nonzero_subnormal_interval_width_keeps_interior_bounds_inside_range() {
    let tiny = f64::from_bits;
    for values in [
        [0.0, tiny(9)],
        [-tiny(9), 0.0],
        [tiny(5), tiny(14)],
        [-tiny(14), -tiny(5)],
    ] {
        let plan = resolve_style(
            &records(&values),
            &viridis(Classification::Numeric {
                attribute: "length".to_owned(),
                classifier: Classifier::EqualInterval { classes: 6 },
            }),
        )
        .expect("six representable intervals within nine subnormal units");
        let bounds = plan
            .classes()
            .iter()
            .map(|class| class.upper_bound().expect("numeric class"))
            .collect::<Vec<_>>();
        assert_eq!(bounds.len(), 6);
        assert!(
            bounds
                .iter()
                .all(|bound| bound.is_finite() && *bound >= values[0] && *bound <= values[1])
        );
        assert!(bounds.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(bounds.last(), Some(&values[1]));
        assert!(
            plan.assignments()
                .iter()
                .all(|assignment| assignment.class_index().is_some())
        );
    }
}
