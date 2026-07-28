use std::collections::HashMap;

/// Represents a weighted graph.
#[derive(Debug, Clone)]
pub struct Graph {
    pub directed: bool,
    pub nodes: Vec<String>,
    /// Edges represented as (source_id, target_id, base_weight)
    pub edges: Vec<(String, String, f64)>,
}

/// Configuration options for degree centrality calculations.
#[derive(Debug, Clone)]
pub struct CentralityOptions<'a> {
    pub root: &'a str,
    /// Transforms the raw edge weight into a calculated weight. Defaults to identity.
    pub weight: fn(f64) -> f64,
    /// Overrides the graph's inherent direction if true.
    pub directed: bool,
    /// Alpha parameter from Opsahl's paper (typically 0 ≤ α ≤ 1).
    /// α = 0 → pure degree, α = 1 → pure strength.
    pub alpha: f64,
    /// Subset of nodes to consider for edge intersections.
    /// Empty slice means all graph nodes.
    pub calling_nodes: &'a [String],
}

impl Default for CentralityOptions<'_> {
    fn default() -> Self {
        CentralityOptions {
            root: "",
            weight: |w| w,
            directed: false,
            alpha: 0.0,
            calling_nodes: &[],
        }
    }
}

/// Result of a degree centrality calculation.
#[derive(Debug, Clone)]
pub enum CentralityValue {
    Undirected { degree: f64 },
    Directed { indegree: f64, outdegree: f64 },
}

/// Precomputed normalized degrees for efficient querying.
#[derive(Debug, Clone)]
pub struct NormalizedCentrality {
    max_degree: f64,
    max_indegree: f64,
    max_outdegree: f64,
    degrees: HashMap<String, f64>,
    indegrees: HashMap<String, f64>,
    outdegrees: HashMap<String, f64>,
}

impl NormalizedCentrality {
    pub fn degree(&self, node_id: &str) -> f64 {
        if self.max_degree == 0.0 {
            return 0.0;
        }
        *self.degrees.get(node_id).unwrap_or(&0.0) / self.max_degree
    }

    pub fn indegree(&self, node_id: &str) -> f64 {
        if self.max_indegree == 0.0 {
            return 0.0;
        }
        *self.indegrees.get(node_id).unwrap_or(&0.0) / self.max_indegree
    }

    pub fn outdegree(&self, node_id: &str) -> f64 {
        if self.max_outdegree == 0.0 {
            return 0.0;
        }
        *self.outdegrees.get(node_id).unwrap_or(&0.0) / self.max_outdegree
    }
}

impl Graph {
    /// Computes the generalized degree centrality for a specific root node.
    pub fn degree_centrality(&self, opts: &CentralityOptions) -> CentralityValue {
        let allowed = if opts.calling_nodes.is_empty() {
            self.nodes.as_slice()
        } else {
            opts.calling_nodes
        };

        let is_directed = opts.directed || self.directed;
        let weight_fn = opts.weight;
        let alpha = opts.alpha;

        if !is_directed {
            let (k, s) = self.compute_undirected_k_s(opts.root, allowed, weight_fn);
            let degree = safe_centrality(k, s, alpha);
            CentralityValue::Undirected { degree }
        } else {
            let (k_in, s_in) = self.compute_directed_incoming(opts.root, allowed, weight_fn);
            let (k_out, s_out) = self.compute_directed_outgoing(opts.root, allowed, weight_fn);

            let indegree = safe_centrality(k_in, s_in, alpha);
            let outdegree = safe_centrality(k_out, s_out, alpha);
            CentralityValue::Directed {
                indegree,
                outdegree,
            }
        }
    }

