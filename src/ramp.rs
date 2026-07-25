use serde::{Deserialize, Serialize};

use crate::StylingError;

/// Stable identity for the complete supported named preset catalog.
pub const BUILT_IN_RAMP_CATALOG_IDENTITY: &str = "colorous_1_0_16_catalog_v1";

/// Stable identity for custom-stop channel interpolation.
pub const CUSTOM_RAMP_INTERPOLATION_IDENTITY: &str = "srgb_linear_channel_round_v1";

/// Stable semantic family for a named built-in ramp.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltInRampKind {
    /// A one-directional ordered gradient.
    Sequential,
    /// A gradient that separates two extremes around a midpoint.
    Diverging,
    /// A gradient whose endpoints meet around a cycle.
    Cyclical,
    /// A fixed ordered set of distinct colors.
    Categorical,
}

/// One stable crate-owned name for a supported `colorous` 1.0.16 preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltInRamp {
    Accent,
    BlueGreen,
    BluePurple,
    Blues,
    BrownGreen,
    Category10,
    Cividis,
    Cool,
    Cubehelix,
    Dark2,
    GreenBlue,
    Greens,
    Greys,
    Inferno,
    Magma,
    OrangeRed,
    Oranges,
    Paired,
    Pastel1,
    Pastel2,
    PinkGreen,
    Plasma,
    PurpleBlue,
    PurpleBlueGreen,
    PurpleGreen,
    PurpleOrange,
    PurpleRed,
    Purples,
    Rainbow,
    RedBlue,
    RedGrey,
    RedPurple,
    RedYellowBlue,
    RedYellowGreen,
    Reds,
    Set1,
    Set2,
    Set3,
    Sinebow,
    Spectral,
    Tableau10,
    Turbo,
    Viridis,
    Warm,
    YellowGreen,
    YellowGreenBlue,
    YellowOrangeBrown,
    YellowOrangeRed,
}

enum BuiltInRampSource {
    Continuous(colorous::Gradient),
    Categorical(&'static [colorous::Color]),
}

const BUILT_IN_RAMPS: [BuiltInRamp; 48] = [
    BuiltInRamp::Accent,
    BuiltInRamp::BlueGreen,
    BuiltInRamp::BluePurple,
    BuiltInRamp::Blues,
    BuiltInRamp::BrownGreen,
    BuiltInRamp::Category10,
    BuiltInRamp::Cividis,
    BuiltInRamp::Cool,
    BuiltInRamp::Cubehelix,
    BuiltInRamp::Dark2,
    BuiltInRamp::GreenBlue,
    BuiltInRamp::Greens,
    BuiltInRamp::Greys,
    BuiltInRamp::Inferno,
    BuiltInRamp::Magma,
    BuiltInRamp::OrangeRed,
    BuiltInRamp::Oranges,
    BuiltInRamp::Paired,
    BuiltInRamp::Pastel1,
    BuiltInRamp::Pastel2,
    BuiltInRamp::PinkGreen,
    BuiltInRamp::Plasma,
    BuiltInRamp::PurpleBlue,
    BuiltInRamp::PurpleBlueGreen,
    BuiltInRamp::PurpleGreen,
    BuiltInRamp::PurpleOrange,
    BuiltInRamp::PurpleRed,
    BuiltInRamp::Purples,
    BuiltInRamp::Rainbow,
    BuiltInRamp::RedBlue,
    BuiltInRamp::RedGrey,
    BuiltInRamp::RedPurple,
    BuiltInRamp::RedYellowBlue,
    BuiltInRamp::RedYellowGreen,
    BuiltInRamp::Reds,
    BuiltInRamp::Set1,
    BuiltInRamp::Set2,
    BuiltInRamp::Set3,
    BuiltInRamp::Sinebow,
    BuiltInRamp::Spectral,
    BuiltInRamp::Tableau10,
    BuiltInRamp::Turbo,
    BuiltInRamp::Viridis,
    BuiltInRamp::Warm,
    BuiltInRamp::YellowGreen,
    BuiltInRamp::YellowGreenBlue,
    BuiltInRamp::YellowOrangeBrown,
    BuiltInRamp::YellowOrangeRed,
];

/// Returns every supported built-in ramp in stable name order.
#[must_use]
pub const fn built_in_ramps() -> &'static [BuiltInRamp] {
    &BUILT_IN_RAMPS
}

