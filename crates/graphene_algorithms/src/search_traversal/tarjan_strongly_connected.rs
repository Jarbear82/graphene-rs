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
        let mut state = graphene_core::GraphState::<()>::new();
        let mut node_to_id = HashMap::new();
        let mut id_to_node = HashMap::new();

        for node in self.nodes.keys() {
            let id = state.add_node(
                graphene_core::math::Vec2::default(),
                graphene_core::math::Size2::default(),
            );
            node_to_id.insert(node.clone(), id);
            id_to_node.insert(id, node.clone());
        }

        for (src, neighbors) in &self.build_adjacency_lists() {
            if let Some(&src_id) = node_to_id.get(src) {
                for tgt in neighbors {
                    if let Some(&tgt_id) = node_to_id.get(tgt) {
                        state.add_edge(src_id, tgt_id, graphene_core::EdgeData::default());
                    }
                }
            }
        }

        let raw_components = crate::pathfinding::graph_state_pathfinding::tarjan_scc(&state);

        let mut scc_components = Vec::new();
        for comp in raw_components {
            let mut component = Vec::new();
            for id in comp {
                if let Some(node) = id_to_node.get(&id) {
                    component.push(node.clone());
                }
            }
            if !component.is_empty() {
                scc_components.push(component);
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
