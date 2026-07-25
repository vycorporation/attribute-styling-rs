use std::collections::BTreeMap;

use attribute_styling::{
    AttributeValue, Classification, Classifier, ColorRamp, FeatureRecord, FiniteF64, StyleClass,
    StyleSpec, StylingError, pretty_upper_bounds, resolve_style,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct ReferenceFixture {
    schema: String,
    sources: ReferenceSources,
    cases: Vec<ReferenceCase>,
}

#[derive(Deserialize)]
struct ReferenceSources {
    qgis: ReferenceSource,
    r: ReferenceSource,
}

#[derive(Deserialize)]
struct ReferenceSource {
    version: String,
    #[serde(default)]
    revision: Option<String>,
    api: String,
}

#[derive(Deserialize)]
struct ReferenceCase {
    name: String,
    minimum: f64,
    maximum: f64,
    requested_classes: usize,
    qgis_upper_bounds: Vec<f64>,
    r_boundaries: Vec<f64>,
    crate_upper_bounds: Vec<String>,
}

fn records(values: &[f64]) -> Vec<FeatureRecord> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            FeatureRecord::new(
                format!("curve-{index}"),
                BTreeMap::from([(
                    "value".to_owned(),
                    AttributeValue::try_f64(*value).expect("finite fixture"),
                )]),
            )
            .expect("valid feature")
        })
        .collect()
}

fn upper_bounds(classes: &[StyleClass]) -> Vec<f64> {
    classes
        .iter()
        .map(|class| class.upper_bound().expect("numeric class"))
        .collect()
}

#[test]
fn pretty_breaks_use_round_covering_bounds_across_zero() {
    let plan = resolve_style(
        &records(&[-3.2, -2.0, 0.0, 2.0, 8.0, 8.1]),
        &StyleSpec {
            filter: None,
            classification: Classification::Numeric {
                attribute: "value".to_owned(),
                classifier: Classifier::Pretty { classes: 5 },
            },
            ramp: ColorRamp::Viridis { reversed: false },
        },
    )
    .expect("pretty classification");

    assert_eq!(plan.requested_class_count(), Some(5));
    assert_eq!(plan.effective_class_count(), 7);
    assert_eq!(
        upper_bounds(plan.classes()),
        vec![-2.0, 0.0, 2.0, 4.0, 6.0, 8.0, 10.0]
    );
    assert_eq!(
        plan.assignments()
            .iter()
            .map(attribute_styling::FeatureStyleAssignment::class_index)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(0), Some(1), Some(2), Some(5), Some(6)]
    );
}

#[test]
fn public_pretty_helper_rejects_excessive_requested_classes_before_allocation() {
    assert_eq!(
        pretty_upper_bounds(
            FiniteF64::new(0.0).expect("finite"),
            FiniteF64::new(10.0).expect("finite"),
            4097,
        ),
        Err(StylingError::TooManyClasses {
            requested: 4097,
            maximum: 4096,
        })
    );
}

#[test]
fn pretty_breaks_bound_effective_classes_before_style_allocation() {
    let plan = resolve_style(
        &records(&[0.0, 10.0]),
        &StyleSpec {
            filter: None,
            classification: Classification::Numeric {
                attribute: "value".to_owned(),
                classifier: Classifier::Pretty { classes: 4096 },
            },
            ramp: ColorRamp::Viridis { reversed: false },
        },
    )
    .expect("bounded pretty classification");

    assert!(plan.effective_class_count() <= 4096);
    assert_eq!(plan.classes().last().unwrap().upper_bound(), Some(10.0));
}

#[test]
fn public_pretty_helper_rejects_reversed_finite_bounds_explicitly() {
    assert_eq!(
        pretty_upper_bounds(
            FiniteF64::new(2.0).expect("finite"),
            FiniteF64::new(1.0).expect("finite"),
            5,
        ),
        Err(StylingError::InvalidPrettyRange)
    );
}

#[test]
fn subnormal_span_collapses_to_one_covering_class() {
    let smallest_positive = f64::from_bits(1);
    let bounds = pretty_upper_bounds(
        FiniteF64::new(0.0).expect("finite"),
        FiniteF64::new(smallest_positive).expect("finite"),
        5,
    )
    .expect("minimum-span pretty bounds");

    assert_eq!(
        bounds.iter().map(|value| value.get()).collect::<Vec<_>>(),
        vec![smallest_positive]
    );
}

#[test]
fn overflowing_finite_span_is_a_typed_pretty_range_failure() {
    assert_eq!(
        pretty_upper_bounds(
            FiniteF64::new(-f64::MAX).expect("finite"),
            FiniteF64::new(f64::MAX).expect("finite"),
            5,
        ),
        Err(StylingError::UnrepresentablePrettyRange)
    );
}

