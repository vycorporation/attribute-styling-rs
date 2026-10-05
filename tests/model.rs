use attribute_styling::{AttributeValue, FiniteF64, StylingError};

#[test]
fn finite_float_accepts_finite_values() {
    assert_eq!(
        AttributeValue::try_f64(12.5).expect("finite value"),
        AttributeValue::Float(FiniteF64::new(12.5).expect("finite value"))
    );
}

#[test]
fn finite_float_rejects_non_finite_values() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            AttributeValue::try_f64(value),
            Err(StylingError::NonFiniteNumber)
        );
    }
}

#[test]
fn scalar_model_preserves_supported_types() {
    let values = [
        AttributeValue::Null,
        AttributeValue::Boolean(true),
        AttributeValue::Signed(-4),
        AttributeValue::Unsigned(4),
        AttributeValue::try_f64(4.5).expect("finite value"),
        AttributeValue::Text("curve-4".to_owned()),
    ];

    assert_eq!(values.len(), 6);
}

#[test]
fn feature_record_deserialization_rejects_empty_identity() {
    let error = serde_json::from_str::<attribute_styling::FeatureRecord>(
        r#"{"feature_id":"","attributes":{}}"#,
    )
    .expect_err("empty identities must fail during deserialization");
    assert!(
        error
            .to_string()
            .contains("feature identities must not be empty")
    );
}

#[test]
fn feature_record_round_trip_preserves_identity_and_typed_attributes() {
    let expected = attribute_styling::FeatureRecord::new(
        "curve-α",
        std::collections::BTreeMap::from([
            ("null".to_owned(), AttributeValue::Null),
            ("boolean".to_owned(), AttributeValue::Boolean(true)),
            ("signed".to_owned(), AttributeValue::Signed(i64::MIN)),
            ("unsigned".to_owned(), AttributeValue::Unsigned(u64::MAX)),
            (
                "float".to_owned(),
                AttributeValue::try_f64(1.5).expect("finite"),
            ),
            ("text".to_owned(), AttributeValue::Text("色".to_owned())),
        ]),
    )
    .expect("feature");
    let decoded = serde_json::from_str::<attribute_styling::FeatureRecord>(
        &serde_json::to_string(&expected).expect("serialize"),
    )
    .expect("validated feature");
    assert_eq!(decoded, expected);
}
