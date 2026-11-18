#![allow(clippy::float_cmp)]
use attribute_styling::{AttributeValue, FeatureRecord, StyleSpec, resolve_style};
use serde_json::json;

#[allow(clippy::needless_pass_by_value)]
fn resolve(
    values: &[Option<f64>],
    domain: serde_json::Value,
    transform: &str,
) -> attribute_styling::ResolvedStylePlan {
    let records = values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            FeatureRecord::new(
                i.to_string(),
                [(
                    "x".to_owned(),
                    v.map_or(AttributeValue::Null, |v| {
                        AttributeValue::try_f64(v).unwrap()
                    }),
                )]
                .into(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let spec: StyleSpec = serde_json::from_value(json!({"filter":null,"classification":{"kind":"continuous_normalized","attribute":"x","normalization":{"domain":domain,"transform":transform}},"ramp":{"kind":"viridis","reversed":false}})).unwrap();
    resolve_style(&records, &spec).unwrap()
}

#[test]
fn explicit_domain_clips_without_dropping_nulls() {
    let plan = resolve(
        &[Some(-2.0), Some(5.0), Some(20.0), None],
        json!({"kind":"explicit","minimum":0.0,"maximum":10.0}),
        "linear",
    );
    assert_eq!(
        plan.assignments()
            .iter()
            .map(attribute_styling::FeatureStyleAssignment::ramp_position)
            .collect::<Vec<_>>(),
        vec![Some(0.0), Some(0.5), Some(1.0), None]
    );
}
#[test]
fn logarithmic_observed_domain_uses_geometric_midpoint() {
    let plan = resolve(
        &[Some(1.0), Some(10.0), Some(100.0)],
        json!({"kind":"observed"}),
        "log",
    );
    assert!((plan.assignments()[1].ramp_position().unwrap() - 0.5).abs() < 1e-14);
}
#[test]
fn percentile_trim_interpolates_and_clips_tails() {
    let plan = resolve(
        &[Some(0.0), Some(10.0), Some(20.0), Some(30.0)],
        json!({"kind":"percentile_trim","lower":0.25,"upper":0.75}),
        "linear",
    );
    assert!((plan.assignments()[1].ramp_position().unwrap() - 1.0 / 6.0).abs() < 1e-14);
}
#[test]
fn sample_sigma_domain_maps_mean_to_midpoint() {
    let plan = resolve(
        &[Some(0.0), Some(2.0), Some(4.0)],
        json!({"kind":"standard_deviation","deviations":1.0}),
        "linear",
    );
    assert_eq!(plan.assignments()[1].ramp_position(), Some(0.5));
    assert_eq!(plan.assignments()[0].ramp_position(), Some(0.0));
}

#[test]
fn sample_sigma_rejects_single_observation() {
    let records = vec![
        FeatureRecord::new("one", [("x".to_owned(), AttributeValue::Signed(2))].into()).unwrap(),
    ];
    let spec:StyleSpec=serde_json::from_value(json!({"filter":null,"classification":{"kind":"continuous_normalized","attribute":"x","normalization":{"domain":{"kind":"standard_deviation","deviations":1.0},"transform":"linear"}},"ramp":{"kind":"viridis","reversed":false}})).unwrap();
    assert!(resolve_style(&records, &spec).is_err());
}

#[test]
fn frozen_metadata_has_real_ticks_and_assignment_colors() {
    let plan = resolve(
        &[Some(1.0), Some(10.0), Some(100.0)],
        json!({"kind":"observed"}),
        "log",
    );
    let metadata = plan.normalization().unwrap();
    assert_eq!(metadata.minimum(), 1.0);
    assert_eq!(metadata.maximum(), 100.0);
    assert!((metadata.value_at(0.5).unwrap() - 10.0).abs() < 1e-12);
    for assignment in plan.assignments() {
        assert_eq!(
            Some(
                metadata
                    .sample_color(assignment.ramp_position().unwrap())
                    .unwrap()
            ),
            assignment.color()
        );
    }
    let encoded = serde_json::to_value(&plan).unwrap();
    assert_eq!(
        serde_json::from_value::<attribute_styling::ResolvedStylePlan>(encoded.clone()).unwrap(),
        plan
    );
    let mut bad = encoded.clone();
    bad["normalization"]["minimum"] = json!(-1.0);
    assert!(serde_json::from_value::<attribute_styling::ResolvedStylePlan>(bad).is_err());
    let mut bad = encoded;
    bad["normalization"]["ramp"]["reversed"] = json!(true);
    assert!(serde_json::from_value::<attribute_styling::ResolvedStylePlan>(bad).is_err());
    assert!(metadata.position(0.0).is_err());
    assert!(metadata.position(f64::NAN).is_err());
    assert!(metadata.value_at(1.1).is_err());
}

#[test]
fn extreme_finite_linear_and_log_domains_remain_finite() {
    let plan = resolve(
        &[Some(-f64::MAX), Some(0.0), Some(f64::MAX)],
        json!({"kind":"observed"}),
        "linear",
    );
    assert_eq!(plan.assignments()[1].ramp_position(), Some(0.5));
    assert_eq!(plan.normalization().unwrap().value_at(0.5).unwrap(), 0.0);
    let plan = resolve(
        &[Some(f64::from_bits(1)), Some(1.0), Some(f64::MAX)],
        json!({"kind":"observed"}),
        "log",
    );
    let metadata = plan.normalization().unwrap();
    for p in [0.0, 0.25, 0.5, 0.75, 1.0] {
        assert!(metadata.value_at(p).unwrap().is_finite());
    }
    let plan = resolve(
        &[Some(1.0), Some(f64::from_bits(1.0_f64.to_bits() + 1))],
        json!({"kind":"observed"}),
        "log",
    );
    assert_eq!(plan.assignments()[1].ramp_position(), Some(1.0));
}

#[test]
fn tied_percentiles_are_midpoint_and_signed_zero_is_canonical() {
    let plan = resolve(
        &[Some(-0.0), Some(0.0), Some(0.0)],
        json!({"kind":"percentile_trim","lower":0.02,"upper":0.98}),
        "linear",
    );
    assert!(
        plan.assignments()
            .iter()
            .all(|a| a.ramp_position() == Some(0.5))
    );
    assert_eq!(
        plan.normalization().unwrap().minimum().to_bits(),
        0.0_f64.to_bits()
    );
}

#[test]
fn legacy_continuous_serialization_has_no_new_field() {
    let records = vec![
        FeatureRecord::new("one", [("x".to_owned(), AttributeValue::Signed(1))].into()).unwrap(),
    ];
    let spec:StyleSpec=serde_json::from_value(json!({"filter":null,"classification":{"kind":"continuous","attribute":"x"},"ramp":{"kind":"viridis","reversed":false}})).unwrap();
    let plan = resolve_style(&records, &spec).unwrap();
    assert!(
        serde_json::to_value(&plan)
            .unwrap()
            .get("normalization")
            .is_none()
    );
    assert!(plan.normalization().is_none());
}

#[test]
fn invalid_policies_and_observations_fail_closed() {
    use attribute_styling::{
        Classification, ColorRamp, NumericDomain, NumericNormalization, NumericTransform,
        StylingError,
    };
    let records = vec![
        FeatureRecord::new("one", [("x".to_owned(), AttributeValue::Signed(0))].into()).unwrap(),
        FeatureRecord::new("two", [("x".to_owned(), AttributeValue::Signed(1))].into()).unwrap(),
    ];
    for domain in [
        NumericDomain::Explicit {
            minimum: 2.0,
            maximum: 1.0,
        },
        NumericDomain::Explicit {
            minimum: f64::NAN,
            maximum: 1.0,
        },
        NumericDomain::PercentileTrim {
            lower: 0.8,
            upper: 0.2,
        },
        NumericDomain::PercentileTrim {
            lower: -0.1,
            upper: 0.9,
        },
        NumericDomain::PercentileTrim {
            lower: 0.0,
            upper: f64::INFINITY,
        },
        NumericDomain::StandardDeviation { deviations: 0.0 },
        NumericDomain::StandardDeviation {
            deviations: f64::NAN,
        },
    ] {
        let spec = StyleSpec {
            filter: None,
            classification: Classification::ContinuousNormalized {
                attribute: "x".to_owned(),
                normalization: NumericNormalization {
                    domain,
                    transform: NumericTransform::Linear,
                },
            },
            ramp: ColorRamp::Viridis { reversed: false },
        };
        assert_eq!(
            resolve_style(&records, &spec),
            Err(StylingError::InvalidNormalization)
        );
    }
    let mut spec = StyleSpec {
        filter: None,
        classification: Classification::ContinuousNormalized {
            attribute: "x".to_owned(),
            normalization: NumericNormalization {
                domain: NumericDomain::Explicit {
                    minimum: 1.0,
                    maximum: 2.0,
                },
                transform: NumericTransform::Log,
            },
        },
        ramp: ColorRamp::Viridis { reversed: false },
    };
    assert_eq!(
        resolve_style(&records, &spec),
        Err(StylingError::NonPositiveLogValue)
    );
    spec.classification = Classification::ContinuousNormalized {
        attribute: "x".to_owned(),
        normalization: NumericNormalization {
            domain: NumericDomain::StandardDeviation {
                deviations: f64::MAX,
            },
            transform: NumericTransform::Linear,
        },
    };
    let extreme = vec![
        FeatureRecord::new(
            "min",
            [("x".to_owned(), AttributeValue::try_f64(-f64::MAX).unwrap())].into(),
        )
        .unwrap(),
        FeatureRecord::new(
            "max",
            [("x".to_owned(), AttributeValue::try_f64(f64::MAX).unwrap())].into(),
        )
        .unwrap(),
    ];
    assert_eq!(
        resolve_style(&extreme, &spec),
        Err(StylingError::InvalidNormalization)
    );
    let null =
        vec![FeatureRecord::new("null", [("x".to_owned(), AttributeValue::Null)].into()).unwrap()];
    assert_eq!(
        resolve_style(&null, &spec),
        Err(StylingError::EmptyNumericInput)
    );
}

#[test]
fn filtering_precedes_domain_discovery_and_keeps_order() {
    use attribute_styling::{
        Classification, ColorRamp, Comparison, ComparisonOperator, FilterExpression, NumericDomain,
        NumericNormalization, NumericTransform,
    };
    let records = [100, 2, 4]
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            FeatureRecord::new(
                i.to_string(),
                [("x".to_owned(), AttributeValue::Signed(v))].into(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let spec = StyleSpec {
        filter: Some(FilterExpression::Compare(Comparison::new(
            "x",
            ComparisonOperator::LessThan,
            AttributeValue::Signed(10),
        ))),
        classification: Classification::ContinuousNormalized {
            attribute: "x".to_owned(),
            normalization: NumericNormalization {
                domain: NumericDomain::Observed,
                transform: NumericTransform::Linear,
            },
        },
        ramp: ColorRamp::Viridis { reversed: true },
    };
    let plan = resolve_style(&records, &spec).unwrap();
    assert_eq!(plan.normalization().unwrap().maximum(), 4.0);
    assert_eq!(plan.assignments()[0].feature_id(), "1");
    assert_eq!(plan.assignments()[1].feature_id(), "2");
    assert!(!plan.filter_outcomes()[0].included());
    assert_eq!(
        plan.assignments()[0].color(),
        Some(plan.normalization().unwrap().sample_color(0.0).unwrap())
    );
}

#[test]
fn normalized_metadata_rejects_tampered_policy_and_discrete_shape() {
    let plan = resolve(
        &[Some(0.0), Some(5.0), Some(10.0)],
        json!({"kind":"explicit","minimum":0.0,"maximum":10.0}),
        "linear",
    );
    let encoded = serde_json::to_value(&plan).unwrap();
    for (key, value) in [("minimum", json!(1.0)), ("maximum", json!(-1.0))] {
        let mut bad = encoded.clone();
        bad["normalization"][key] = value;
        assert!(serde_json::from_value::<attribute_styling::ResolvedStylePlan>(bad).is_err());
    }
    let mut bad = encoded.clone();
    bad["normalization"]["policy"]["domain"] =
        json!({"kind":"percentile_trim","lower":0.9,"upper":0.1});
    assert!(serde_json::from_value::<attribute_styling::ResolvedStylePlan>(bad).is_err());
    let records = vec![
        FeatureRecord::new("one", [("x".to_owned(), AttributeValue::Signed(1))].into()).unwrap(),
    ];
    let spec:StyleSpec=serde_json::from_value(json!({"filter":null,"classification":{"kind":"single"},"ramp":{"kind":"viridis","reversed":false}})).unwrap();
    let mut bad = serde_json::to_value(resolve_style(&records, &spec).unwrap()).unwrap();
    bad["normalization"] = encoded["normalization"].clone();
    assert!(serde_json::from_value::<attribute_styling::ResolvedStylePlan>(bad).is_err());
}

#[test]
fn adjacent_extreme_sigma_values_do_not_silently_collapse() {
    use attribute_styling::{
        Classification, ColorRamp, NumericDomain, NumericNormalization, NumericTransform,
    };
    let records = [f64::from_bits(f64::MAX.to_bits() - 1), f64::MAX]
        .into_iter()
        .enumerate()
        .map(|(i, v)| {
            FeatureRecord::new(
                i.to_string(),
                [("x".to_owned(), AttributeValue::try_f64(v).unwrap())].into(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let spec = StyleSpec {
        filter: None,
        classification: Classification::ContinuousNormalized {
            attribute: "x".to_owned(),
            normalization: NumericNormalization {
                domain: NumericDomain::StandardDeviation { deviations: 1.0 },
                transform: NumericTransform::Linear,
            },
        },
        ramp: ColorRamp::Viridis { reversed: false },
    };
    // Exact mean + sample sigma exceeds f64::MAX: refuse rather than invent ties.
    assert!(resolve_style(&records, &spec).is_err());
}

#[test]
fn adjacent_representable_sigma_values_retain_variation() {
    let plan = resolve(
        &[Some(1.0), Some(f64::from_bits(1.0_f64.to_bits() + 1))],
        json!({"kind":"standard_deviation","deviations":2.0}),
        "linear",
    );
    assert!(plan.normalization().unwrap().minimum() < plan.normalization().unwrap().maximum());
    assert!(plan.assignments()[0].ramp_position() < plan.assignments()[1].ramp_position());
}
