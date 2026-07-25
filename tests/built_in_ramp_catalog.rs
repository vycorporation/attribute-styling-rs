use std::collections::{BTreeMap, BTreeSet};

use attribute_styling::{
    AttributeValue, BuiltInRamp, BuiltInRampKind, Classification, ColorRamp, FeatureRecord, Rgba,
    StyleSpec, StylingError, built_in_ramps, resolve_style,
};

const EXPECTED_NAMES: [&str; 48] = [
    "accent",
    "blue-green",
    "blue-purple",
    "blues",
    "brown-green",
    "category10",
    "cividis",
    "cool",
    "cubehelix",
    "dark2",
    "green-blue",
    "greens",
    "greys",
    "inferno",
    "magma",
    "orange-red",
    "oranges",
    "paired",
    "pastel1",
    "pastel2",
    "pink-green",
    "plasma",
    "purple-blue",
    "purple-blue-green",
    "purple-green",
    "purple-orange",
    "purple-red",
    "purples",
    "rainbow",
    "red-blue",
    "red-grey",
    "red-purple",
    "red-yellow-blue",
    "red-yellow-green",
    "reds",
    "set1",
    "set2",
    "set3",
    "sinebow",
    "spectral",
    "tableau10",
    "turbo",
    "viridis",
    "warm",
    "yellow-green",
    "yellow-green-blue",
    "yellow-orange-brown",
    "yellow-orange-red",
];

#[test]
fn catalog_has_complete_unique_stably_ordered_colorous_inventory() {
    let catalog = built_in_ramps();
    let names = catalog
        .iter()
        .map(|preset| preset.name())
        .collect::<Vec<_>>();

    assert_eq!(names, EXPECTED_NAMES);
    assert_eq!(names.iter().copied().collect::<BTreeSet<_>>().len(), 48);
    for preset in catalog {
        assert_eq!(BuiltInRamp::from_name(preset.name()), Some(*preset));
    }
    assert_eq!(BuiltInRamp::from_name("Viridis"), None);
    assert_eq!(BuiltInRamp::from_name("viridis-alias"), None);
}

#[test]
fn metadata_distinguishes_palette_families_and_categorical_capacity() {
    assert_eq!(BuiltInRamp::Viridis.kind(), BuiltInRampKind::Sequential);
    assert_eq!(BuiltInRamp::Spectral.kind(), BuiltInRampKind::Diverging);
    assert_eq!(BuiltInRamp::Rainbow.kind(), BuiltInRampKind::Cyclical);
    assert_eq!(BuiltInRamp::Set3.kind(), BuiltInRampKind::Categorical);
    assert_eq!(BuiltInRamp::Set3.recommended_category_count(), Some(12));
    assert_eq!(BuiltInRamp::Viridis.recommended_category_count(), None);

    let categorical = built_in_ramps()
        .iter()
        .filter(|preset| preset.kind() == BuiltInRampKind::Categorical)
        .count();
    assert_eq!(categorical, 10);
}

#[test]
fn continuous_family_samples_are_pinned_behind_crate_owned_rgba() {
    let cases = [
        (BuiltInRamp::Viridis, Rgba::new(68, 1, 84, 255)),
        (BuiltInRamp::Blues, Rgba::new(247, 251, 255, 255)),
        (BuiltInRamp::Spectral, Rgba::new(158, 1, 66, 255)),
        (BuiltInRamp::Rainbow, Rgba::new(109, 63, 169, 255)),
    ];
    for (preset, expected) in cases {
        let ramp = ColorRamp::BuiltIn {
            preset,
            reversed: false,
        };
        assert_eq!(
            ramp.sample(0.0)
                .unwrap_or_else(|error| panic!("{}: {error}", preset.name())),
            expected
        );
    }

    let reversed = ColorRamp::BuiltIn {
        preset: BuiltInRamp::Viridis,
        reversed: true,
    };
    assert_eq!(
        reversed.sample(0.0).expect("reversed"),
        Rgba::new(253, 231, 37, 255)
    );
    assert_eq!(
        reversed.sample_discrete(0, 1).expect("single midpoint"),
        Rgba::new(32, 144, 140, 255)
    );
}

