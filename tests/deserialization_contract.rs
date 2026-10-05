use std::collections::BTreeMap;

use attribute_styling::{
    AttributeValue, Classification, Classifier, ColorRamp, FeatureRecord, ResolvedStylePlan,
    StyleSpec, resolve_style,
};
use serde_json::{Value, json};

fn plan(classification: Classification) -> ResolvedStylePlan {
    let records = [Some(10.0), Some(20.0), None]
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            FeatureRecord::new(
                index.to_string(),
                BTreeMap::from([(
                    "value".to_owned(),
                    value.map_or(AttributeValue::Null, |value| {
                        AttributeValue::try_f64(value).expect("finite")
                    }),
                )]),
            )
            .expect("feature")
        })
        .collect::<Vec<_>>();
    resolve_style(
        &records,
        &StyleSpec {
            filter: None,
            classification,
            ramp: ColorRamp::Viridis { reversed: false },
        },
    )
    .expect("valid plan")
}

fn numeric_plan() -> Value {
    serde_json::to_value(plan(Classification::Numeric {
        attribute: "value".to_owned(),
        classifier: Classifier::EqualInterval { classes: 2 },
    }))
    .expect("serialize")
}

#[test]
fn resolved_plan_deserialization_rejects_inconsistent_fields() {
    for (path, replacement) in [
        ("/effective_class_count", json!(77)),
        ("/requested_class_count", json!(0)),
        ("/requested_class_count", json!(4097)),
        ("/requested_class_count", Value::Null),
        ("/assignments/0/class_index", json!(999)),
        ("/assignments/0/feature_id", json!("ghost")),
        ("/assignments/0/feature_id", json!("")),
        ("/assignments/0/ramp_position", json!(2.0)),
        ("/assignments/0/color", Value::Null),
        ("/assignments/0/color/red", json!(17)),
        ("/assignments/0/ramp_position", json!(0.5)),
        (
            "/assignments/2/color",
            json!({"red":0,"green":0,"blue":0,"alpha":255}),
        ),
        ("/filter_outcomes/0/included", json!(false)),
        ("/filter_outcomes/1/feature_id", json!("0")),
        ("/filter_outcomes/0/feature_id", json!("")),
        ("/classes/0/index", json!(1)),
        ("/classes/0/lower_bound", json!(16.0)),
        ("/classes/0/upper_bound", Value::Null),
        ("/classes/0/label", json!("wrong interval")),
        ("/classes/1/lower_bound", json!(14.0)),
        ("/legend/0/label", json!("wrong legend")),
        ("/assignments", json!([])),
        ("/filter_outcomes", json!([])),
    ] {
        let mut value = numeric_plan();
        *value.pointer_mut(path).expect("existing field") = replacement.clone();
        if let Some(suffix) = path.strip_prefix("/classes/") {
            *value
                .pointer_mut(&format!("/legend/{suffix}"))
                .expect("matching legend") = replacement;
        }
        assert!(
            serde_json::from_value::<ResolvedStylePlan>(value).is_err(),
            "accepted {path}"
        );
    }
}

#[test]
fn valid_plans_round_trip_for_every_classification_family() {
    let mut classifications = vec![
        Classification::Single,
        Classification::Categorical {
            attribute: "value".to_owned(),
        },
        Classification::Continuous {
            attribute: "value".to_owned(),
        },
    ];
    for classifier in [
        Classifier::EqualInterval { classes: 2 },
        Classifier::Quantile { classes: 2 },
        Classifier::Pretty { classes: 2 },
        Classifier::Manual {
            upper_bounds: vec![10.0, 20.0, 30.0],
        },
    ] {
        classifications.push(Classification::Numeric {
            attribute: "value".to_owned(),
            classifier,
        });
    }
    for classification in classifications {
        let expected = plan(classification);
        let decoded: ResolvedStylePlan =
            serde_json::from_str(&serde_json::to_string(&expected).expect("serialize"))
                .expect("valid plan round trip");
        assert_eq!(decoded, expected);
    }
}

#[test]
fn resolved_plan_deserialization_bounds_class_and_legend_collections() {
    for field in ["classes", "legend"] {
        let mut value = numeric_plan();
        let class = value["classes"][0].clone();
        value[field] = json!(vec![class; 4097]);
        let error = serde_json::from_value::<ResolvedStylePlan>(value)
            .map(drop)
            .expect_err("class limit");
        assert!(error.to_string().contains("4096"), "{error}");
    }
}

#[test]
fn continuous_plans_reject_invalid_assignment_shapes_and_positions() {
    let expected = plan(Classification::Continuous {
        attribute: "value".to_owned(),
    });
    for (path, replacement) in [
        ("/assignments/0/ramp_position", json!(-0.1)),
        ("/assignments/0/ramp_position", json!(1.1)),
        ("/assignments/0/ramp_position", Value::Null),
        ("/assignments/0/class_index", json!(0)),
        ("/assignments/0/color", Value::Null),
        ("/requested_class_count", json!(1)),
    ] {
        let mut value = serde_json::to_value(&expected).expect("serialize");
        *value.pointer_mut(path).expect("existing field") = replacement;
        assert!(
            serde_json::from_value::<ResolvedStylePlan>(value).is_err(),
            "accepted {path}"
        );
    }
}

#[test]
fn filtered_plans_round_trip_and_reject_assignment_reordering() {
    let mut value = numeric_plan();
    value["filter_outcomes"][1]["included"] = json!(false);
    value["assignments"]
        .as_array_mut()
        .expect("assignments")
        .remove(1);
    let expected: ResolvedStylePlan =
        serde_json::from_value(value.clone()).expect("valid filtered plan");
    assert_eq!(serde_json::to_value(expected).expect("serialize"), value);
    value["assignments"]
        .as_array_mut()
        .expect("assignments")
        .swap(0, 1);
    assert!(serde_json::from_value::<ResolvedStylePlan>(value).is_err());
}

#[test]
fn maximum_class_count_round_trips_with_unused_manual_intervals() {
    let expected = plan(Classification::Numeric {
        attribute: "value".to_owned(),
        classifier: Classifier::Manual {
            upper_bounds: (10..4106).map(f64::from).collect(),
        },
    });
    let decoded: ResolvedStylePlan =
        serde_json::from_value(serde_json::to_value(&expected).expect("serialize"))
            .expect("4096 classes remain supported");
    assert_eq!(decoded, expected);
}

#[test]
fn independently_deserialized_result_entries_validate_local_invariants() {
    let value = numeric_plan();
    let mut outcome = value["filter_outcomes"][0].clone();
    outcome["feature_id"] = json!("");
    assert!(serde_json::from_value::<attribute_styling::FilterOutcome>(outcome).is_err());

    let mut class = value["classes"][0].clone();
    class["lower_bound"] = json!(100.0);
    assert!(serde_json::from_value::<attribute_styling::StyleClass>(class).is_err());

    let mut assignment = value["assignments"][0].clone();
    assignment["ramp_position"] = json!(2.0);
    assert!(
        serde_json::from_value::<attribute_styling::FeatureStyleAssignment>(assignment).is_err()
    );
}