    /// Computes normalized centrality for all nodes in the calling set.
    pub fn normalized_degree_centrality(&self, opts: &CentralityOptions) -> NormalizedCentrality {
        let allowed = if opts.calling_nodes.is_empty() {
            self.nodes.as_slice()
        } else {
            opts.calling_nodes
        };

        let is_directed = opts.directed || self.directed;
        let weight_fn = opts.weight;
        let alpha = opts.alpha;

        let mut max_degree = 0.0;
        let mut max_indegree = 0.0;
        let mut max_outdegree = 0.0;
        let mut degrees = HashMap::with_capacity(allowed.len());
        let mut indegrees = HashMap::with_capacity(allowed.len());
        let mut outdegrees = HashMap::with_capacity(allowed.len());

        if !is_directed {
            for node in allowed {
                let (k, s) = self.compute_undirected_k_s(node, allowed, weight_fn);
                let val = safe_centrality(k, s, alpha);
                degrees.insert(node.clone(), val);
                if val > max_degree {
                    max_degree = val;
                }
            }
        } else {
            for node in allowed {
                let (k_in, s_in) = self.compute_directed_incoming(node, allowed, weight_fn);
                let (k_out, s_out) = self.compute_directed_outgoing(node, allowed, weight_fn);

                let ind = safe_centrality(k_in, s_in, alpha);
                let outd = safe_centrality(k_out, s_out, alpha);

                indegrees.insert(node.clone(), ind);
                outdegrees.insert(node.clone(), outd);

                if ind > max_indegree {
                    max_indegree = ind;
                }
                if outd > max_outdegree {
                    max_outdegree = outd;
                }
            }
        }

        NormalizedCentrality {
            max_degree,
            max_indegree,
            max_outdegree,
            degrees,
            indegrees,
            outdegrees,
        }
    }

    // --- Internal Helpers ---
    fn compute_undirected_k_s(
        &self,
        root: &str,
        allowed: &[String],
        weight_fn: fn(f64) -> f64,
    ) -> (f64, f64) {
        let allowed_set: std::collections::HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();
        let mut k = 0.0;
        let mut s = 0.0;
        for (src, tgt, w) in &self.edges {
            if src == root && tgt == root {
                continue;
            }
            let connects = src == root || tgt == root;
            let within_allowed = allowed_set.contains(src.as_str()) && allowed_set.contains(tgt.as_str());
            if connects && within_allowed {
                k += 1.0;
                s += weight_fn(*w);
            }
        }
        (k, s)
    }

    fn compute_directed_incoming(
        &self,
        root: &str,
        allowed: &[String],
        weight_fn: fn(f64) -> f64,
    ) -> (f64, f64) {
        let allowed_set: std::collections::HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();
        let mut k = 0.0;
        let mut s = 0.0;
        for (src, tgt, w) in &self.edges {
            if src == root && tgt == root {
                continue;
            }
            if tgt == root && allowed_set.contains(src.as_str()) {
                k += 1.0;
                s += weight_fn(*w);
            }
        }
        (k, s)
    }

    fn compute_directed_outgoing(
        &self,
        root: &str,
        allowed: &[String],
        weight_fn: fn(f64) -> f64,
    ) -> (f64, f64) {
        let allowed_set: std::collections::HashSet<&str> = allowed.iter().map(|s| s.as_str()).collect();
        let mut k = 0.0;
        let mut s = 0.0;
        for (src, tgt, w) in &self.edges {
            if src == root && tgt == root {
                continue;
            }
            if src == root && allowed_set.contains(tgt.as_str()) {
                k += 1.0;
                s += weight_fn(*w);
            }
        }
        (k, s)
    }
}

/// Computes $k^{1-\alpha} \cdot s^{\alpha}$ safely.
fn safe_centrality(k: f64, s: f64, alpha: f64) -> f64 {
    let k_pow = if k == 0.0 && (1.0 - alpha) < 0.0 {
        0.0
    } else {
        k.powf(1.0 - alpha)
    };
    let s_pow = if s == 0.0 && alpha < 0.0 {
        0.0
    } else {
        s.powf(alpha)
    };
    k_pow * s_pow
}
