/// Implemented by Zoe Xi @zoexi for GSOC 2016
/// https://github.com/cytoscape/cytoscape.js-hierarchical
///
/// Implemented from the reference library: https://harthur.github.io/clusterfck/
use std::collections::HashMap;
use std::f64::INFINITY;

// ─── Configuration ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distance {
    Euclidean,
    Manhattan,
    Cosine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linkage {
    Min,  // single linkage
    Max,  // complete linkage
    Mean, // average linkage (Ward-like)
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Threshold,
    Dendrogram,
}

#[derive(Debug, Clone)]
pub struct HierarchicalClusteringOptions {
    pub distance: Distance,
    pub linkage: Linkage,
    pub mode: Mode,
    /// The distance threshold (only used in `Threshold` mode).
    pub threshold: f64,
    /// Whether to add the dendrogram to the graph for viz.
    pub add_dendrogram: bool,
    /// Depth at which dendrogram branches are merged into the returned clusters.
    pub dendrogram_depth: usize,
    /// Array of attribute functions (index -> value) for computing distances.
    pub attributes: Vec<fn(usize) -> f64>,
}

impl Default for HierarchicalClusteringOptions {
    fn default() -> Self {
        Self {
            distance: Distance::Euclidean,
            linkage: Linkage::Min,
            mode: Mode::Threshold,
            threshold: INFINITY,
            add_dendrogram: false,
            dendrogram_depth: 0,
            attributes: Vec::new(),
        }
    }
}

// ─── Types ──────────────────────────────────────────────────────────────────

/// A data point in the clustering space.
#[derive(Debug, Clone)]
pub struct Node {
    pub id: usize,
    pub data: f64, // simplified attribute; real impl may carry a Vec<f64>
}

/// Represents a cluster during the agglomerative process.
#[derive(Debug, Clone)]
struct Cluster {
    value: usize,         // leaf node id (dendrogram mode) or representative
    keys: Vec<usize>,     // member node ids (threshold mode); None in dendrogram mode
    key: Option<usize>,   // unique cluster identifier
    index: Option<usize>, // position in the clusters vector
    size: usize,          // number of leaf nodes this cluster represents
}

/// A binary tree used for building dendrograms.
#[derive(Debug, Clone)]
pub enum DendrogramNode {
    Leaf {
        value: usize,
    },
    Internal {
        left: Box<DendrogramNode>,
        right: Box<DendrogramNode>,
    },
}

/// The result of a clustering run — groups of node ids.
#[derive(Debug, Clone)]
pub struct ClusteredNodes(pub Vec<Vec<usize>>);

// ─── Distance computation ───────────────────────────────────────────────────

fn compute_distance(
    dist_metric: Distance,
    attrs_len: usize,
    a_values: &[f64], // pre-computed attribute vector for node a
    b_values: &[f64], // pre-computed attribute vector for node b
) -> f64 {
    match dist_metric {
        Distance::Euclidean => {
            let mut sum = 0.0;
            for i in 0..attrs_len {
                let diff = a_values[i] - b_values[i];
                sum += diff * diff;
            }
            sum.sqrt()
        }
        Distance::Manhattan => {
            let mut sum = 0.0;
            for i in 0..attrs_len {
                sum += (a_values[i] - b_values[i]).abs();
            }
            sum
        }
        Distance::Cosine => {
            let mut dot = 0.0;
            let mut norm_a = 0.0;
            let mut norm_b = 0.0;
            for i in 0..attrs_len {
                dot += a_values[i] * b_values[i];
                norm_a += a_values[i].powi(2);
                norm_b += b_values[i].powi(2);
            }
            let denom = norm_a.sqrt() * norm_b.sqrt();
            if denom == 0.0 {
                INFINITY
            } else {
                (1.0 - dot / denom).sqrt() // convert similarity to distance
            }
        }
    }
}

// ─── Core algorithm ─────────────────────────────────────────────────────────

