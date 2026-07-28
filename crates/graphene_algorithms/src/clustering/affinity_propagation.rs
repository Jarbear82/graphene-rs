/// Preference calculation strategy
#[derive(Debug, Clone)]
pub enum Preference {
    Median,
    Mean,
    Min,
    Max,
    Custom(f64),
}

/// Configuration for Affinity Propagation
#[derive(Debug, Clone)]
pub struct ApConfig {
    pub damping: f64,
    pub preference: Preference,
    pub max_iterations: usize,
    pub min_iterations: usize,
}

impl Default for ApConfig {
    fn default() -> Self {
        Self {
            damping: 0.8,
            preference: Preference::Median,
            max_iterations: 1000,
            min_iterations: 100,
        }
    }
}

use crate::clustering::ClusteringError;

impl ApConfig {
    /// Validates configuration constraints
    pub fn validate(&self) -> Result<(), ClusteringError> {
        if !(0.5..1.0).contains(&self.damping) {
            return Err(ClusteringError::InvalidDamping(self.damping));
        }
        Ok(())
    }

    /// Computes the preference value from the similarity matrix `S`
    pub fn compute_preference(&self, s: &[f64]) -> f64 {
        match &self.preference {
            Preference::Median => median(s),
            Preference::Mean => mean(s),
            Preference::Min => *s.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
            Preference::Max => *s.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap(),
            Preference::Custom(p) => *p,
        }
    }
}

// --- Math Helpers ---

fn median(v: &[f64]) -> f64 {
    let mut sorted = v.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

// --- Core Algorithm ---

/// Runs Affinity Propagation clustering.
///
/// # Arguments
/// * `n` - Number of data points
/// * `similarity_fn` - Closure returning similarity between two indices.
///   Must return **negative distance** (as in the original JS).
/// * `config` - Clustering configuration
///
/// # Returns
/// A list of clusters, where each cluster is a `Vec<usize>` of point indices.
pub fn affinity_propagation<F>(
    n: usize,
    mut similarity_fn: F,
    config: &ApConfig,
) -> Result<Vec<Vec<usize>>, ClusteringError>
where
    F: FnMut(usize, usize) -> f64,
{
    config.validate()?;

    let n2 = n * n;
    // s: Similarity matrix (1D), r: Responsibility, a: Availability
    let mut s = vec![f64::NEG_INFINITY; n2];
    let mut r = vec![0.0_f64; n2];
    let mut a = vec![0.0_f64; n2];

    // Build similarity matrix (off-diagonal)
    for i in 0..n {
        for j in 0..n {
            if i != j {
                s[i * n + j] = similarity_fn(i, j);
            }
        }
    }

    // Set diagonal preferences
    let p = config.compute_preference(&s);
    for i in 0..n {
        s[i * n + i] = p;
    }

    let mut old = vec![0.0_f64; n];
    let mut rp = vec![0.0_f64; n];
    let mut se = vec![0usize; n]; // Consistency counter over min_iterations
    let mut e = vec![0i32; n * config.min_iterations]; // Stores exemplar status (0 or 1)

    let mut iter = 0;
    for _ in 0..config.max_iterations {
        // Update Responsibility Matrix r
        for i in 0..n {
            let mut max = f64::NEG_INFINITY;
            let mut max2 = f64::NEG_INFINITY;
            let mut max_i = 0;

            for j in 0..n {
                old[j] = r[i * n + j];
                let as_val = a[i * n + j] + s[i * n + j];
                if as_val >= max {
                    max2 = max;
                    max = as_val;
                    max_i = j;
                } else if as_val > max2 {
                    max2 = as_val;
                }
            }

            for j in 0..n {
                r[i * n + j] =
                    (1.0 - config.damping) * (s[i * n + j] - max) + config.damping * old[j];
            }
            r[i * n + max_i] =
                (1.0 - config.damping) * (s[i * n + max_i] - max2) + config.damping * old[max_i];
        }

        // Update Availability Matrix a
        for i in 0..n {
            let mut sum = 0.0_f64;
            for j in 0..n {
                old[j] = a[j * n + i];
                rp[j] = r[j * n + i].max(0.0);
                sum += rp[j];
            }

            sum -= rp[i];
            rp[i] = r[i * n + i];
            sum += rp[i];

            for j in 0..n {
                a[j * n + i] =
                    (1.0 - config.damping) * (sum - rp[j]).min(0.0) + config.damping * old[j];
            }
            a[i * n + i] = (1.0 - config.damping) * (sum - rp[i]) + config.damping * old[i];
        }

        // Check convergence
        let mut k = 0;
        for i in 0..n {
            let e_val = if a[i * n + i] + r[i * n + i] > 0.0 {
                1i32
            } else {
                0
            };
            e[(iter % config.min_iterations) * n + i] = e_val;
            k += e_val;
        }

        if k > 0 && (iter >= config.min_iterations - 1 || iter == config.max_iterations - 1) {
            se.fill(0);
            for j in 0..config.min_iterations {
                for i in 0..n {
                    se[i] += e[j * n + i] as usize;
                }
            }

            let consistent = se.iter().all(|&s| s == 0 || s == config.min_iterations);
            if consistent {
                break;
            }
        }

        iter += 1;
    }

    // Identify exemplars (cluster centers)
    let mut exemplars: Vec<usize> = Vec::new();
    for i in 0..n {
        if a[i * n + i] + r[i * n + i] > 0.0 {
            exemplars.push(i);
        }
    }

    // Refine exemplars and assign clusters
    let cluster_indices = refine_exemplars(n, &s, &mut exemplars)?;

    // Group points into clusters
    let mut clusters: Vec<Vec<usize>> = vec![Vec::new(); exemplars.len()];
    for i in 0..n {
        if let Some(cluster_idx) = cluster_indices
            .get(i)
            .copied()
        {
            clusters[cluster_idx as usize].push(i);
        }
    }

    Ok(clusters)
}

// --- Assignment Helpers ---

fn assign_clusters(n: usize, s: &[f64], exemplars: &[usize]) -> Result<Vec<i32>, ClusteringError> {
    let mut clusters = vec![-1_i32; n];

    for i in 0..n {
        let mut max = f64::NEG_INFINITY;
        let mut index = -1_i32;

        for &e in exemplars {
            let sim = s[i * n + e];
            if sim > max {
                max = sim;
                index = e as i32;
            }
        }

        if index != -1 {
            clusters[i] = index;
        }
    }

    // Ensure exemplar positions map to themselves
    for &e in exemplars {
        clusters[e as usize] = e as i32;
    }

    Ok(clusters)
}

fn refine_exemplars(n: usize, s: &[f64], exemplars: &mut Vec<usize>) -> Result<Vec<i32>, ClusteringError> {
    let clusters = assign_clusters(n, s, exemplars)?;

    for ei in 0..exemplars.len() {
        let ii: Vec<usize> = (0..n)
            .filter(|&c| clusters.get(c).copied() == Some(exemplars[ei] as i32))
            .collect();

        if ii.is_empty() {
            continue;
        }

        let mut max_i = 0;
        let mut max_sum = f64::NEG_INFINITY;
        for i in 0..ii.len() {
            let sum: f64 = (0..ii.len()).map(|j| s[ii[j] * n + ii[i]]).sum();
            if sum > max_sum {
                max_sum = sum;
                max_i = i;
            }
        }
        exemplars[ei] = ii[max_i];
    }

    assign_clusters(n, s, exemplars)
}
