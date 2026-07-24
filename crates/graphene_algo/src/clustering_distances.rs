/// Distance metrics used in clustering algorithms.
pub enum Metric {
    SquaredEuclidean,
    Euclidean,
    Manhattan,
    Max, // Chebyshev distance
}

/// Wraps a closure for use as a custom distance method.
pub struct CustomFn<F>(pub F);

impl<F> std::ops::Deref for CustomFn<F>
where
    F: Fn(&[f64], &[f64]) -> f64 + 'static,
{
    type Target = dyn Fn(&[f64], &[f64]) -> f64;
    fn deref(&self) -> &Self::Target {
        &self.0
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

/// Generic dispatcher that calls the correct metric.
fn compute_generic(method: &Metric, a: &[f64], b: &[f64]) -> f64 {
    match method {
        Metric::SquaredEuclidean => squared_euclidean(a, b),
        Metric::Euclidean => euclidean(a, b),
        Metric::Manhattan => manhattan(a, b),
        Metric::Max => max_distance(a, b),
    }
}

/// Compute the distance between two points.
///
/// * `method` – a built-in [`Metric`] or a custom distance function (via [`CustomFn`]).
/// * `a`, `b` – slices of coordinate values.
pub fn compute_distance(method: &Metric, a: &[f64], b: &[f64]) -> f64 {
    compute_generic(method, a, b)
}
