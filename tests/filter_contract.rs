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

#[test]
fn boolean_short_circuit_cannot_hide_invalid_operands() {
    let curve = feature(
        "curve",
        [
            ("x", AttributeValue::Signed(1)),
            ("kind", AttributeValue::Text("edge".to_owned())),
            ("empty", AttributeValue::Null),
        ],
    );
    let true_leaf = FilterExpression::Compare(Comparison::new(
        "x",
        ComparisonOperator::Equal,
        AttributeValue::Signed(1),
    ));
    let false_leaf = FilterExpression::Compare(Comparison::new(
        "x",
        ComparisonOperator::Equal,
        AttributeValue::Signed(2),
    ));
    for invalid in [
        (
            FilterExpression::IsNull {
                attribute: "missing".to_owned(),
            },
            StylingError::UnknownAttribute("missing".to_owned()),
        ),
        (
            FilterExpression::Compare(Comparison::new(
                "kind",
                ComparisonOperator::Equal,
                AttributeValue::Signed(1),
            )),
            StylingError::IncompatibleTypes,
        ),
        (
            FilterExpression::And(Vec::new()),
            StylingError::EmptyBooleanExpression,
        ),
    ] {
        for expression in [
            FilterExpression::Or(vec![true_leaf.clone(), invalid.0.clone()]),
            FilterExpression::And(vec![false_leaf.clone(), invalid.0.clone()]),
        ] {
            assert_eq!(evaluate_filter(&curve, &expression), Err(invalid.1.clone()));
        }
    }
    assert_eq!(
        evaluate_filter(
            &curve,
            &FilterExpression::In {
                attribute: "x".to_owned(),
                values: vec![
                    AttributeValue::Signed(1),
                    AttributeValue::Text("bad".to_owned())
                ]
            }
        ),
        Err(StylingError::IncompatibleTypes)
    );
}

#[test]
fn mixed_integer_filters_compare_exactly_without_lossy_conversion() {
    let curve = feature(
        "large",
        [
            ("u", AttributeValue::Unsigned(u64::MAX)),
            ("i", AttributeValue::Signed(i64::MIN)),
            ("empty", AttributeValue::Null),
        ],
    );
    for (attribute, operator, literal) in [
        (
            "u",
            ComparisonOperator::GreaterThan,
            AttributeValue::Signed(i64::MAX),
        ),
        (
            "i",
            ComparisonOperator::LessThan,
            AttributeValue::Unsigned(0),
        ),
        (
            "u",
            ComparisonOperator::Equal,
            AttributeValue::Unsigned(u64::MAX),
        ),
    ] {
        assert_eq!(
            evaluate_filter(
                &curve,
                &FilterExpression::Compare(Comparison::new(attribute, operator, literal))
            ),
            Ok(true)
        );
    }
    assert_eq!(
        evaluate_filter(
            &curve,
            &FilterExpression::Compare(Comparison::new(
                "u",
                ComparisonOperator::GreaterThan,
                AttributeValue::try_f64(0.0).expect("finite")
            ))
        ),
        Err(StylingError::NumberOutsideExactF64Range)
    );
}