#[test]
fn every_catalog_name_maps_to_the_exact_supported_colorous_preset() {
    fn rgba(color: colorous::Color) -> Rgba {
        Rgba::new(color.r, color.g, color.b, 255)
    }

    let continuous = [
        (BuiltInRamp::BlueGreen, colorous::BLUE_GREEN),
        (BuiltInRamp::BluePurple, colorous::BLUE_PURPLE),
        (BuiltInRamp::Blues, colorous::BLUES),
        (BuiltInRamp::BrownGreen, colorous::BROWN_GREEN),
        (BuiltInRamp::Cividis, colorous::CIVIDIS),
        (BuiltInRamp::Cool, colorous::COOL),
        (BuiltInRamp::Cubehelix, colorous::CUBEHELIX),
        (BuiltInRamp::GreenBlue, colorous::GREEN_BLUE),
        (BuiltInRamp::Greens, colorous::GREENS),
        (BuiltInRamp::Greys, colorous::GREYS),
        (BuiltInRamp::Inferno, colorous::INFERNO),
        (BuiltInRamp::Magma, colorous::MAGMA),
        (BuiltInRamp::OrangeRed, colorous::ORANGE_RED),
        (BuiltInRamp::Oranges, colorous::ORANGES),
        (BuiltInRamp::PinkGreen, colorous::PINK_GREEN),
        (BuiltInRamp::Plasma, colorous::PLASMA),
        (BuiltInRamp::PurpleBlue, colorous::PURPLE_BLUE),
        (BuiltInRamp::PurpleBlueGreen, colorous::PURPLE_BLUE_GREEN),
        (BuiltInRamp::PurpleGreen, colorous::PURPLE_GREEN),
        (BuiltInRamp::PurpleOrange, colorous::PURPLE_ORANGE),
        (BuiltInRamp::PurpleRed, colorous::PURPLE_RED),
        (BuiltInRamp::Purples, colorous::PURPLES),
        (BuiltInRamp::Rainbow, colorous::RAINBOW),
        (BuiltInRamp::RedBlue, colorous::RED_BLUE),
        (BuiltInRamp::RedGrey, colorous::RED_GREY),
        (BuiltInRamp::RedPurple, colorous::RED_PURPLE),
        (BuiltInRamp::RedYellowBlue, colorous::RED_YELLOW_BLUE),
        (BuiltInRamp::RedYellowGreen, colorous::RED_YELLOW_GREEN),
        (BuiltInRamp::Reds, colorous::REDS),
        (BuiltInRamp::Sinebow, colorous::SINEBOW),
        (BuiltInRamp::Spectral, colorous::SPECTRAL),
        (BuiltInRamp::Turbo, colorous::TURBO),
        (BuiltInRamp::Viridis, colorous::VIRIDIS),
        (BuiltInRamp::Warm, colorous::WARM),
        (BuiltInRamp::YellowGreen, colorous::YELLOW_GREEN),
        (BuiltInRamp::YellowGreenBlue, colorous::YELLOW_GREEN_BLUE),
        (
            BuiltInRamp::YellowOrangeBrown,
            colorous::YELLOW_ORANGE_BROWN,
        ),
        (BuiltInRamp::YellowOrangeRed, colorous::YELLOW_ORANGE_RED),
    ];
    for (preset, source) in continuous {
        let ramp = ColorRamp::BuiltIn {
            preset,
            reversed: false,
        };
        for (index, position) in [0.0, 0.5, 1.0].into_iter().enumerate() {
            assert_eq!(
                ramp.sample(position)
                    .unwrap_or_else(|error| panic!("{}: {error}", preset.name())),
                rgba(source.eval_continuous(position)),
                "{} continuous sample {index}",
                preset.name()
            );
            assert_eq!(
                ramp.sample_discrete(index, 3)
                    .unwrap_or_else(|error| panic!("{}: {error}", preset.name())),
                rgba(source.eval_rational(index, 3)),
                "{} discrete sample {index}",
                preset.name()
            );
        }
    }

    let categorical: [(BuiltInRamp, &[colorous::Color]); 10] = [
        (BuiltInRamp::Accent, &colorous::ACCENT),
        (BuiltInRamp::Category10, &colorous::CATEGORY10),
        (BuiltInRamp::Dark2, &colorous::DARK2),
        (BuiltInRamp::Paired, &colorous::PAIRED),
        (BuiltInRamp::Pastel1, &colorous::PASTEL1),
        (BuiltInRamp::Pastel2, &colorous::PASTEL2),
        (BuiltInRamp::Set1, &colorous::SET1),
        (BuiltInRamp::Set2, &colorous::SET2),
        (BuiltInRamp::Set3, &colorous::SET3),
        (BuiltInRamp::Tableau10, &colorous::TABLEAU10),
    ];
    for (preset, source) in categorical {
        let ramp = ColorRamp::BuiltIn {
            preset,
            reversed: false,
        };
        for (index, expected) in source.iter().copied().enumerate() {
            assert_eq!(
                ramp.sample_discrete(index, source.len())
                    .unwrap_or_else(|error| panic!("{}: {error}", preset.name())),
                rgba(expected),
                "{} color {index}",
                preset.name()
            );
        }
    }
}

#[test]
fn categorical_palettes_use_exact_fixed_colors_and_enforce_capacity() {
    let ramp = ColorRamp::BuiltIn {
        preset: BuiltInRamp::Accent,
        reversed: false,
    };
    assert_eq!(
        ramp.sample_discrete(0, 3).expect("first"),
        Rgba::new(127, 201, 127, 255)
    );
    assert_eq!(
        ramp.sample_discrete(2, 3).expect("third"),
        Rgba::new(253, 192, 134, 255)
    );
    assert_eq!(
        ramp.sample_discrete(0, 9),
        Err(StylingError::TooManyPaletteColors {
            palette: "accent".to_owned(),
            requested: 9,
            maximum: 8,
        })
    );
    assert_eq!(
        ramp.sample(0.5),
        Err(StylingError::CategoricalPaletteRequiresDiscreteSampling(
            "accent".to_owned()
        ))
    );
}

#[test]
fn categorical_style_resolution_indexes_fixed_palette_without_interpolation() {
    let input = ["c", "a", "b"]
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            FeatureRecord::new(
                format!("feature-{index}"),
                BTreeMap::from([(
                    "category".to_owned(),
                    AttributeValue::Text(value.to_owned()),
                )]),
            )
            .expect("valid feature")
        })
        .collect::<Vec<_>>();
    let plan = resolve_style(
        &input,
        &StyleSpec {
            filter: None,
            classification: Classification::Categorical {
                attribute: "category".to_owned(),
            },
            ramp: ColorRamp::BuiltIn {
                preset: BuiltInRamp::Set1,
                reversed: false,
            },
        },
    )
    .expect("categorical plan");

    assert_eq!(
        plan.classes()
            .iter()
            .map(attribute_styling::StyleClass::color)
            .collect::<Vec<_>>(),
        vec![
            Rgba::new(228, 26, 28, 255),
            Rgba::new(55, 126, 184, 255),
            Rgba::new(77, 175, 74, 255),
        ]
    );
}
