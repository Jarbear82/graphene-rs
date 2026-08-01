/// Distance metrics used in clustering algorithms using enum dispatch.
pub enum Metric {
    SquaredEuclidean,
    Euclidean,
    Manhattan,
    Max, // Chebyshev distance
}

impl Metric {
    #[inline(always)]
    pub fn evaluate(&self, a: &[f64], b: &[f64]) -> f64 {
        match self {
            Metric::SquaredEuclidean => squared_euclidean(a, b),
            Metric::Euclidean => euclidean(a, b),
            Metric::Manhattan => manhattan(a, b),
            Metric::Max => max_distance(a, b),
        }
    }
}

// ─── built-in implementations ──────────────────────────────

fn squared_euclidean(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum()
}

fn euclidean(a: &[f64], b: &[f64]) -> f64 {
    squared_euclidean(a, b).sqrt()
}

fn manhattan(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum()
}

fn max_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).abs())
        .fold(f64::NEG_INFINITY, f64::max)
}

/// Compute the distance between two points.
///
/// * `method` – a built-in [`Metric`].
/// * `a`, `b` – slices of coordinate values.
#[inline(always)]
pub fn compute_distance(method: &Metric, a: &[f64], b: &[f64]) -> f64 {
    method.evaluate(a, b)
}
