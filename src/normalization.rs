//! Typed continuous domains and frozen, renderer-neutral legend evidence.
// Exact equality is intentional for degenerate domains, endpoints and serialized identity.
#![allow(clippy::float_cmp)]
use crate::{ColorRamp, Rgba, StylingError};
use serde::{Deserialize, Serialize};

/// How the numeric extent is selected, after filtering and excluding nulls.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NumericDomain {
    /// Minimum and maximum of selected values.
    Observed,
    /// Caller-owned finite, ordered bounds; outliers are clipped.
    Explicit {
        /// Lower bound.
        minimum: f64,
        /// Upper bound.
        maximum: f64,
    },
    /// Type-7 empirical percentiles, with fractions in [0, 1].
    PercentileTrim {
        /// Lower percentile fraction.
        lower: f64,
        /// Upper percentile fraction, strictly greater than lower.
        upper: f64,
    },
    /// Mean ± deviations times sample standard deviation (n−1 denominator).
    StandardDeviation {
        /// Positive finite standard deviation multiplier.
        deviations: f64,
    },
}

/// Continuous mapping within the resolved domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericTransform {
    /// Linear interpolation.
    Linear,
    /// Natural logarithm; nonpositive observations or bounds are errors.
    Log,
}

/// Typed policy for normalized continuous styling.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NumericNormalization {
    /// Domain-selection policy.
    pub domain: NumericDomain,
    /// Mapping within the domain.
    pub transform: NumericTransform,
}

/// Immutable domain and ramp evidence for a normalized continuous plan.
///
/// Deserialization validates structure, not provenance of the original values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "UncheckedNormalization")]
pub struct ResolvedNormalization {
    policy: NumericNormalization,
    minimum: f64,
    maximum: f64,
    ramp: ColorRamp,
}

#[derive(Deserialize)]
struct UncheckedNormalization {
    policy: NumericNormalization,
    minimum: f64,
    maximum: f64,
    ramp: ColorRamp,
}

impl TryFrom<UncheckedNormalization> for ResolvedNormalization {
    type Error = StylingError;
    fn try_from(value: UncheckedNormalization) -> Result<Self, Self::Error> {
        let result = Self {
            policy: value.policy,
            minimum: value.minimum,
            maximum: value.maximum,
            ramp: value.ramp,
        };
        result.validate()?;
        Ok(result)
    }
}

impl NumericNormalization {
    fn validate(&self) -> Result<(), StylingError> {
        let valid = match self.domain {
            NumericDomain::Observed => true,
            NumericDomain::Explicit { minimum, maximum } => {
                minimum.is_finite() && maximum.is_finite() && minimum <= maximum
            }
            NumericDomain::PercentileTrim { lower, upper } => {
                lower.is_finite()
                    && upper.is_finite()
                    && lower >= 0.0
                    && upper <= 1.0
                    && lower < upper
            }
            NumericDomain::StandardDeviation { deviations } => {
                deviations.is_finite() && deviations > 0.0
            }
        };
        if valid {
            Ok(())
        } else {
            Err(StylingError::InvalidNormalization)
        }
    }
}

impl ResolvedNormalization {
    pub(crate) fn resolve(
        values: &[f64],
        policy: &NumericNormalization,
        ramp: &ColorRamp,
    ) -> Result<Self, StylingError> {
        policy.validate()?;
        if values.is_empty() {
            return Err(StylingError::EmptyNumericInput);
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err(StylingError::NonFiniteNumber);
        }
        if policy.transform == NumericTransform::Log && values.iter().any(|v| *v <= 0.0) {
            return Err(StylingError::NonPositiveLogValue);
        }
        let mut sorted = values.to_vec();
        sorted.sort_by(f64::total_cmp);
        let (minimum, maximum) = match policy.domain {
            NumericDomain::Observed => (sorted[0], sorted[sorted.len() - 1]),
            NumericDomain::Explicit { minimum, maximum } => (minimum, maximum),
            NumericDomain::PercentileTrim { lower, upper } => {
                (percentile(&sorted, lower), percentile(&sorted, upper))
            }
            NumericDomain::StandardDeviation { deviations } => {
                if sorted.len() < 2 {
                    return Err(StylingError::InsufficientStandardDeviationValues);
                }
                sigma_domain(&sorted, deviations)?
            }
        };
        let result = Self {
            policy: policy.clone(),
            minimum: zero(minimum),
            maximum: zero(maximum),
            ramp: ramp.clone(),
        };
        result.validate()?;
        Ok(result)
    }

    fn validate(&self) -> Result<(), StylingError> {
        self.policy.validate()?;
        if !self.minimum.is_finite() || !self.maximum.is_finite() || self.minimum > self.maximum {
            return Err(StylingError::InvalidNormalization);
        }
        if let NumericDomain::Explicit { minimum, maximum } = self.policy.domain
            && (minimum != self.minimum || maximum != self.maximum)
        {
            return Err(StylingError::InvalidNormalization);
        }
        if self.policy.transform == NumericTransform::Log && self.minimum <= 0.0 {
            return Err(StylingError::NonPositiveLogValue);
        }
        self.ramp.sample(0.5)?;
        Ok(())
    }

    /// Returns the exact normalization policy.
    #[must_use]
    pub const fn policy(&self) -> &NumericNormalization {
        &self.policy
    }
    /// Returns the effective lower bound, in original attribute units.
    #[must_use]
    pub const fn minimum(&self) -> f64 {
        self.minimum
    }
    /// Returns the effective upper bound, in original attribute units.
    #[must_use]
    pub const fn maximum(&self) -> f64 {
        self.maximum
    }
    /// Returns the frozen ramp used to paint assignments.
    #[must_use]
    pub const fn ramp(&self) -> &ColorRamp {
        &self.ramp
    }

