use std::cmp::Ordering;

/// Represents an edge in an undirected graph.
#[derive(Debug, Clone)]
pub struct Edge {
    pub source: usize,
    pub target: usize,
    pub weight: f64,
}

/// Disjoint Set Union (Union-Find) structure for tracking connected components.
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<u32>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        UnionFind {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }

    fn find(&mut self, x: usize) -> usize {
        // Path compression
        if self.parent[x] != x {
            self.parent[x] = self.find(self.parent[x]);
        }
        self.parent[x]
    }

    /// Unions the sets containing `x` and `y`. Returns `true` if they were in different sets.
    fn union(&mut self, x: usize, y: usize) -> bool {
        let root_x = self.find(x);
        let root_y = self.find(y);

        if root_x != root_y {
            // Union by rank
            match self.rank[root_x].cmp(&self.rank[root_y]) {
                Ordering::Less => self.parent[root_x] = root_y,
                Ordering::Greater => self.parent[root_y] = root_x,
                Ordering::Equal => {
                    self.parent[root_y] = root_x;
                    self.rank[root_x] += 1;
                }
            }
            true
        } else {
            false
        }
    }
}

/// Finds the Minimum Spanning Tree using Kruskal's algorithm.
///
/// `num_nodes` should match the highest node index in the graph (+1).
/// `edge_weight` is a closure that extracts the weight from an edge.
/// Defaults to a uniform weight of 1.0 if `None`.
pub fn kruskal(
    mut edges: Vec<Edge>,
    num_nodes: usize,
    edge_weight: Option<fn(&Edge) -> f64>,
) -> Vec<Edge> {
    let weight_fn = edge_weight.unwrap_or(|_| 1.0);

    // Sort edges by weight in non-decreasing order (equivalent to JS `S = edges.sort(...)`)
    edges.sort_by(|a, b| {
        weight_fn(a)
            .partial_cmp(&weight_fn(b))
            .unwrap_or(Ordering::Equal)
    });

    // Initialize one set per node (equivalent to JS forest initialization loop)
    let mut uf = UnionFind::new(num_nodes);
    let mut mst_edges: Vec<Edge> = Vec::with_capacity(num_nodes.saturating_sub(1));

    for edge in edges {
        let root_u = uf.find(edge.source);
        let root_v = uf.find(edge.target);

        // If nodes are in different sets, add edge to MST and merge sets (equivalent to JS `if setUIndex !== setVIndex`)
        if uf.union(root_u, root_v) {
            mst_edges.push(edge);
        }
    }

    mst_edges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kruskal_example() {
        let edges = vec![
            Edge {
                source: 0,
                target: 1,
                weight: 4.0,
            },
            Edge {
                source: 0,
                target: 2,
                weight: 3.0,
            },
            Edge {
                source: 1,
                target: 2,
                weight: 1.0,
            },
            Edge {
                source: 2,
                target: 3,
                weight: 2.0,
            },
            Edge {
                source: 1,
                target: 3,
                weight: 5.0,
            },
        ];

        let mst = kruskal(edges, 4, Some(|e: &Edge| e.weight));
        assert_eq!(mst.len(), 3);
    }
}
