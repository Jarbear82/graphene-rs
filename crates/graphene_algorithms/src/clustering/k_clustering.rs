use rand::seq::SliceRandom;
use rand::thread_rng;
use std::f64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DistanceMetric {
    Euclidean,
    Manhattan,
}

impl Default for DistanceMetric {
    fn default() -> Self {
        DistanceMetric::Euclidean
    }
}

#[derive(Debug, Clone)]
pub struct ClusterOptions {
    pub k: usize,
    pub m: f64,                     // Fuzzifier for FCM (default 2)
    pub max_iterations: usize,      // default 100
    pub sensitivity_threshold: f64, // convergence threshold
    pub distance: DistanceMetric,   // default Euclidean
}

impl Default for ClusterOptions {
    fn default() -> Self {
        Self {
            k: 2,
            m: 2.0,
            max_iterations: 100,
            sensitivity_threshold: 1e-4,
            distance: DistanceMetric::Euclidean,
        }
    }
}

// Helper: compute distance between two points
fn compute_distance(a: &[f64], b: &[f64], metric: &DistanceMetric) -> f64 {
    match metric {
        DistanceMetric::Euclidean => a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt(),
        DistanceMetric::Manhattan => a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum(),
    }
}

// Helper: check if two matrices have converged
fn matrices_converged(u1: &[Vec<f64>], u2: &[Vec<f64>], threshold: f64) -> bool {
    for i in 0..u1.len() {
        for j in 0..u1[i].len() {
            if (u1[i][j] - u2[i][j]).abs() > threshold {
                return false;
            }
        }
    }
    true
}

// Helper: find index of minimum value
fn argmin(vals: &[f64]) -> Option<usize> {
    vals.iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i)
}

use crate::clustering::ClusteringError;