/// Merge the two closest clusters. Returns `true` if a merge happened, `false` if
/// we've reached the stopping condition (threshold or single cluster left).
fn merge_closest(
    clusters: &mut Vec<Cluster>,
    index: &mut HashMap<usize, usize>, // key -> position in clusters
    dists: &mut Vec<Vec<f64>>,
    mins: &mut Vec<usize>,
    opts: &HierarchicalClusteringOptions,
) -> bool {
    if clusters.len() <= 1 {
        return false;
    }

    // ── find the closest pair from cached mins ──────────────────────────────
    let mut min_key: usize = 0;
    let mut min_dist: f64 = INFINITY;

    for i in 0..clusters.len() {
        if let Some(key) = clusters[i].key {
            let d = dists[key][mins[key]];
            if d < min_dist {
                min_dist = d;
                min_key = key;
            }
        }
    }

    // Stop conditions
    match opts.mode {
        Mode::Threshold if min_dist >= opts.threshold => return false,
        Mode::Dendrogram if clusters.len() == 1 => return false,
        _ => {}
    }

    let c1_key = min_key;
    let c2_key = mins[min_key];
    let c1_pos = index[&c1_key];
    let c2_pos = index[&c2_key];
    let c1_size = clusters[c1_pos].size;
    let c2_size = clusters[c2_pos].size;
    let c1_val = clusters[c1_pos].value;

    // ── merge two closest clusters ──────────────────────────────────────────
    let new_key = min_key; // keep the key of the first cluster
    let new_size = c1_size + c2_size;

    match opts.mode {
        Mode::Dendrogram => {
            clusters[c1_pos] = Cluster {
                value: c1_val,
                keys: vec![],
                key: Some(new_key),
                index: Some(c1_pos),
                size: new_size,
            };
        }
        Mode::Threshold => {
            let mut merged_keys = clusters[c1_pos].keys.clone();
            merged_keys.extend(&clusters[c2_pos].keys);
            clusters[c1_pos] = Cluster {
                value: c1_val,
                keys: merged_keys,
                key: Some(new_key),
                index: Some(c1_pos),
                size: new_size,
            };
        }
    }

    // Remove c2 from the vector
    let c2_idx = c2_pos;
    let last_idx = clusters.len() - 1;
    if c2_idx < last_idx {
        clusters.swap(c2_idx, last_idx);
        let swapped = &mut clusters[c2_idx];
        swapped.index = Some(c2_idx);
        let swapped_key = swapped.key.unwrap();
        index.insert(swapped_key, c2_idx);
    }
    clusters.pop();

    index.insert(c1_key, c1_pos);

    // ── update distances with the merged cluster ────────────────────────────
    for i in 0..clusters.len() {
        let cur_key = clusters[i].key.unwrap();

        if c1_key == cur_key {
            dists[min_key][cur_key] = INFINITY;
            continue;
        }

        let new_dist = match opts.linkage {
            Linkage::Min => {
                let d1 = dists[c1_key][cur_key];
                let d2 = dists[c2_key][cur_key];
                d1.min(d2)
            }
            Linkage::Max => {
                let d1 = dists[c1_key][cur_key];
                let d2 = dists[c2_key][cur_key];
                d1.max(d2)
            }
            Linkage::Mean => {
                (dists[c1_key][cur_key] * c1_size as f64
                    + dists[c2_key][cur_key] * c2_size as f64)
                    / new_size as f64
            }
            Linkage::Custom => {
                (dists[c1_key][cur_key] + dists[c2_key][cur_key]) / 2.0
            }
        };

        dists[min_key][cur_key] = new_dist;
        dists[cur_key][min_key] = new_dist; // symmetric
    }

    // ── update cached mins ──────────────────────────────────────────────────
    for i in 0..clusters.len() {
        let key1 = clusters[i].key.unwrap();
        let mut best = key1;
        let mut d_best = INFINITY;
        for j in 0..clusters.len() {
            let key2 = clusters[j].key.unwrap();
            if key1 == key2 {
                continue;
            }
            let d = dists[key1][key2];
            if d < d_best {
                d_best = d;
                best = key2;
            }
        }
        mins[key1] = best;
        clusters[i].index = Some(i);
    }

    true
}

/// Recursively collect all leaf node ids from a dendrogram subtree.
fn get_all_children(node: &DendrogramNode, out: &mut Vec<usize>) {
    match node {
        DendrogramNode::Leaf { value } => out.push(*value),
        DendrogramNode::Internal { left, right } => {
            get_all_children(left, out);
            get_all_children(right, out);
        }
    }
}

/// Build a dendrogram binary tree from the final cluster hierarchy.
fn build_dendrogram(root: &Cluster) -> Option<DendrogramNode> {
    if root.keys.is_empty() && root.value == 0 {
        return None;
    }
    // In our simplified standalone version, clusters with keys represent merged groups.
    // Leaf nodes are clusters where value corresponds directly to a single node id.
    Some(DendrogramNode::Leaf { value: root.value })
}

/// Cut the dendrogram at depth `k` to produce final cluster groups.
fn build_clusters_from_tree(root: &DendrogramNode, k: usize) -> Vec<Vec<usize>> {
    match (k, root) {
        (_, DendrogramNode::Leaf { value }) => vec![vec![*value]],
        (0, _) => {
            // Don't cut tree — return all nodes as one cluster.
            let mut leaves = Vec::new();
            get_all_children(root, &mut leaves);
            vec![leaves]
        }
        (1, DendrogramNode::Internal { left, right }) => {
            let mut left_leaves = Vec::new();
            let mut right_leaves = Vec::new();
            get_all_children(left, &mut left_leaves);
            get_all_children(right, &mut right_leaves);
            vec![left_leaves, right_leaves]
        }
        (k, DendrogramNode::Internal { left, right }) => {
            let mut result = Vec::new();
            result.extend(build_clusters_from_tree(left, k - 1));
            result.extend(build_clusters_from_tree(right, k - 1));
            result
        }
    }
}