#[test]
fn overflowing_round_endpoint_is_a_typed_pretty_range_failure() {
    assert_eq!(
        pretty_upper_bounds(
            FiniteF64::new(0.0).expect("finite"),
            FiniteF64::new(f64::MAX).expect("finite"),
            1,
        ),
        Err(StylingError::UnrepresentablePrettyRange)
    );
}

#[test]
fn adjacent_large_values_collapse_when_decimal_steps_are_not_representable() {
    let minimum = 1e300_f64;
    let maximum = f64::from_bits(minimum.to_bits() + 1);
    let bounds = pretty_upper_bounds(
        FiniteF64::new(minimum).expect("finite"),
        FiniteF64::new(maximum).expect("finite"),
        5,
    )
    .expect("minimum-span pretty bounds");

    assert_eq!(
        bounds.iter().map(|value| value.get()).collect::<Vec<_>>(),
        vec![maximum]
    );
}

#[test]
fn checked_in_qgis_and_r_reference_matrix_pins_crate_semantics() {
    let fixture: ReferenceFixture =
        serde_json::from_str(include_str!("fixtures/pretty_breaks/qgis-r-v1.json"))
            .expect("valid reference fixture");
    assert_eq!(fixture.schema, "pretty_break_reference_fixture_v1");
    assert_eq!(fixture.sources.qgis.version, "4.2.0-Belém do Pará");
    assert_eq!(fixture.sources.qgis.revision.as_deref(), Some("ec9a7f91"));
    assert_eq!(fixture.sources.qgis.api, "QgsSymbolLayerUtils.prettyBreaks");
    assert_eq!(fixture.sources.r.version, "4.6.0");
    assert_eq!(fixture.sources.r.revision, None);
    assert_eq!(fixture.sources.r.api, "base::pretty");

    for case in fixture.cases {
        let actual = pretty_upper_bounds(
            FiniteF64::new(case.minimum).expect("finite fixture minimum"),
            FiniteF64::new(case.maximum).expect("finite fixture maximum"),
            case.requested_classes,
        )
        .unwrap_or_else(|error| panic!("{}: {error}", case.name))
        .into_iter()
        .map(FiniteF64::get)
        .collect::<Vec<_>>();
        let expected = case
            .crate_upper_bounds
            .iter()
            .map(|value| value.parse::<f64>().expect("valid expected f64"))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "{}", case.name);
        assert!(
            !case.qgis_upper_bounds.is_empty(),
            "{} lacks QGIS evidence",
            case.name
        );
        assert!(
            !case.r_boundaries.is_empty(),
            "{} lacks R evidence",
            case.name
        );
    }
}

#[test]
fn scaled_range_matrix_is_finite_monotonic_covering_bounded_and_deterministic() {
    for exponent in -300..=300 {
        let scale = 10_f64.powi(exponent);
        for (minimum, maximum) in [
            (0.12 * scale, 9.87 * scale),
            (-9.87 * scale, -0.12 * scale),
            (-3.2 * scale, 8.1 * scale),
        ] {
            let minimum = FiniteF64::new(minimum).expect("finite matrix minimum");
            let maximum = FiniteF64::new(maximum).expect("finite matrix maximum");
            let first = pretty_upper_bounds(minimum, maximum, 7).expect("matrix pretty bounds");
            let second = pretty_upper_bounds(minimum, maximum, 7).expect("repeat bounds");

            assert_eq!(first, second, "exponent {exponent}");
            assert!(!first.is_empty(), "exponent {exponent}");
            assert!(first.len() <= 4096, "exponent {exponent}");
            assert!(
                first.iter().all(|value| value.get().is_finite()),
                "exponent {exponent}"
            );
            assert!(
                first.windows(2).all(|pair| pair[0] < pair[1]),
                "exponent {exponent}"
            );
            assert!(
                first.last().unwrap().get() >= maximum.get(),
                "exponent {exponent}"
            );
        }
    }
}

#[test]
fn pretty_style_reuses_explicit_zero_class_and_null_only_failures() {
    let zero = resolve_style(
        &records(&[1.0]),
        &StyleSpec {
            filter: None,
            classification: Classification::Numeric {
                attribute: "value".to_owned(),
                classifier: Classifier::Pretty { classes: 0 },
            },
            ramp: ColorRamp::Viridis { reversed: false },
        },
    );
    assert_eq!(zero, Err(StylingError::ZeroClasses));

    let null_only = vec![
        FeatureRecord::new(
            "curve-null",
            BTreeMap::from([("value".to_owned(), AttributeValue::Null)]),
        )
        .expect("valid null feature"),
    ];
    let nulls = resolve_style(
        &null_only,
        &StyleSpec {
            filter: None,
            classification: Classification::Numeric {
                attribute: "value".to_owned(),
                classifier: Classifier::Pretty { classes: 5 },
            },
            ramp: ColorRamp::Viridis { reversed: false },
        },
    );
    assert_eq!(nulls, Err(StylingError::EmptyNumericInput));
}
