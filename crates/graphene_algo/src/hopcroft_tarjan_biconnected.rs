use std::collections::{HashMap, HashSet};

/// Represents the state of each node during traversal.
#[derive(Debug)]
struct NodeState {
    id: usize,  // Discovery time
    low: usize, // Low-link value
    cut_vertex: bool,
}

/// An entry on the DFS stack representing a traversed edge.
#[derive(Debug)]
struct StackEntry {
    from: u32,
    to: u32,
    edge_id: u32,
}

/// Result of the Hopcroft-Tarjan biconnected components algorithm.
#[derive(Debug, Clone)]
pub struct BiconnectedResult {
    /// List of cut vertices (articulation points) in the graph.
    pub cut_vertices: Vec<u32>,
    /// Each component is a set of edge IDs belonging to that biconnected component.
    pub components: Vec<HashSet<u32>>,
}

/// Computes biconnected components and cut vertices for an undirected graph.
///
/// # Arguments
/// * `adj` - Adjacency list mapping each node ID to a vector of `(neighbor_id, edge_id)` pairs.
pub fn hopcroft_tarjan_biconnected(adj: &HashMap<u32, Vec<(u32, u32)>>) -> BiconnectedResult {
    let mut nodes: HashMap<u32, NodeState> = HashMap::new();
    let mut stack: Vec<StackEntry> = Vec::new();
    let mut components: Vec<HashSet<u32>> = Vec::new();
    let mut visited_edges: HashSet<u32> = HashSet::new();
    let mut id_counter: usize = 0;

    fn dfs(
        root: u32,
        current: u32,
        parent: Option<u32>,
        adj: &HashMap<u32, Vec<(u32, u32)>>,
        nodes: &mut HashMap<u32, NodeState>,
        stack: &mut Vec<StackEntry>,
        components: &mut Vec<HashSet<u32>>,
        visited_edges: &mut HashSet<u32>,
        id_counter: &mut usize,
        root_children: &mut usize,
    ) {
        nodes.insert(
            current,
            NodeState {
                id: *id_counter,
                low: *id_counter,
                cut_vertex: false,
            },
        );

        if let Some(neighbors) = adj.get(&current) {
            for &(neighbor, edge_id) in neighbors {
                // Skip the edge back to the parent
                if neighbor == parent.unwrap_or(u32::MAX) {
                    continue;
                }

                // Track visited edges to handle undirected graphs correctly
                if !visited_edges.insert(edge_id) {
                    continue;
                }

                stack.push(StackEntry {
                    from: current,
                    to: neighbor,
                    edge_id,
                });

                if nodes.contains_key(&neighbor) {
                    // Back-edge: update low-link value
                    let current_low = nodes[&current].low;
                    let neighbor_id = nodes[&neighbor].id;
                    nodes.get_mut(&current).unwrap().low = std::cmp::min(current_low, neighbor_id);
                } else {
                    // Tree-edge: recurse
                    if current == root {
                        *root_children += 1;
                    }

                    dfs(
                        root,
                        neighbor,
                        Some(current),
                        adj,
                        nodes,
                        stack,
                        components,
                        visited_edges,
                        id_counter,
                        root_children,
                    );

                    let current_low = nodes[&current].low;
                    let neighbor_low = nodes[&neighbor].low;
                    nodes.get_mut(&current).unwrap().low = std::cmp::min(current_low, neighbor_low);

                    // If `current` is an articulation point for this subtree, extract component
                    if nodes[&current].id <= nodes[&neighbor].low {
                        let mut comp_edges = HashSet::new();
                        while !stack.is_empty()
                            && !(stack.last().unwrap().from == current
                                && stack.last().unwrap().to == neighbor)
                        {
                            comp_edges.insert(stack.pop().unwrap().edge_id);
                        }
                        if let Some(e) = stack.pop() {
                            comp_edges.insert(e.edge_id);
                        }
                        components.push(comp_edges);

                        nodes.get_mut(&current).unwrap().cut_vertex = true;
                    }
                }
            }
        }
    }

    // Main loop over disconnected parts of the graph
    for &node in adj.keys() {
        if !nodes.contains_key(&node) {
            let mut root_children = 0;
            dfs(
                node,
                node,
                None,
                adj,
                &mut nodes,
                &mut stack,
                &mut components,
                &mut visited_edges,
                &mut id_counter,
                &mut root_children,
            );

            // Root is a cut vertex if it has more than one child in the DFS tree
            nodes.get_mut(&node).unwrap().cut_vertex = root_children > 1;
        }
    }

    let cut_vertices: Vec<u32> = nodes
        .iter()
        .filter(|(_, state)| state.cut_vertex)
        .map(|(id, _)| *id)
        .collect();

    BiconnectedResult {
        cut_vertices,
        components,
    }
}

/// Convenience wrapper to make graph construction and algorithm calls ergonomic.
#[derive(Debug, Clone)]
pub struct Graph {
    pub adj: HashMap<u32, Vec<(u32, u32)>>,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            adj: HashMap::new(),
        }
    }

    pub fn add_edge(&mut self, src: u32, dst: u32, id: u32) {
        self.adj.entry(src).or_default().push((dst, id));
        self.adj.entry(dst).or_default().push((src, id));
    }

    pub fn hopcroft_tarjan_biconnected(&self) -> BiconnectedResult {
        hopcroft_tarjan_biconnected(&self.adj)
    }
}