/// Process options: apply defaults.
fn set_options(raw: HierarchicalClusteringOptions) -> HierarchicalClusteringOptions {
    raw
}

// ─── Public API ─────────────────────────────────────────────────────────────

/// Perform agglomerative hierarchical clustering on a set of nodes.
///
/// # Arguments
/// * `nodes` — node ids and their attribute vectors for distance computation.
/// * `opts`  — clustering parameters (distance metric, linkage, mode, etc.).
pub fn hierarchical_clustering(
    nodes: &[Node],
    opts: HierarchicalClusteringOptions,
) -> ClusteredNodes {
    if nodes.is_empty() {
        return ClusteredNodes(Vec::new());
    }

    let opts = set_options(opts);

    // ── prepare attribute lookup per node ───────────────────────────────────
    let attrs_len = if opts.attributes.is_empty() { 1 } else { opts.attributes.len() };
    let node_attrs: Vec<Vec<f64>> = if opts.attributes.is_empty() {
        nodes.iter().map(|n| vec![n.data]).collect()
    } else {
        nodes
            .iter()
            .map(|n| (0..opts.attributes.len()).map(|i| (opts.attributes[i])(n.id)).collect())
            .collect()
    };

    // ── initialize: each node starts as its own cluster ─────────────────────
    let mut clusters: Vec<Cluster> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| Cluster {
            value: n.id,
            keys: vec![n.id],
            key: Some(i),
            index: Some(i),
            size: 1,
        })
        .collect();

    let mut dists: Vec<Vec<f64>> = vec![vec![0.0; nodes.len()]; nodes.len()];
    let mut mins: Vec<usize> = vec![0; nodes.len()];
    let mut index: HashMap<usize, usize> = HashMap::with_capacity(nodes.len());

    for (i, c) in clusters.iter().enumerate() {
        index.insert(c.key.unwrap(), i);
    }

    // ── compute initial pairwise distances ──────────────────────────────────
    for i in 0..nodes.len() {
        mins[i] = 0;
        for j in 0..=i {
            let dist = if i == j {
                INFINITY
            } else {
                compute_distance(opts.distance, attrs_len, &node_attrs[i], &node_attrs[j])
            };
            dists[i][j] = dist;
            dists[j][i] = dist;

            if dist < dists[i][mins[i]] {
                mins[i] = j;
            }
        }
    }

    // ── iterative merging ───────────────────────────────────────────────────
    while merge_closest(&mut clusters, &mut index, &mut dists, &mut mins, &opts) {}

    // ── produce results ─────────────────────────────────────────────────────
    match opts.mode {
        Mode::Dendrogram => {
            if let Some(first) = clusters.first() {
                let dendro = build_dendrogram(first);
                let groups = build_clusters_from_tree(
                    &dendro.unwrap_or(DendrogramNode::Leaf { value: nodes[0].id }),
                    opts.dendrogram_depth,
                );
                ClusteredNodes(groups)
            } else {
                ClusteredNodes(Vec::new())
            }
        }
        Mode::Threshold => {
            let grouped = clusters
                .iter()
                .filter_map(|c| {
                    if c.keys.is_empty() {
                        None
                    } else {
                        Some(c.keys.clone())
                    }
                })
                .collect();
            ClusteredNodes(grouped)
        }
    }
}

/// Convenience function alias.
pub fn hca(nodes: &[Node], opts: HierarchicalClusteringOptions) -> ClusteredNodes {
    hierarchical_clustering(nodes, opts)
}

// ─── Example / main ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_linkage() {
        let nodes = vec![
            Node { id: 0, data: 1.0 },
            Node { id: 1, data: 1.1 },
            Node { id: 2, data: 5.0 },
            Node { id: 3, data: 5.2 },
        ];

        let opts = HierarchicalClusteringOptions {
            distance: Distance::Euclidean,
            linkage: Linkage::Min,
            mode: Mode::Threshold,
            threshold: INFINITY,
            ..Default::default()
        };

        let result = hierarchical_clustering(&nodes, opts);
        assert!(!result.0.is_empty());
    }

    #[test]
    fn test_with_threshold() {
        let nodes = vec![
            Node { id: 0, data: 1.0 },
            Node { id: 1, data: 2.0 },
            Node { id: 2, data: 10.0 },
        ];

        let opts = HierarchicalClusteringOptions {
            distance: Distance::Euclidean,
            linkage: Linkage::Min,
            mode: Mode::Threshold,
            threshold: 5.0,
            ..Default::default()
        };

        let result = hierarchical_clustering(&nodes, opts);
        // With threshold 5.0 and Euclidean distances (1→2 = 1, 2→10 = 8), we expect
        // nodes 0 and 1 to cluster together, while node 2 stays separate.
        assert_eq!(result.0.len(), 2);
    }

    #[test]
    fn test_empty_nodes() {
        let result = hierarchical_clustering(&[], Default::default());
        assert!(result.0.is_empty());
    }
}
