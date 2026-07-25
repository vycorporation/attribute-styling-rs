use crate::{FiniteF64, MAXIMUM_CLASSES, StylingError};

const ROUNDING_EPSILON: f64 = 1e-10;
const HIGH_UNIT_BIAS: f64 = 1.5;
const FIVE_UNIT_BIAS: f64 = 0.5 + 1.5 * HIGH_UNIT_BIAS;
const MAXIMUM_CLASSES_F64: f64 = 4096.0;

/// Stable identity for the crate-owned pretty-break implementation.
pub const PRETTY_BREAKS_IDENTITY: &str = "pretty_125_covering_v1";

/// Computes finite inclusive upper bounds using round 1/2/5 decimal steps.
///
/// The unreturned lower extent and the final returned upper bound cover the
/// complete observed range. Degenerate input returns the observed value.
///
/// # Errors
///
/// Returns a typed error when the requested class count is zero or exceeds
/// the crate resource limit, when the bounds are reversed, or when finite
/// covering bounds cannot be represented.
pub fn pretty_upper_bounds(
    minimum: FiniteF64,
    maximum: FiniteF64,
    requested_classes: usize,
) -> Result<Vec<FiniteF64>, StylingError> {
    if requested_classes == 0 {
        return Err(StylingError::ZeroClasses);
    }
    if requested_classes > MAXIMUM_CLASSES {
        return Err(StylingError::TooManyClasses {
            requested: requested_classes,
            maximum: MAXIMUM_CLASSES,
        });
    }
    let minimum = minimum.get();
    let maximum = maximum.get();
    if minimum.total_cmp(&maximum).is_gt() {
        return Err(StylingError::InvalidPrettyRange);
    }
    if minimum.total_cmp(&maximum).is_eq() {
        return Ok(vec![FiniteF64::new(maximum)?]);
    }

    let span = maximum - minimum;
    if !span.is_finite() {
        return Err(StylingError::UnrepresentablePrettyRange);
    }
    #[allow(clippy::cast_precision_loss)]
    let cell = span / requested_classes as f64;
    if cell == 0.0 {
        return Ok(vec![FiniteF64::new(maximum)?]);
    }
    let base = 10_f64.powf(cell.log10().floor());
    if base == 0.0 {
        return Ok(vec![FiniteF64::new(maximum)?]);
    }
    let units = [base, 2.0 * base, 5.0 * base, 10.0 * base];
    let mut unit_index = 0;
    if units[1] - cell < HIGH_UNIT_BIAS * (cell - units[0]) {
        unit_index = 1;
        if units[2] - cell < FIVE_UNIT_BIAS * (cell - units[1]) {
            unit_index = 2;
            if units[3] - cell < HIGH_UNIT_BIAS * (cell - units[2]) {
                unit_index = 3;
            }
        }
    }
    let mut unit = units[unit_index];
    if !unit.is_finite() || unit <= 0.0 {
        return Err(StylingError::UnrepresentablePrettyRange);
    }

    let Some((mut start, mut end)) = covering_indices(minimum, maximum, unit) else {
        return Ok(vec![FiniteF64::new(maximum)?]);
    };
    while end - start > MAXIMUM_CLASSES_F64 && unit_index + 1 < units.len() {
        unit_index += 1;
        unit = units[unit_index];
        let Some(indices) = covering_indices(minimum, maximum, unit) else {
            return Ok(vec![FiniteF64::new(maximum)?]);
        };
        (start, end) = indices;
    }

    let interval_count = end - start;
    if interval_count == 0.0 {
        return Ok(vec![FiniteF64::new(maximum)?]);
    }
    if !interval_count.is_finite() || !(1.0..=MAXIMUM_CLASSES_F64).contains(&interval_count) {
        return Err(StylingError::UnrepresentablePrettyRange);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let interval_count = interval_count as usize;
    (1..=interval_count)
        .map(|index| {
            #[allow(clippy::cast_precision_loss)]
            let value = (start + index as f64) * unit;
            FiniteF64::new(if value == 0.0 { 0.0 } else { value })
                .map_err(|_| StylingError::UnrepresentablePrettyRange)
        })
        .collect()
}

fn covering_indices(minimum: f64, maximum: f64, unit: f64) -> Option<(f64, f64)> {
    let mut start = (minimum / unit + ROUNDING_EPSILON).floor();
    let mut end = (maximum / unit - ROUNDING_EPSILON).ceil();
    for _ in 0..4 {
        if start * unit <= minimum + ROUNDING_EPSILON * unit {
            break;
        }
        let next = start - 1.0;
        if next.total_cmp(&start).is_eq() {
            return None;
        }
        start = next;
    }
    if start * unit > minimum + ROUNDING_EPSILON * unit {
        return None;
    }
    for _ in 0..4 {
        if end * unit >= maximum - ROUNDING_EPSILON * unit {
            break;
        }
        let next = end + 1.0;
        if next.total_cmp(&end).is_eq() {
            return None;
        }
        end = next;
    }
    if end * unit < maximum - ROUNDING_EPSILON * unit {
        return None;
    }
    Some((start, end))
}