/// K-Means Clustering
pub fn k_means(data: &[Vec<f64>], opts: &ClusterOptions) -> Result<Vec<Vec<usize>>, ClusteringError> {
    if data.is_empty() || data[0].is_empty() {
        return Err(ClusteringError::EmptyData);
    }
    if opts.k > data.len() || opts.k == 0 {
        return Err(ClusteringError::InvalidK(opts.k));
    }

    let mut assignments = vec![0usize; data.len()];
    let mut centroids: Vec<Vec<f64>> = init_centroids(data, opts.k)?;
    let mut is_moving = true;
    let dim = data[0].len();

    for _ in 0..opts.max_iterations {
        // Step 2: Assign nodes to nearest centroid
        for (i, point) in data.iter().enumerate() {
            assignments[i] = argmin(
                &centroids
                    .iter()
                    .map(|c| compute_distance(point, c, &opts.distance))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        }

        // Step 3: Update centroids
        let mut new_centroids = vec![vec![0.0; dim]; opts.k];
        let mut counts = vec![0usize; opts.k];

        for (i, point) in data.iter().enumerate() {
            let c = assignments[i];
            counts[c] += 1;
            for d in 0..dim {
                new_centroids[c][d] += point[d];
            }
        }

        let mut still_moving = false;
        for c in 0..opts.k {
            if counts[c] > 0 {
                for d in 0..dim {
                    new_centroids[c][d] /= counts[c] as f64;
                }
                // Check if centroids moved significantly
                if compute_distance(&centroids[c], &new_centroids[c], &opts.distance)
                    > opts.sensitivity_threshold
                {
                    still_moving = true;
                }
            } else {
                still_moving = true; // Empty cluster implies instability
            }
        }

        centroids = new_centroids;
        if !still_moving {
            break;
        }
    }

    // Build final clusters
    let mut clusters = vec![vec![]; opts.k];
    for (i, c) in assignments.into_iter().enumerate() {
        clusters[c].push(i);
    }
    Ok(clusters)
}

/// K-Medoids Clustering
pub fn k_medoids(data: &[Vec<f64>], opts: &ClusterOptions) -> Result<Vec<Vec<usize>>, ClusteringError> {
    if data.is_empty() || data[0].is_empty() {
        return Err(ClusteringError::EmptyData);
    }
    if opts.k > data.len() || opts.k == 0 {
        return Err(ClusteringError::InvalidK(opts.k));
    }

    let mut medoids = random_medoid_indices(data, opts.k)?;
    let mut assignments = vec![0usize; data.len()];
    let mut is_moving = true;

    while is_moving && medoids.iter().all(|&i| i < data.len()) {
        // Step 2: Assign nodes to nearest medoid
        for (i, point) in data.iter().enumerate() {
            assignments[i] = argmin(
                &medoids
                    .iter()
                    .map(|&m| compute_distance(point, &data[m], &opts.distance))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        }

        is_moving = false;

        // Step 3: For each medoid, find node with lowest configuration cost in its cluster
        for m in 0..opts.k {
            let cluster_indices: Vec<usize> = assignments
                .iter()
                .enumerate()
                .filter(|(_, &c)| c == m)
                .map(|(i, _)| i)
                .collect();

            if cluster_indices.is_empty() {
                is_moving = true;
                continue;
            }

            let mut min_cost = f64::MAX;
            let mut new_medoid_idx = medoids[m];

            for &candidate in &cluster_indices {
                let cost: f64 = cluster_indices
                    .iter()
                    .map(|&idx| compute_distance(&data[candidate], &data[idx], &opts.distance))
                    .sum();
                if cost < min_cost {
                    min_cost = cost;
                    new_medoid_idx = candidate;
                }
            }

            if new_medoid_idx != medoids[m] {
                medoids[m] = new_medoid_idx;
                is_moving = true;
            }
        }
    }

    let mut clusters = vec![vec![]; opts.k];
    for (i, c) in assignments.into_iter().enumerate() {
        clusters[c].push(i);
    }
    Ok(clusters)
}

/// Fuzzy C-Means Clustering
pub fn fuzzy_c_means(
    data: &[Vec<f64>],
    opts: &ClusterOptions,
) -> Result<(Vec<Vec<usize>>, Vec<Vec<f64>>), ClusteringError> {
    if data.is_empty() || data[0].is_empty() {
        return Err(ClusteringError::EmptyData);
    }
    if opts.k > data.len() || opts.k == 0 {
        return Err(ClusteringError::InvalidK(opts.k));
    }

    let n = data.len();
    let dim = data[0].len();
    let pow_val = 2.0 / (opts.m - 1.0);

    // Initialize membership matrix U (N x K)
    let mut u: Vec<Vec<f64>> = vec![vec![0.0; opts.k]; n];
    for i in 0..n {
        let total = opts.k as f64;
        for c in 0..opts.k {
            u[i][c] = 1.0 / total; // Uniform initial membership
        }
    }

    let mut is_moving = true;
    let mut iterations = 0;

    while is_moving && iterations < opts.max_iterations {
        iterations += 1;
        is_moving = false;

        let prev_u = u.clone();

        // Step 2: Calculate cluster centers
        let mut centroids = vec![vec![0.0; dim]; opts.k];
        for c in 0..opts.k {
            let mut sum_u_m = 0.0;
            for i in 0..n {
                let u_m = u[i][c].powf(opts.m);
                sum_u_m += u_m;
                for d in 0..dim {
                    centroids[c][d] += u_m * data[i][d];
                }
            }
            if sum_u_m > 0.0 {
                for d in 0..dim {
                    centroids[c][d] /= sum_u_m;
                }
            }
        }

        // Step 3: Update membership matrix U
        let mut new_u = vec![vec![0.0; opts.k]; n];
        for i in 0..n {
            for c in 0..opts.k {
                let mut denom = 0.0;
                for k in 0..opts.k {
                    let dist_n_c = compute_distance(&data[i], &centroids[c], &opts.distance);
                    let dist_n_k = compute_distance(&data[i], &centroids[k], &opts.distance);
                    if dist_n_k == 0.0 {
                        denom += 1.0;
                        continue;
                    } // Avoid div by zero
                    denom += (dist_n_c / dist_n_k).powf(pow_val);
                }
                new_u[i][c] = if denom == 0.0 { 0.0 } else { 1.0 / denom };
            }
        }

        u = new_u;

        // Step 4: Check convergence
        if !matrices_converged(&u, &prev_u, opts.sensitivity_threshold) {
            is_moving = true;
        }
    }

    // Assign nodes to cluster with highest membership
    let mut clusters = vec![vec![]; opts.k];
    for i in 0..n {
        if let Some(max_c) = u[i]
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
            .map(|(c, _)| c)
        {
            clusters[max_c].push(i);
        }
    }

    Ok((clusters, u))
}

// --- Utilities ---
fn init_centroids(data: &[Vec<f64>], k: usize) -> Result<Vec<Vec<f64>>, ClusteringError> {
    let mut indices: Vec<usize> = (0..data.len()).collect();
    indices.shuffle(&mut thread_rng());

    Ok(indices
        .into_iter()
        .take(k)
        .map(|i| data[i].clone())
        .collect())
}

fn random_medoid_indices(data: &[Vec<f64>], k: usize) -> Result<Vec<usize>, ClusteringError> {
    let mut indices: Vec<usize> = (0..data.len()).collect();
    indices.shuffle(&mut thread_rng());

    Ok(indices.into_iter().take(k).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_k_clustering_example() {
        let options = ClusterOptions::default();

        let data: Vec<Vec<f64>> = vec![
            vec![1.0, 2.0],
            vec![1.5, 1.8],
            vec![5.0, 8.0],
            vec![8.0, 8.0],
            vec![1.0, 0.6],
            vec![9.0, 11.0],
        ];

        let clusters = k_means(&data, &options).expect("k_means failed");
        assert_eq!(clusters.len(), options.k);

        let (fcm_clusters, membership) = fuzzy_c_means(
            &data,
            &ClusterOptions {
                m: 2.5,
                ..Default::default()
            },
        )
        .expect("fuzzy_c_means failed");
        assert_eq!(fcm_clusters.len(), options.k);
        assert_eq!(membership.len(), data.len());
    }
}