impl BuiltInRamp {
    /// Resolves an exact lowercase kebab-case built-in name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        built_in_ramps()
            .iter()
            .copied()
            .find(|preset| preset.name() == name)
    }

    /// Returns the stable lowercase kebab-case name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Accent => "accent",
            Self::BlueGreen => "blue-green",
            Self::BluePurple => "blue-purple",
            Self::Blues => "blues",
            Self::BrownGreen => "brown-green",
            Self::Category10 => "category10",
            Self::Cividis => "cividis",
            Self::Cool => "cool",
            Self::Cubehelix => "cubehelix",
            Self::Dark2 => "dark2",
            Self::GreenBlue => "green-blue",
            Self::Greens => "greens",
            Self::Greys => "greys",
            Self::Inferno => "inferno",
            Self::Magma => "magma",
            Self::OrangeRed => "orange-red",
            Self::Oranges => "oranges",
            Self::Paired => "paired",
            Self::Pastel1 => "pastel1",
            Self::Pastel2 => "pastel2",
            Self::PinkGreen => "pink-green",
            Self::Plasma => "plasma",
            Self::PurpleBlue => "purple-blue",
            Self::PurpleBlueGreen => "purple-blue-green",
            Self::PurpleGreen => "purple-green",
            Self::PurpleOrange => "purple-orange",
            Self::PurpleRed => "purple-red",
            Self::Purples => "purples",
            Self::Rainbow => "rainbow",
            Self::RedBlue => "red-blue",
            Self::RedGrey => "red-grey",
            Self::RedPurple => "red-purple",
            Self::RedYellowBlue => "red-yellow-blue",
            Self::RedYellowGreen => "red-yellow-green",
            Self::Reds => "reds",
            Self::Set1 => "set1",
            Self::Set2 => "set2",
            Self::Set3 => "set3",
            Self::Sinebow => "sinebow",
            Self::Spectral => "spectral",
            Self::Tableau10 => "tableau10",
            Self::Turbo => "turbo",
            Self::Viridis => "viridis",
            Self::Warm => "warm",
            Self::YellowGreen => "yellow-green",
            Self::YellowGreenBlue => "yellow-green-blue",
            Self::YellowOrangeBrown => "yellow-orange-brown",
            Self::YellowOrangeRed => "yellow-orange-red",
        }
    }

    /// Returns the stable semantic family.
    #[must_use]
    pub const fn kind(self) -> BuiltInRampKind {
        match self {
            Self::Accent
            | Self::Category10
            | Self::Dark2
            | Self::Paired
            | Self::Pastel1
            | Self::Pastel2
            | Self::Set1
            | Self::Set2
            | Self::Set3
            | Self::Tableau10 => BuiltInRampKind::Categorical,
            Self::BrownGreen
            | Self::PinkGreen
            | Self::PurpleGreen
            | Self::PurpleOrange
            | Self::RedBlue
            | Self::RedGrey
            | Self::RedYellowBlue
            | Self::RedYellowGreen
            | Self::Spectral => BuiltInRampKind::Diverging,
            Self::Rainbow | Self::Sinebow => BuiltInRampKind::Cyclical,
            _ => BuiltInRampKind::Sequential,
        }
    }

    /// Returns the exact fixed-color capacity for a categorical preset.
    #[must_use]
    pub const fn recommended_category_count(self) -> Option<usize> {
        match self {
            Self::Accent | Self::Dark2 | Self::Pastel2 | Self::Set2 => Some(8),
            Self::Pastel1 | Self::Set1 => Some(9),
            Self::Category10 | Self::Tableau10 => Some(10),
            Self::Paired | Self::Set3 => Some(12),
            _ => None,
        }
    }

    fn source(self) -> BuiltInRampSource {
        match self {
            Self::Accent => BuiltInRampSource::Categorical(&colorous::ACCENT),
            Self::BlueGreen => BuiltInRampSource::Continuous(colorous::BLUE_GREEN),
            Self::BluePurple => BuiltInRampSource::Continuous(colorous::BLUE_PURPLE),
            Self::Blues => BuiltInRampSource::Continuous(colorous::BLUES),
            Self::BrownGreen => BuiltInRampSource::Continuous(colorous::BROWN_GREEN),
            Self::Category10 => BuiltInRampSource::Categorical(&colorous::CATEGORY10),
            Self::Cividis => BuiltInRampSource::Continuous(colorous::CIVIDIS),
            Self::Cool => BuiltInRampSource::Continuous(colorous::COOL),
            Self::Cubehelix => BuiltInRampSource::Continuous(colorous::CUBEHELIX),
            Self::Dark2 => BuiltInRampSource::Categorical(&colorous::DARK2),
            Self::GreenBlue => BuiltInRampSource::Continuous(colorous::GREEN_BLUE),
            Self::Greens => BuiltInRampSource::Continuous(colorous::GREENS),
            Self::Greys => BuiltInRampSource::Continuous(colorous::GREYS),
            Self::Inferno => BuiltInRampSource::Continuous(colorous::INFERNO),
            Self::Magma => BuiltInRampSource::Continuous(colorous::MAGMA),
            Self::OrangeRed => BuiltInRampSource::Continuous(colorous::ORANGE_RED),
            Self::Oranges => BuiltInRampSource::Continuous(colorous::ORANGES),
            Self::Paired => BuiltInRampSource::Categorical(&colorous::PAIRED),
            Self::Pastel1 => BuiltInRampSource::Categorical(&colorous::PASTEL1),
            Self::Pastel2 => BuiltInRampSource::Categorical(&colorous::PASTEL2),
            Self::PinkGreen => BuiltInRampSource::Continuous(colorous::PINK_GREEN),
            Self::Plasma => BuiltInRampSource::Continuous(colorous::PLASMA),
            Self::PurpleBlue => BuiltInRampSource::Continuous(colorous::PURPLE_BLUE),
            Self::PurpleBlueGreen => BuiltInRampSource::Continuous(colorous::PURPLE_BLUE_GREEN),
            Self::PurpleGreen => BuiltInRampSource::Continuous(colorous::PURPLE_GREEN),
            Self::PurpleOrange => BuiltInRampSource::Continuous(colorous::PURPLE_ORANGE),
            Self::PurpleRed => BuiltInRampSource::Continuous(colorous::PURPLE_RED),
            Self::Purples => BuiltInRampSource::Continuous(colorous::PURPLES),
            Self::Rainbow => BuiltInRampSource::Continuous(colorous::RAINBOW),
            Self::RedBlue => BuiltInRampSource::Continuous(colorous::RED_BLUE),
            Self::RedGrey => BuiltInRampSource::Continuous(colorous::RED_GREY),
            Self::RedPurple => BuiltInRampSource::Continuous(colorous::RED_PURPLE),
            Self::RedYellowBlue => BuiltInRampSource::Continuous(colorous::RED_YELLOW_BLUE),
            Self::RedYellowGreen => BuiltInRampSource::Continuous(colorous::RED_YELLOW_GREEN),
            Self::Reds => BuiltInRampSource::Continuous(colorous::REDS),
            Self::Set1 => BuiltInRampSource::Categorical(&colorous::SET1),
            Self::Set2 => BuiltInRampSource::Categorical(&colorous::SET2),
            Self::Set3 => BuiltInRampSource::Categorical(&colorous::SET3),
            Self::Sinebow => BuiltInRampSource::Continuous(colorous::SINEBOW),
            Self::Spectral => BuiltInRampSource::Continuous(colorous::SPECTRAL),
            Self::Tableau10 => BuiltInRampSource::Categorical(&colorous::TABLEAU10),
            Self::Turbo => BuiltInRampSource::Continuous(colorous::TURBO),
            Self::Viridis => BuiltInRampSource::Continuous(colorous::VIRIDIS),
            Self::Warm => BuiltInRampSource::Continuous(colorous::WARM),
            Self::YellowGreen => BuiltInRampSource::Continuous(colorous::YELLOW_GREEN),
            Self::YellowGreenBlue => BuiltInRampSource::Continuous(colorous::YELLOW_GREEN_BLUE),
            Self::YellowOrangeBrown => BuiltInRampSource::Continuous(colorous::YELLOW_ORANGE_BROWN),
            Self::YellowOrangeRed => BuiltInRampSource::Continuous(colorous::YELLOW_ORANGE_RED),
        }
    }
}

