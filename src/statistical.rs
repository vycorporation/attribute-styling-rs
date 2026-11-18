//! Statistical classifiers. No random sampling or renderer dependencies.

use crate::StylingError;

// Limits backpointers to 128 MiB on 64-bit hosts, before allocation.
const MAX_CELLS: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Default)]
struct Moments {
    weight: f64,
    mean: f64,
    sse: f64,
}
impl Moments {
    fn merge(self, rhs: Self) -> Self {
        if self.weight == 0.0 {
            return rhs;
        }
        if rhs.weight == 0.0 {
            return self;
        }
        let weight = self.weight + rhs.weight;
        let delta = rhs.mean - self.mean;
        Self {
            weight,
            mean: self.mean + delta * (rhs.weight / weight),
            sse: self.sse + rhs.sse + delta * delta * self.weight * (rhs.weight / weight),
        }
    }
}

fn normalize(sorted: &[f64]) -> Result<Vec<f64>, StylingError> {
    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    let span = max - min;
    let normalized = sorted
        .iter()
        .map(|x| {
            if span.is_finite() {
                (x - min) / span
            } else {
                (x * 0.5 - min * 0.5) / (max * 0.5 - min * 0.5)
            }
        })
        .collect::<Vec<_>>();
    if sorted
        .windows(2)
        .zip(normalized.windows(2))
        .any(|(x, y)| x[0] < x[1] && y[0] >= y[1])
    {
        return Err(StylingError::UnrepresentableNumericRange);
    }
    Ok(normalized)
}

fn interpolate(min: f64, max: f64, t: f64) -> f64 {
    let span = max - min;
    crate::model::canonicalize_zero(if span.is_finite() {
        min + span * t
    } else {
        min * (1.0 - t) + max * t
    })
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn standard_deviation(sorted: &[f64], classes: usize) -> Result<Vec<f64>, StylingError> {
    let min = sorted[0];
    let max = sorted[sorted.len() - 1];
    if min.total_cmp(&max).is_eq() || classes == 1 {
        return Ok(vec![max]);
    }
    let normalized = normalize(sorted)?;
    let moments = normalized.iter().fold(Moments::default(), |m, x| {
        m.merge(Moments {
            weight: 1.0,
            mean: *x,
            sse: 0.0,
        })
    });
    let sigma = (moments.sse / moments.weight).sqrt();
    let mut bounds = (1..classes)
        .map(|i| moments.mean + (i as f64 - classes as f64 / 2.0) * sigma)
        .filter(|t| *t > 0.0 && *t < 1.0)
        .map(|t| interpolate(min, max, t))
        .filter(|b| *b > min && *b < max)
        .collect::<Vec<_>>();
    bounds.dedup();
    bounds.push(max);
    Ok(bounds)
}

struct MomentTree {
    nodes: Vec<Moments>,
    base: usize,
}
impl MomentTree {
    fn new(leaves: &[Moments]) -> Self {
        let base = leaves.len().next_power_of_two();
        let mut nodes = vec![Moments::default(); 2 * base];
        nodes[base..base + leaves.len()].copy_from_slice(leaves);
        for i in (1..base).rev() {
            nodes[i] = nodes[2 * i].merge(nodes[2 * i + 1]);
        }
        Self { nodes, base }
    }
    fn cost(&self, start: usize, end: usize) -> f64 {
        let (mut l, mut r) = (start + self.base, end + self.base);
        let (mut left, mut right) = (Moments::default(), Moments::default());
        while l < r {
            if l % 2 == 1 {
                left = left.merge(self.nodes[l]);
                l += 1;
            }
            if r % 2 == 1 {
                r -= 1;
                right = self.nodes[r].merge(right);
            }
            l /= 2;
            r /= 2;
        }
        left.merge(right).sse
    }
}

struct Row<'a> {
    tree: &'a MomentTree,
    previous: &'a [f64],
    current: &'a mut [f64],
    splits: &'a mut [usize],
}
impl Row<'_> {
    fn solve(&mut self, lo: usize, hi: usize, opt_lo: usize, opt_hi: usize) {
        if lo > hi {
            return;
        }
        let mid = lo + (hi - lo) / 2;
        let mut best = f64::INFINITY;
        let mut split = opt_lo;
        for j in opt_lo..=opt_hi.min(mid - 1) {
            let cost = self.previous[j] + self.tree.cost(j, mid);
            if cost < best {
                best = cost;
                split = j;
            }
        }
        self.current[mid] = best;
        self.splits[mid] = split;
        if mid > lo {
            self.solve(lo, mid - 1, opt_lo, split);
        }
        if mid < hi {
            self.solve(mid + 1, hi, split, opt_hi);
        }
    }
}

pub(crate) fn jenks(sorted: &[f64], classes: usize) -> Result<Vec<f64>, StylingError> {
    let max = sorted[sorted.len() - 1];
    if classes == 1 || sorted[0].total_cmp(&max).is_eq() {
        return Ok(vec![max]);
    }
    let mut values = Vec::new();
    let mut weights = Vec::new();
    for x in sorted {
        if values.last() == Some(x) {
            *weights.last_mut().expect("matching value has weight") += 1.0;
        } else {
            values.push(*x);
            weights.push(1.0);
        }
    }
    let n = values.len();
    let k = classes.min(n);
    if k == n {
        return Ok(values);
    }
    let cells = (n + 1)
        .checked_mul(k + 1)
        .filter(|v| *v <= MAX_CELLS)
        .ok_or(StylingError::ClassificationWorkspaceExceeded)?;
    let normalized = normalize(&values)?;
    // Distinct normalized values alone are insufficient: squared gaps can
    // underflow and turn different candidate costs into artificial ties.
    if normalized.windows(2).any(|pair| {
        let gap = pair[1] - pair[0];
        gap * gap < f64::MIN_POSITIVE
    }) {
        return Err(StylingError::UnrepresentableNumericRange);
    }
    let leaves = normalized
        .into_iter()
        .zip(weights)
        .map(|(mean, weight)| Moments {
            mean,
            weight,
            sse: 0.0,
        })
        .collect::<Vec<_>>();
    let tree = MomentTree::new(&leaves);
    let mut splits = Vec::new();
    splits
        .try_reserve_exact(cells)
        .map_err(|_| StylingError::ClassificationWorkspaceExceeded)?;
    splits.resize(cells, 0);
    let mut previous = vec![f64::INFINITY; n + 1];
    previous[0] = 0.0;
    for class in 1..=k {
        let mut current = vec![f64::INFINITY; n + 1];
        Row {
            tree: &tree,
            previous: &previous,
            current: &mut current,
            splits: &mut splits[class * (n + 1)..(class + 1) * (n + 1)],
        }
        .solve(class, n, class - 1, n - 1);
        previous = current;
    }
    let mut bounds = Vec::with_capacity(k);
    let mut end = n;
    for class in (1..=k).rev() {
        bounds.push(values[end - 1]);
        end = splits[class * (n + 1) + end];
    }
    bounds.reverse();
    Ok(bounds)
}
