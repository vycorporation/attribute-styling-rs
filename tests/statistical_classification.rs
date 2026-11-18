use attribute_styling::{
    AttributeValue, Classification, Classifier, ColorRamp, FeatureRecord, StyleSpec, resolve_style,
};
use std::collections::BTreeMap;

fn bounds(values: &[f64], kind: &str, classes: usize) -> Vec<f64> {
    let classifier: Classifier =
        serde_json::from_value(serde_json::json!({"kind":kind,"classes":classes}))
            .expect("classifier supported");
    let records = values
        .iter()
        .enumerate()
        .map(|(i, x)| {
            FeatureRecord::new(
                i.to_string(),
                BTreeMap::from([("v".into(), AttributeValue::try_f64(*x).unwrap())]),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let plan = resolve_style(
        &records,
        &StyleSpec {
            filter: None,
            classification: Classification::Numeric {
                attribute: "v".into(),
                classifier,
            },
            ramp: ColorRamp::Viridis { reversed: false },
        },
    )
    .unwrap();
    assert_eq!(plan.requested_class_count(), Some(classes));
    plan.classes()
        .iter()
        .map(|c| c.upper_bound().unwrap())
        .collect()
}
#[test]
fn jenks_separates_clusters_and_never_splits_ties() {
    assert_eq!(
        bounds(&[1., 2., 2., 3., 40., 41., 42.], "jenks", 2),
        vec![3., 42.]
    );
    assert_eq!(bounds(&[2., 2., 8., 8.], "jenks", 8), vec![2., 8.]);
}
#[test]
fn standard_deviation_uses_population_sigma() {
    let b = bounds(&[-2., 0., 2.], "standard_deviation", 4);
    let sigma = (8.0_f64 / 3.0).sqrt();
    assert_eq!(b.len(), 4);
    for (a, e) in b.iter().zip([-sigma, 0., sigma, 2.]) {
        assert!((a - e).abs() < 1e-14);
    }
}
#[test]
fn statistical_constant_ranges_have_one_class() {
    for kind in ["jenks", "standard_deviation"] {
        assert_eq!(bounds(&[9., 9.], kind, 5), vec![9.]);
    }
}

fn sse(values: &[f64]) -> f64 {
    let origin = values[0];
    #[allow(clippy::cast_precision_loss)]
    let mean = values.iter().map(|v| v - origin).sum::<f64>() / values.len() as f64;
    values.iter().map(|v| (v - origin - mean).powi(2)).sum()
}
fn optimal_cost(values: &[f64], k: usize) -> f64 {
    if k == 1 {
        return sse(values);
    }
    (1..values.len())
        .filter(|i| values[*i - 1] < values[*i])
        .map(|i| sse(&values[..i]) + optimal_cost(&values[i..], k - 1))
        .fold(f64::INFINITY, f64::min)
}
#[test]
fn jenks_agrees_with_exhaustive_partition_oracle() {
    // Deterministically cover unequal duplicate frequencies and near-equal alternatives.
    let mut state = 42_u32;
    for _ in 0..300 {
        let mut values = (0..9)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                f64::from(state % 31)
            })
            .collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        for k in 2..=4 {
            let b = bounds(&values, "jenks", k);
            let mut start = 0;
            let mut cost = 0.;
            for upper in b {
                let end = values.partition_point(|v| *v <= upper);
                cost += sse(&values[start..end]);
                start = end;
            }
            let expected = optimal_cost(&values, k);
            assert!(
                (cost - expected).abs() < 1e-9,
                "{values:?}: {cost} vs {expected}"
            );
        }
    }
}
#[test]
fn extreme_and_translated_inputs_remain_covering() {
    for v in [
        vec![-f64::MAX, 0., f64::MAX],
        vec![f64::from_bits(1), f64::from_bits(2), f64::from_bits(3)],
        vec![1e15, 1e15 + 1., 1e15 + 2., 1e15 + 30.],
    ] {
        for kind in ["jenks", "standard_deviation"] {
            let b = bounds(&v, kind, 2);
            assert_eq!(b.last(), v.last());
            assert!(b.iter().all(|v| v.is_finite()));
            assert!(b.windows(2).all(|p| p[0] < p[1]));
        }
    }
}
#[test]
fn jenks_is_permutation_invariant_and_retains_frequencies() {
    let mut a = vec![0.; 20];
    a.extend([4., 7., 10.]);
    let mut reverse = a.clone();
    reverse.reverse();
    assert_eq!(bounds(&a, "jenks", 2), bounds(&reverse, "jenks", 2));
    assert_eq!(bounds(&a, "jenks", 2), vec![0., 10.]);
}
#[test]
fn deviation_odd_counts_and_outliers_reduce_effective_classes() {
    assert_eq!(
        bounds(&[0., 0., 0., 0., 100.], "standard_deviation", 8).last(),
        Some(&100.)
    );
    let b = bounds(&[-2., 0., 2.], "standard_deviation", 3);
    let half = (8.0_f64 / 3.).sqrt() / 2.;
    assert!((b[0] + half).abs() < 1e-14);
    assert!((b[1] - half).abs() < 1e-14);
}

fn resolve_nullable(
    values: &[Option<f64>],
    classifier: Classifier,
) -> Result<attribute_styling::ResolvedStylePlan, attribute_styling::StylingError> {
    let records = values
        .iter()
        .enumerate()
        .map(|(i, x)| {
            FeatureRecord::new(
                i.to_string(),
                BTreeMap::from([(
                    "v".into(),
                    x.map_or(AttributeValue::Null, |x| {
                        AttributeValue::try_f64(x).unwrap()
                    }),
                )]),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    resolve_style(
        &records,
        &StyleSpec {
            filter: None,
            classification: Classification::Numeric {
                attribute: "v".into(),
                classifier,
            },
            ramp: ColorRamp::Viridis { reversed: false },
        },
    )
}
#[test]
fn nulls_bounds_and_roundtrip_share_existing_plan_contract() {
    for classifier in [
        Classifier::Jenks { classes: 2 },
        Classifier::StandardDeviation { classes: 2 },
    ] {
        let plan =
            resolve_nullable(&[Some(0.), Some(1.), None, Some(2.)], classifier.clone()).unwrap();
        assert_eq!(plan.assignments()[2].class_index(), None);
        assert_eq!(plan.assignments()[0].class_index(), Some(0));
        assert_eq!(
            plan.assignments()[1].class_index(),
            Some(usize::from(matches!(classifier, Classifier::Jenks { .. })))
        );
        assert_eq!(plan.assignments()[3].class_index(), Some(1));
        let json = serde_json::to_string(&plan).unwrap();
        assert_eq!(
            serde_json::from_str::<attribute_styling::ResolvedStylePlan>(&json).unwrap(),
            plan
        );
        assert_eq!(
            resolve_nullable(&[None], classifier).unwrap_err(),
            attribute_styling::StylingError::EmptyNumericInput
        );
    }
}
#[test]
fn invalid_counts_and_workspace_are_explicit_errors() {
    use attribute_styling::StylingError;
    for classifier in [
        Classifier::Jenks { classes: 0 },
        Classifier::StandardDeviation { classes: 0 },
    ] {
        assert_eq!(
            resolve_nullable(&[Some(1.)], classifier).unwrap_err(),
            StylingError::ZeroClasses
        );
    }
    let values = (0..4096).map(|v| Some(f64::from(v))).collect::<Vec<_>>();
    assert_eq!(
        resolve_nullable(&values, Classifier::Jenks { classes: 4095 }).unwrap_err(),
        StylingError::ClassificationWorkspaceExceeded
    );
    assert_eq!(
        resolve_nullable(&values, Classifier::Jenks { classes: 4096 })
            .unwrap()
            .classes()
            .len(),
        4096
    );
}
#[test]
fn normalization_collision_is_not_silently_accepted() {
    for classifier in [
        Classifier::Jenks { classes: 2 },
        Classifier::StandardDeviation { classes: 2 },
    ] {
        assert_eq!(
            resolve_nullable(
                &[
                    Some(-f64::MAX),
                    Some(0.),
                    Some(f64::from_bits(1)),
                    Some(f64::MAX)
                ],
                classifier
            )
            .unwrap_err(),
            attribute_styling::StylingError::UnrepresentableNumericRange
        );
    }
}
#[test]
fn repeated_byte_distribution_uses_all_observations() {
    let values = (0..100)
        .flat_map(|_| (0..256).map(f64::from))
        .collect::<Vec<_>>();
    let b = bounds(&values, "jenks", 4);
    assert_eq!(b, vec![63., 127., 191., 255.]);
}

#[test]
fn jenks_rejects_underflowing_squared_gaps_instead_of_changing_optimum() {
    for v in [[0., 1., 3., 1e200], [0., 1e-200, 3e-200, 1.]] {
        let values = v.map(Some);
        assert_eq!(
            resolve_nullable(&values, Classifier::Jenks { classes: 3 }).unwrap_err(),
            attribute_styling::StylingError::UnrepresentableNumericRange
        );
    }
}

#[test]
fn full_u16_range_is_classified_without_sampling() {
    let values = (0..=u16::MAX).map(f64::from).collect::<Vec<_>>();
    assert_eq!(
        bounds(&values, "jenks", 4),
        vec![16_383., 32_767., 49_151., 65_535.]
    );
}