/// An eight-bit sRGB color with straight alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rgba {
    /// Red channel.
    pub red: u8,
    /// Green channel.
    pub green: u8,
    /// Blue channel.
    pub blue: u8,
    /// Alpha channel.
    pub alpha: u8,
}

impl Rgba {
    /// Constructs an sRGB color.
    #[must_use]
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

/// One validated custom color-ramp stop.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ColorStop {
    /// Position in the closed interval [0, 1].
    position: f64,
    /// Color at the stop.
    color: Rgba,
}

impl ColorStop {
    /// Constructs a ramp stop.
    ///
    /// # Errors
    ///
    /// Returns [`StylingError::InvalidRampPosition`] unless `position` is
    /// finite and in [0, 1].
    pub fn new(position: f64, color: Rgba) -> Result<Self, StylingError> {
        validate_position(position)?;
        Ok(Self { position, color })
    }

    /// Returns the validated stop position.
    #[must_use]
    pub const fn position(self) -> f64 {
        self.position
    }

    /// Returns the stop color.
    #[must_use]
    pub const fn color(self) -> Rgba {
        self.color
    }
}

/// A crate-owned color-ramp specification.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum ColorRamp {
    /// Perceptually uniform Viridis, privately backed by `colorous`.
    Viridis {
        /// Sample the ramp from high to low.
        reversed: bool,
    },
    /// One named preset from the complete supported built-in catalog.
    BuiltIn {
        /// Stable crate-owned preset identity.
        preset: BuiltInRamp,
        /// Sample the ramp from high to low.
        reversed: bool,
    },
    /// Piecewise-linear interpolation between caller-provided sRGB stops.
    Custom {
        /// Strictly increasing stops.
        stops: Vec<ColorStop>,
        /// Sample the ramp from high to low.
        reversed: bool,
    },
}

