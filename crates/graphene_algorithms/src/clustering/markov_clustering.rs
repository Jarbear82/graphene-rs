/// Configuration options for the Markov Cluster algorithm.
#[derive(Debug, Clone)]
pub struct MclOptions {
    /// Power to raise the adjacency matrix during expansion. Affects computation time and granularity.
    pub expand_factor: f64,
    /// Element-wise power applied during inflation. Higher values produce tighter clusters.
    pub inflate_factor: f64,
    /// Value added to the diagonal to simulate self-loops. `1.0` is standard for neutral behavior.
    pub mult_factor: f64,
    /// Maximum iterations before forcing termination.
    pub max_iterations: usize,
}

impl Default for MclOptions {
    fn default() -> Self {
        Self {
            expand_factor: 2.0,
            inflate_factor: 2.0,
            mult_factor: 1.0,
            max_iterations: 20,
        }
    }
}

/// Performs the Markov Cluster (MCL) algorithm on a graph.
///
/// # Arguments
/// * `nodes` - Slice of node identifiers (any type that is `Clone + PartialEq`).
/// * `edges` - Slice of `(source_index, target_index)` pairs where indices correspond to `nodes`.
/// * `similarity` - Closure that computes the weight for each edge given its source and target indices.
/// * `opts` - Optional configuration overrides. Uses defaults if `None`.
///
/// # Returns
/// A vector of clusters, where each cluster is a vector of node identifiers.
pub fn markov_clustering<N, F>(
    nodes: &[N],
    edges: &[(usize, usize)],
    mut similarity: F,
    opts: Option<MclOptions>,
) -> Vec<Vec<N>>
where
    N: Clone + PartialEq,
    F: FnMut(usize, usize) -> f64,
{
    let opts = opts.unwrap_or_default();
    let n = nodes.len();
    let size = n * n;

    // Initialize similarity/stochastic matrix
    let mut m = vec![0.0f64; size];

    // Build adjacency matrix from edges (assumes undirected/symmetric graph)
    for &(src, tgt) in edges {
        if src < n && tgt < n {
            let w = similarity(src, tgt);
            m[src * n + tgt] += w;
            m[tgt * n + src] += w;
        }
    }

    // Add self-loops to diagonal
    add_loops(&mut m, n, opts.mult_factor);

    // Column-normalize to make it stochastic
    normalize(&mut m, n);

    let mut converged = false;
    let mut iterations = 0;

    while !converged && iterations < opts.max_iterations {
        // Expand: raise matrix to expand_factor power via repeated multiplication
        let expanded = expand(&m, n, opts.expand_factor);

        // Inflate: element-wise power + normalize
        m = inflate(&expanded, n, opts.inflate_factor);

        // Check convergence against the pre-inflation (expanded) matrix
        if has_converged(&m, &expanded, size) {
            converged = true;
        }

        iterations += 1;
    }

    // Assign nodes to clusters based on non-zero attractors
    let mut clusters: Vec<Vec<N>> = Vec::new();
    for i in 0..n {
        let mut cluster = Vec::new();
        for j in 0..n {
            // Matches JS: Math.round(val * 1000) / 1000 > 0
            if (m[i * n + j] * 1000.0).round() / 1000.0 > 0.0 {
                cluster.push(nodes[j].clone());
            }
        }
        if !cluster.is_empty() {
            clusters.push(cluster);
        }
    }

    // Remove duplicate clusters caused by matrix symmetry
    remove_duplicates(&mut clusters);

    clusters
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers (mapped directly from JS logic)
// ─────────────────────────────────────────────────────────────────────────────

fn add_loops(m: &mut [f64], n: usize, val: f64) {
    for i in 0..n {
        m[i * n + i] = val;
    }
}

fn normalize(m: &mut [f64], n: usize) {
    for col in 0..n {
        let mut sum = 0.0f64;
        for row in 0..n {
            sum += m[row * n + col];
        }
        // Skip normalization if column is all-zero to prevent NaN propagation
        if sum > 1e-12 {
            for row in 0..n {
                m[row * n + col] /= sum;
            }
        }
    }
}

fn mmult(a: &[f64], b: &[f64], n: usize) -> Vec<f64> {
    let mut c = vec![0.0f64; n * n];
    // Cache-friendly loop order (i, k, j) produces identical results to the original JS
    for i in 0..n {
        for k in 0..n {
            let a_ik = a[i * n + k];
            if a_ik == 0.0 {
                continue;
            } // Sparse optimization
            let b_row_start = k * n;
            let c_row_start = i * n;
            for j in 0..n {
                c[c_row_start + j] += a_ik * b[b_row_start + j];
            }
        }
    }
    c
}

fn expand(m: &[f64], n: usize, power: f64) -> Vec<f64> {
    let mut result = m.to_vec();
    for _ in 1..(power as usize) {
        result = mmult(&result, m, n);
    }
    result
}

fn inflate(m: &[f64], n: usize, power: f64) -> Vec<f64> {
    let mut result = vec![0.0f64; n * n];
    for i in 0..(n * n) {
        result[i] = m[i].powf(power);
    }
    normalize(&mut result, n);
    result
}

fn has_converged(m: &[f64], expanded: &[f64], len: usize) -> bool {
    let round_factor = 4.0;
    for i in 0..len {
        let v1 = (m[i] * 10_f64.powf(round_factor)).round() / 10_f64.powf(round_factor);
        let v2 = (expanded[i] * 10_f64.powf(round_factor)).round() / 10_f64.powf(round_factor);
        // Direct equality after rounding matches JS `Math.round` behavior
        if v1 != v2 {
            return false;
        }
    }
    true
}

fn remove_duplicates<N: PartialEq>(clusters: &mut Vec<Vec<N>>) {
    let mut unique = Vec::with_capacity(clusters.len());
    for cluster in clusters.drain(..) {
        if !unique.iter().any(|c| c == &cluster) {
            unique.push(cluster);
        }
    }
    *clusters = unique;
}

// ─────────────────────────────────────────────────────────────────────────────
// Example usage
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcl_simple_triangle() {
        // Nodes: A, B, C (indices 0, 1, 2)
        let nodes = vec!["A".to_string(), "B".to_string(), "C".to_string()];

        // Triangle graph with equal weights
        let edges = vec![(0, 1), (1, 0), (1, 2), (2, 1), (2, 0), (0, 2)];

        let clusters = markov_clustering(&nodes, &edges, |_, _| 1.0, None);

        // All nodes should converge to a single cluster
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].len(), 3);
    }
}
