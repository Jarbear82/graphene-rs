use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Types & Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    BFS,
    DFS,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisitorAction {
    /// Continue traversing (equivalent to `ret === undefined` or no return)
    Continue,
    /// Stop the entire traversal early (equivalent to `ret === false`)
    StopAll,
    /// Stop and mark this node as found (equivalent to `ret === true`)
    FoundNode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub id: EdgeId,
    pub source: Node,
    pub target: Node,
}

/// Interleaved edges and nodes visited during traversal (matches JS `connectedEles`)
#[derive(Debug, Clone, Copy)]
pub enum GraphElement<'a> {
    Node(&'a Node),
    Edge(&'a Edge),
}

// ---------------------------------------------------------------------------
// Graph Structure
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    /// Controls traversal direction (equivalent to `directed` param)
    pub directed: bool,
    // O(1) lookups replacing cytoscape's internal maps
    node_by_id: HashMap<u64, usize>,
    edge_by_id: HashMap<u64, usize>,
}

impl Graph {
    pub fn new(nodes: Vec<Node>, edges: Vec<Edge>, directed: bool) -> Self {
        let mut node_by_id = HashMap::new();
        for (i, n) in nodes.iter().enumerate() {
            node_by_id.insert(n.id.0, i);
        }
        let mut edge_by_id = HashMap::new();
        for (i, e) in edges.iter().enumerate() {
            edge_by_id.insert(e.id.0, i);
        }

        Self {
            nodes,
            edges,
            directed,
            node_by_id,
            edge_by_id,
        }
    }

    pub fn node(&self, id: &NodeId) -> Option<&Node> {
        self.node_by_id.get(&id.0).map(|&i| &self.nodes[i])
    }

    pub fn edge(&self, id: &EdgeId) -> Option<&Edge> {
        self.edge_by_id.get(&id.0).map(|&i| &self.edges[i])
    }

    /// Returns edges incident to `node`
    pub fn adjacent_edges(&self, node: &Node) -> Vec<&Edge> {
        self.edges
            .iter()
            .filter(|e| e.source.id.0 == node.id.0 || e.target.id.0 == node.id.0)
            .collect()
    }

    /// Returns nodes connected via `edge`, excluding `excluding`
    pub fn neighbors_for_edge(&self, edge: &Edge, excluding: &Node) -> Vec<&Node> {
        self.nodes
            .iter()
            .filter(|n| {
                n.id.0 != excluding.id.0 && (edge.source.id.0 == n.id.0 || edge.target.id.0 == n.id.0)
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Search Queue Abstraction
// ---------------------------------------------------------------------------

enum SearchQueue {
    BFS(VecDeque<u64>),
    DFS(Vec<u64>),
}

impl SearchQueue {
    fn push(&mut self, id: u64) {
        match self {
            SearchQueue::BFS(q) => q.push_back(id),
            SearchQueue::DFS(q) => q.push(id),
        }
    }

    fn pop(&mut self) -> Option<u64> {
        match self {
            SearchQueue::BFS(q) => q.pop_front(),
            SearchQueue::DFS(q) => q.pop(),
        }
    }
}

// ---------------------------------------------------------------------------
// Traversal Function
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct SearchResult<'a> {
    /// Interleaved edges and nodes visited (matches JS `path`)
    pub path: Vec<GraphElement<'a>>,
    /// Node that triggered `StopAll` or `FoundNode`, if any (matches JS `found`)
    pub found_node: Option<&'a Node>,
}

impl Graph {
    /// Performs BFS or DFS traversal.
    /// * `roots`: Starting node IDs
    /// * `mode`: BFS or DFS
    /// * `directed_override`: Overrides the graph's default directed flag for this run
    /// * `visitor`: Closure called for each visited node
    pub fn search<F>(
        &self,
        roots: &[NodeId],
        mode: SearchMode,
        directed_override: bool,
        mut visitor: F,
    ) -> SearchResult<'_>
    where
        F: FnMut(&Node, Option<&Edge>, Option<&Node>, usize, usize) -> VisitorAction,
    {
        let mut queue = match mode {
            SearchMode::BFS => SearchQueue::BFS(VecDeque::new()),
            SearchMode::DFS => SearchQueue::DFS(Vec::new()),
        };

        let mut visited = HashSet::new();
        let mut connected_nodes_ids = Vec::new();
        let mut connected_by = HashMap::new(); // target_node_id -> edge_id
        let mut depth_map = HashMap::new();

        // Enqueue roots
        for &root_id in roots {
            if let Some(node) = self.node(&root_id) {
                queue.push(root_id.0);
                depth_map.insert(root_id.0, 0);

                if mode == SearchMode::BFS {
                    visited.insert(root_id.0);
                    connected_nodes_ids.push(root_id);
                }
            }
        }

        let mut j = 0; // visitor call counter (equivalent to `j++` in JS)
        let mut found_node: Option<&Node> = None;

        while let Some(v_id) = queue.pop() {
            let v = match self.node_by_id.get(&v_id).map(|&i| &self.nodes[i]) {
                Some(n) => n,
                None => continue,
            };

            // DFS requires lazy marking: check visited on pop, mark after
            if mode == SearchMode::DFS && visited.contains(&v_id) {
                continue;
            }
            visited.insert(v_id);
            if mode == SearchMode::DFS {
                connected_nodes_ids.push(NodeId(v_id));
            }

            let depth = depth_map.get(&v_id).copied().unwrap_or(0);

            // Determine prevEdge & prevNode from the edge that discovered this node
            let prev_edge_id = connected_by.get(&v_id).copied();
            let prev_edge = prev_edge_id.and_then(|id| self.edge(&EdgeId(id)));
            let prev_node = if let Some(e) = prev_edge {
                let other = if e.source.id.0 == v_id {
                    &e.target
                } else {
                    &e.source
                };
                self.node(&other.id)
            } else {
                None
            };

            // Invoke visitor (equivalent to `fn(v, prevEdge, prevNode, j++, depth)`)
            let action = visitor(v, prev_edge, prev_node, j, depth);
            j += 1;

            match action {
                VisitorAction::FoundNode | VisitorAction::StopAll => break,
                VisitorAction::Continue => {}
            }

            // Traverse adjacent edges (matches JS `connectedEdges().filter(...)`)
            for e in self.adjacent_edges(v) {
                let is_forward = !directed_override || e.source.id.0 == v_id;
                if !is_forward {
                    continue;
                }

                let w_nodes = self.neighbors_for_edge(e, v);
                if w_nodes.is_empty() {
                    continue;
                }

                let w_id = w_nodes[0].id; // JS takes the first match: `w = w[0]`

                if !visited.contains(&w_id.0) {
                    queue.push(w_id.0);

                    if mode == SearchMode::BFS {
                        visited.insert(w_id.0);
                        connected_nodes_ids.push(w_id);
                    }

                    connected_by.insert(w_id.0, e.id.0);
                    depth_map.insert(w_id.0, depth + 1);
                }
            }
        }

        // Reconstruct path (matches JS final loop)
        let mut path = Vec::with_capacity(connected_nodes_ids.len() * 2);
        for node_id in &connected_nodes_ids {
            if let Some(e_id) = connected_by.get(&node_id.0) {
                if let Some(e) = self.edge(&EdgeId(*e_id)) {
                    path.push(GraphElement::Edge(e));
                }
            }
            if let Some(n) = self.node(node_id) {
                path.push(GraphElement::Node(n));
            }
        }

        SearchResult {
            path,
            found_node: found_node.or_else(|| connected_nodes_ids.last().and_then(|id| self.node(id))),
        }
    }
}