impl ColorRamp {
    /// Samples the ramp at a finite position in [0, 1].
    ///
    /// # Errors
    ///
    /// Returns a typed error for an invalid sample position or invalid custom
    /// stop ordering.
    pub fn sample(&self, position: f64) -> Result<Rgba, StylingError> {
        validate_position(position)?;
        match self {
            Self::Viridis { reversed } => {
                let position = if *reversed { 1.0 - position } else { position };
                let color = colorous::VIRIDIS.eval_continuous(position);
                Ok(Rgba::new(color.r, color.g, color.b, 255))
            }
            Self::BuiltIn { preset, reversed } => {
                let BuiltInRampSource::Continuous(gradient) = preset.source() else {
                    return Err(StylingError::CategoricalPaletteRequiresDiscreteSampling(
                        preset.name().to_owned(),
                    ));
                };
                let position = if *reversed { 1.0 - position } else { position };
                Ok(to_rgba(gradient.eval_continuous(position)))
            }
            Self::Custom { stops, reversed } => {
                validate_stops(stops)?;
                let position = if *reversed { 1.0 - position } else { position };
                sample_custom(stops, position)
            }
        }
    }

    /// Samples one color from an ordered set of `count` discrete colors.
    ///
    /// Categorical presets return their exact fixed colors and reject requests
    /// above their documented capacity. Continuous presets use their pinned
    /// rational sampling rule, except that a one-color request samples the
    /// midpoint. Custom ramps and the legacy Viridis variant use evenly spaced
    /// positions including both endpoints.
    ///
    /// # Errors
    ///
    /// Returns a typed error for zero colors, an out-of-range index, a
    /// categorical capacity violation, or an invalid custom ramp.
    pub fn sample_discrete(&self, index: usize, count: usize) -> Result<Rgba, StylingError> {
        if count == 0 {
            return Err(StylingError::ZeroClasses);
        }
        if index >= count {
            return Err(StylingError::InvalidPaletteIndex { index, count });
        }
        match self {
            Self::BuiltIn { preset, reversed } => {
                let index = if *reversed { count - 1 - index } else { index };
                match preset.source() {
                    BuiltInRampSource::Categorical(colors) => {
                        if count > colors.len() {
                            return Err(StylingError::TooManyPaletteColors {
                                palette: preset.name().to_owned(),
                                requested: count,
                                maximum: colors.len(),
                            });
                        }
                        Ok(to_rgba(colors[index]))
                    }
                    BuiltInRampSource::Continuous(gradient) => {
                        let color = if count == 1 {
                            gradient.eval_continuous(0.5)
                        } else {
                            gradient.eval_rational(index, count)
                        };
                        Ok(to_rgba(color))
                    }
                }
            }
            _ => self.sample(discrete_position(index, count)),
        }
    }
}

