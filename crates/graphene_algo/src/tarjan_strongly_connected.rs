use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;

/// A directed graph consisting of nodes and edges.
#[derive(Clone)]
pub struct Graph<NodeId, EdgeId> {
    pub nodes: HashMap<NodeId, ()>, // set-like node collection
    pub edges: HashMap<EdgeId, (NodeId, NodeId)>, // source -> target
}

impl<NodeId, EdgeId> Graph<NodeId, EdgeId>
where
    NodeId: Eq + Hash + Clone + Ord,
    EdgeId: Eq + Hash + Clone + Ord,
{
    /// Build adjacency lists from the raw edges (calls automatically).
    fn build_adjacency_lists(&self) -> HashMap<NodeId, Vec<NodeId>> {
        let mut adj = HashMap::<NodeId, Vec<NodeId>>::new();
        for src in self.nodes.keys() {
            adj.entry(src.clone()).or_default();
        }
        for (_id, (src, tgt)) in &self.edges {
            adj.entry(src.clone()).or_default().push(tgt.clone());
        }
        adj
    }

    /// Runs Tarjan's strongly connected components algorithm.
    ///
    /// Returns:
    /// - `components`: list of each SCC found, containing its nodes + edges
    /// - `cut`: the subgraph remaining after removing every node/edge that
    ///   participates in a cycle (i.e. the acyclic "remainder")
    pub fn tarjan_strongly_connected_components(&self) -> TarjanResult<NodeId, EdgeId> {
        let adj = self.build_adjacency_lists();

        // State carried across DFS iterations
        let mut index_counter: usize = 0;
        let mut index_map: HashMap<NodeId, usize> = HashMap::new();
        let mut lowlink_map: HashMap<NodeId, usize> = HashMap::new();
        let mut on_stack_set: HashSet<NodeId> = HashSet::new();
        let mut stack: Vec<NodeId> = Vec::new();
        let mut scc_components: Vec<Vec<NodeId>> = Vec::new();

        // DFS stack frame
        #[derive(Clone)]
        enum Frame<N> {
            Entry {
                node: N,
            },
            NeighborIter {
                node: N,
                neighbors: Vec<N>,
                neighbor_idx: usize,
            },
        }

        for start in &self.nodes.keys().collect::<Vec<_>>() {
            if index_map.contains_key(start) {
                continue; // already visited
            }

            let mut dfs_stack = VecDeque::new();
            dfs_stack.push_back(Frame::Entry {
                node: (*start).clone(),
            });

            while let Some(frame) = dfs_stack.pop_front() {
                match frame {
                    Frame::Entry { node: u_id } => {
                        let idx = index_counter;
                        index_counter += 1;

                        index_map.insert(u_id.clone(), idx);
                        lowlink_map.insert(u_id.clone(), idx);

                        on_stack_set.insert(u_id.clone());
                        stack.push(u_id.clone());

                        let mut neighbors_iter = adj.get(&u_id).cloned().unwrap_or_default();

                        // Re-push entry so we can resume after first neighbor finishes
                        dfs_stack.push_front(Frame::NeighborIter {
                            node: u_id,
                            neighbors: neighbors_iter.into_iter().collect(),
                            neighbor_idx: 0,
                        });
                    }

                    Frame::NeighborIter {
                        node: u_id,
                        mut neighbors,
                        neighbor_idx,
                    } => {
                        let lowlink_u = lowlink_map[&u_id];

                        // Process remaining neighbors starting from the last saved index
                        let start_idx = neighbor_idx.min(neighbors.len());
                        for (i, v_id) in neighbors.drain(start_idx..).enumerate() {
                            let global_i = i + start_idx;

                            if !index_map.contains_key(&v_id) {
                                // Tree edge — recurse (pushed back onto the front so it runs before remaining neighbors)
                                lowlink_map.insert(u_id.clone(), lowlink_u); // save current lowlink
                                dfs_stack.push_front(Frame::Entry { node: v_id.clone() });
                                break; // restart outer while with the recursive call on top
                            } else if on_stack_set.contains(&v_id) {
                                // Back edge — update lowlink
                                let ll_v = *lowlink_map.get(&v_id).unwrap();
                                lowlink_map.insert(u_id.clone(), lowlink_u.min(ll_v));
                            }
                        }

                        if neighbors.len() == 0 {
                            // All neighbors processed — check if root of an SCC
                            if index_map[&u_id] == *lowlink_map.get(&u_id).unwrap() {
                                let mut component_nodes: Vec<NodeId> = Vec::new();
                                loop {
                                    let top_node = stack.pop().unwrap();
                                    on_stack_set.remove(&top_node);
                                    lowlink_map.insert(top_node.clone(), index_counter);
                                    component_nodes.push(top_node.clone());
                                    if top_node == u_id {
                                        break;
                                    }
                                }
                                scc_components.push(component_nodes);
                            }
                        } else {
                            // Resume after children finish — save progress back on stack
                            let n_len = neighbors.len();
                            dfs_stack.push_front(Frame::NeighborIter {
                                node: u_id,
                                neighbors,
                                neighbor_idx: start_idx
                                    .min(neighbor_idx.max(start_idx) + n_len),
                            });
                        }
                    }
                }
            }
        }

        // Build component objects (node collection + internal edges)
        let mut components = Vec::new();
        for node_ids in &scc_components {
            let mut comp_nodes: HashMap<NodeId, ()> = HashMap::new();
            let mut comp_edges: HashMap<EdgeId, (NodeId, NodeId)> = HashMap::new();

            let node_set: HashSet<&NodeId> = node_ids.iter().collect();
            for nid in node_ids {
                comp_nodes.insert(nid.clone(), ());
            }
            for (eid, (src, tgt)) in &self.edges {
                if node_set.contains(src) && node_set.contains(tgt) {
                    comp_edges.insert(eid.clone(), (src.clone(), tgt.clone()));
                }
            }

            components.push(Component {
                nodes: comp_nodes,
                edges: comp_edges,
            });
        }

        // Compute `cut` — everything in the original graph that is NOT in any SCC
        let mut cycle_node_ids = HashSet::new();
        for comp in &scc_components {
            for nid in comp {
                cycle_node_ids.insert(nid);
            }
        }

        let mut cut_nodes: HashMap<NodeId, ()> = self.nodes.clone();
        let mut cut_edges: HashMap<EdgeId, (NodeId, NodeId)> = self.edges.clone();

        // Remove from cut all nodes/edges in any component
        for comp in &components {
            for nid in comp.nodes.keys() {
                cut_nodes.remove(nid);
                cut_edges.retain(|_, (src, tgt)| src != nid && tgt != nid);
            }
        }

        TarjanResult {
            components,
            cut: Graph {
                nodes: cut_nodes,
                edges: cut_edges,
            },
        }
    }
}

/// A single strongly connected component — its nodes and the internal edges.
#[derive(Clone)]
pub struct Component<NodeId, EdgeId> {
    pub nodes: HashMap<NodeId, ()>,
    pub edges: HashMap<EdgeId, (NodeId, NodeId)>,
}

/// Return value of `tarjan_strongly_connected_components`.
#[derive(Clone)]
pub struct TarjanResult<NodeId, EdgeId> {
    /// Each strongly connected component found.
    pub components: Vec<Component<NodeId, EdgeId>>,
    /// The "cut": remaining nodes/edges after all SCCs are removed (acyclic remainder).
    pub cut: Graph<NodeId, EdgeId>,
}

// === Convenience for string IDs ===
pub type SccResult = TarjanResult<String, usize>;
