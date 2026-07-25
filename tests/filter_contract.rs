use std::collections::BTreeMap;

use attribute_styling::{
    AttributeValue, Comparison, ComparisonOperator, FeatureRecord, FilterExpression, StylingError,
    evaluate_filter,
};

fn feature(id: &str, values: [(&str, AttributeValue); 3]) -> FeatureRecord {
    FeatureRecord::new(
        id,
        values
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect::<BTreeMap<_, _>>(),
    )
    .expect("valid feature")
}

#[test]
fn null_comparison_membership_and_boolean_composition_are_explicit() {
    let curve = feature(
        "curve-7",
        [
            ("score", AttributeValue::try_f64(0.75).expect("finite")),
            ("kind", AttributeValue::Text("edge".to_owned())),
            ("reviewed", AttributeValue::Null),
        ],
    );
    let filter = FilterExpression::And(vec![
        FilterExpression::Compare(Comparison::new(
            "score",
            ComparisonOperator::GreaterThanOrEqual,
            AttributeValue::try_f64(0.5).expect("finite"),
        )),
        FilterExpression::In {
            attribute: "kind".to_owned(),
            values: vec![
                AttributeValue::Text("edge".to_owned()),
                AttributeValue::Text("ridge".to_owned()),
            ],
        },
        FilterExpression::IsNull {
            attribute: "reviewed".to_owned(),
        },
        FilterExpression::Not(Box::new(FilterExpression::IsNull {
            attribute: "score".to_owned(),
        })),
    ]);

    assert!(evaluate_filter(&curve, &filter).expect("compatible filter"));
}

#[test]
fn missing_attributes_and_incompatible_comparisons_fail_closed() {
    let curve = feature(
        "curve-7",
        [
            ("score", AttributeValue::try_f64(0.75).expect("finite")),
            ("kind", AttributeValue::Text("edge".to_owned())),
            ("reviewed", AttributeValue::Null),
        ],
    );

    assert_eq!(
        evaluate_filter(
            &curve,
            &FilterExpression::IsNull {
                attribute: "missing".to_owned(),
            },
        ),
        Err(StylingError::UnknownAttribute("missing".to_owned()))
    );
    assert_eq!(
        evaluate_filter(
            &curve,
            &FilterExpression::Compare(Comparison::new(
                "kind",
                ComparisonOperator::GreaterThan,
                AttributeValue::Signed(1),
            )),
        ),
        Err(StylingError::IncompatibleTypes)
    );
}

#[test]
fn empty_boolean_groups_are_invalid() {
    let curve = feature(
        "curve-7",
        [
            ("score", AttributeValue::try_f64(0.75).expect("finite")),
            ("kind", AttributeValue::Text("edge".to_owned())),
            ("reviewed", AttributeValue::Null),
        ],
    );

    assert_eq!(
        evaluate_filter(&curve, &FilterExpression::And(Vec::new())),
        Err(StylingError::EmptyBooleanExpression)
    );
    assert_eq!(
        evaluate_filter(&curve, &FilterExpression::Or(Vec::new())),
        Err(StylingError::EmptyBooleanExpression)
    );
}