fn to_rgba(color: colorous::Color) -> Rgba {
    Rgba::new(color.r, color.g, color.b, 255)
}

#[allow(clippy::cast_precision_loss)]
fn discrete_position(index: usize, count: usize) -> f64 {
    if count <= 1 {
        0.5
    } else {
        index as f64 / (count - 1) as f64
    }
}

fn validate_position(position: f64) -> Result<(), StylingError> {
    if position.is_finite() && (0.0..=1.0).contains(&position) {
        Ok(())
    } else {
        Err(StylingError::InvalidRampPosition)
    }
}

fn validate_stops(stops: &[ColorStop]) -> Result<(), StylingError> {
    if stops.is_empty()
        || stops.windows(2).any(|pair| {
            !pair[0].position.is_finite()
                || !pair[1].position.is_finite()
                || pair[0].position >= pair[1].position
        })
    {
        return Err(StylingError::UnorderedRampStops);
    }
    if stops
        .iter()
        .any(|stop| !(0.0..=1.0).contains(&stop.position))
    {
        return Err(StylingError::InvalidRampPosition);
    }
    Ok(())
}

fn sample_custom(stops: &[ColorStop], position: f64) -> Result<Rgba, StylingError> {
    let first = stops.first().ok_or(StylingError::UnorderedRampStops)?;
    if position <= first.position {
        return Ok(first.color);
    }
    for pair in stops.windows(2) {
        let left = pair[0];
        let right = pair[1];
        if position <= right.position {
            let local = (position - left.position) / (right.position - left.position);
            return Ok(interpolate(left.color, right.color, local));
        }
    }
    Ok(stops.last().ok_or(StylingError::UnorderedRampStops)?.color)
}

fn interpolate(left: Rgba, right: Rgba, position: f64) -> Rgba {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn channel(left: u8, right: u8, position: f64) -> u8 {
        let interpolated = f64::from(left) + (f64::from(right) - f64::from(left)) * position;
        interpolated.round().clamp(0.0, 255.0) as u8
    }
    Rgba::new(
        channel(left.red, right.red, position),
        channel(left.green, right.green, position),
        channel(left.blue, right.blue, position),
        channel(left.alpha, right.alpha, position),
    )
}
