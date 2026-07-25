use serde::{Deserialize, Serialize};

use crate::StylingError;

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
    /// Caller-provided fixed colors for discrete classification.
    Fixed {
        /// Ordered fixed colors.
        colors: Vec<Rgba>,
        /// Sample the fixed sequence from high to low.
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
            Self::Fixed { .. } => Err(StylingError::FixedRampRequiresDiscreteSampling),
            Self::Custom { stops, reversed } => {
                validate_stops(stops)?;
                let position = if *reversed { 1.0 - position } else { position };
                sample_custom(stops, position)
            }
        }
    }

    /// Samples one color from an ordered set of `count` discrete colors.
    ///
    /// Fixed ramps return their first `count` colors exactly and reject
    /// requests above their capacity. Other ramps are sampled at evenly spaced
    /// positions including both endpoints.
    ///
    /// # Errors
    ///
    /// Returns a typed error for zero colors, an out-of-range index, a fixed
    /// ramp capacity violation, or an invalid custom ramp.
    pub fn sample_discrete(&self, index: usize, count: usize) -> Result<Rgba, StylingError> {
        if count == 0 {
            return Err(StylingError::ZeroClasses);
        }
        if index >= count {
            return Err(StylingError::InvalidPaletteIndex { index, count });
        }
        match self {
            Self::Fixed { colors, reversed } => {
                if count > colors.len() {
                    return Err(StylingError::TooManyFixedRampColors {
                        requested: count,
                        maximum: colors.len(),
                    });
                }
                let index = if *reversed { count - 1 - index } else { index };
                Ok(colors[index])
            }
            _ => self.sample(discrete_position(index, count)),
        }
    }
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