    /// Maps an original value to a clipped ramp position.
    /// # Errors
    /// Rejects nonfinite values and nonpositive logarithmic values.
    pub fn position(&self, value: f64) -> Result<f64, StylingError> {
        if !value.is_finite() {
            return Err(StylingError::NonFiniteNumber);
        }
        if self.policy.transform == NumericTransform::Log && value <= 0.0 {
            return Err(StylingError::NonPositiveLogValue);
        }
        if self.minimum == self.maximum {
            return Ok(0.5);
        }
        let value = value.clamp(self.minimum, self.maximum);
        let position = match self.policy.transform {
            NumericTransform::Linear => {
                if (self.maximum - self.minimum).is_infinite() {
                    (value * 0.5 - self.minimum * 0.5) / (self.maximum * 0.5 - self.minimum * 0.5)
                } else {
                    (value - self.minimum) / (self.maximum - self.minimum)
                }
            }
            NumericTransform::Log => {
                log_ratio(self.minimum, value) / log_ratio(self.minimum, self.maximum)
            }
        };
        Ok(position.clamp(0.0, 1.0))
    }

    /// Returns an original-unit tick value at a normalized ramp position.
    /// # Errors
    /// Rejects nonfinite positions or positions outside [0, 1].
    pub fn value_at(&self, position: f64) -> Result<f64, StylingError> {
        if !position.is_finite() || !(0.0..=1.0).contains(&position) {
            return Err(StylingError::InvalidNormalization);
        }
        if position == 0.0 {
            return Ok(self.minimum);
        }
        if position == 1.0 {
            return Ok(self.maximum);
        }
        let value = match self.policy.transform {
            NumericTransform::Linear => interpolate(self.minimum, self.maximum, position),
            NumericTransform::Log => {
                let span = log_ratio(self.minimum, self.maximum);
                if span < 1.0 {
                    self.minimum + self.minimum * (span * position).exp_m1()
                } else {
                    interpolate(self.minimum.ln(), self.maximum.ln(), position).exp()
                }
            }
        };
        Ok(value.clamp(self.minimum, self.maximum))
    }

    /// Samples the same frozen ramp as the assignments (no numeric relabeling).
    /// # Errors
    /// Returns the ramp's error for invalid positions.
    pub fn sample_color(&self, position: f64) -> Result<Rgba, StylingError> {
        self.ramp.sample(position)
    }
}

fn zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}
fn log_ratio(minimum: f64, value: f64) -> f64 {
    let relative = (value - minimum) / minimum;
    if relative.is_finite() {
        relative.ln_1p()
    } else {
        value.ln() - minimum.ln()
    }
}
fn interpolate(lower: f64, upper: f64, position: f64) -> f64 {
    if lower == upper {
        return lower;
    }
    // Same-sign subtraction is finite; opposite-sign weighted endpoints avoid overflow.
    if lower.signum() == upper.signum() {
        lower + (upper - lower) * position
    } else {
        lower * (1.0 - position) + upper * position
    }
}
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    let rank = fraction * (sorted.len() - 1) as f64;
    let index = rank.floor() as usize;
    interpolate(
        sorted[index],
        sorted[(index + 1).min(sorted.len() - 1)],
        rank - index as f64,
    )
}
#[allow(clippy::cast_precision_loss)]
fn sigma_domain(sorted: &[f64], deviations: f64) -> Result<(f64, f64), StylingError> {
    let scale = sorted.iter().copied().map(f64::abs).fold(0.0, f64::max);
    if scale == 0.0 {
        return Ok((0.0, 0.0));
    }
    // Center before scaling so adjacent large floats do not lose their variance
    // when a rounded scaled mean is subtracted from a rounded observation.
    let origin = sorted[sorted.len() / 2];
    let mut mean = 0.0;
    let mut m2 = 0.0;
    for (i, value) in sorted.iter().enumerate() {
        let difference = *value - origin;
        let value = if difference.is_finite() {
            difference / scale
        } else {
            value / scale - origin / scale
        };
        let delta = value - mean;
        mean += delta / (i + 1) as f64;
        m2 += delta * (value - mean);
    }
    let sigma = (m2.max(0.0) / (sorted.len() - 1) as f64).sqrt();
    let lower_offset = mean - deviations * sigma;
    let upper_offset = mean + deviations * sigma;
    // Check remaining headroom before adding a tiny offset to a large origin:
    // rounded addition could otherwise conceal a mathematically overflowing bound.
    let upper_budget = if origin >= 0.0 {
        (f64::MAX - origin) / scale
    } else {
        f64::MAX / scale - origin / scale
    };
    let lower_budget = if origin <= 0.0 {
        (-f64::MAX - origin) / scale
    } else {
        -f64::MAX / scale - origin / scale
    };
    if !lower_offset.is_finite()
        || !upper_offset.is_finite()
        || lower_offset < lower_budget
        || upper_offset > upper_budget
    {
        return Err(StylingError::InvalidNormalization);
    }
    let combine = |offset: f64| {
        let delta = offset * scale;
        if delta.is_finite() {
            origin + delta
        } else {
            (origin / scale + offset) * scale
        }
    };
    let minimum = combine(lower_offset);
    let maximum = combine(upper_offset);
    if sorted[0] != sorted[sorted.len() - 1] && (sigma == 0.0 || minimum == maximum) {
        return Err(StylingError::InvalidNormalization);
    }
    Ok((minimum, maximum))
}
