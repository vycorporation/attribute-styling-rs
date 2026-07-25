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
