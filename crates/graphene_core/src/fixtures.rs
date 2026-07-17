use crate::{EdgeData, GraphState, NodeId, Size2, Vec2};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct GraphFixture<S: Copy + Default> {
    pub name: String,
    pub description: String,
    pub state: GraphState<S>,
    pub weights: HashMap<usize, f32>, // edge_idx -> weight
    pub node_labels: HashMap<NodeId, String>,
    pub edge_labels: HashMap<usize, String>,
    pub node_attributes: HashMap<NodeId, HashMap<String, String>>,
    pub edge_attributes: HashMap<usize, HashMap<String, String>>,
    pub compound_groups: HashMap<NodeId, Vec<NodeId>>, // parent -> children
    pub hyperedges: Vec<Vec<NodeId>>,
    pub chart_data: HashMap<NodeId, HashMap<String, f32>>, // node -> {metric: value}
}

impl<S: Copy + Default> GraphFixture<S> {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            state: GraphState::new(),
            weights: HashMap::new(),
            node_labels: HashMap::new(),
            edge_labels: HashMap::new(),
            node_attributes: HashMap::new(),
            edge_attributes: HashMap::new(),
            compound_groups: HashMap::new(),
            hyperedges: Vec::new(),
            chart_data: HashMap::new(),
        }
    }
}

pub fn get_all_fixtures<S: Copy + Default>() -> Vec<GraphFixture<S>> {
    let mut fixtures = Vec::new();

    // 1. UNDIRECTED
    {
        // Small: A - B, B - C, C - A
        let mut f = GraphFixture::new("Undirected Small (Cycle)", "3-node simple cycle.");
        let a = f.state.add_node(Vec2::new(0.0, -50.0), Size2::new(30.0, 30.0));
        let b = f.state.add_node(Vec2::new(50.0, 50.0), Size2::new(30.0, 30.0));
        let c = f.state.add_node(Vec2::new(-50.0, 50.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(b, c, EdgeData::default());
        f.state.add_edge(c, a, EdgeData::default());
        fixtures.push(f);

        // Medium: Petersen Graph structure
        let mut f = GraphFixture::new("Undirected Medium (Petersen)", "10 nodes, 15 edges Petersen graph structure.");
        let mut nodes = Vec::new();
        for i in 0..10 {
            let angle = (i as f32) * std::f32::consts::TAU / 5.0;
            let r = if i < 5 { 100.0 } else { 50.0 };
            let pos = Vec2::new(angle.cos() * r, angle.sin() * r);
            let id = f.state.add_node(pos, Size2::new(30.0, 30.0));
            f.node_labels.insert(id, format!("{}", i + 1));
            nodes.push(id);
        }
        let edges = vec![
            (0, 1), (1, 2), (2, 3), (3, 4), (4, 0), // Outer cycle
            (0, 5), (1, 6), (2, 7), (3, 8), (4, 9), // Spoke edges
            (5, 7), (7, 9), (9, 6), (6, 8), (8, 5), // Inner star
        ];
        for (u, v) in edges {
            f.state.add_edge(nodes[u], nodes[v], EdgeData::default());
        }
        fixtures.push(f);

        // Large: Grid-like Mesh
        let mut f = GraphFixture::new("Undirected Large (Grid)", "5x5 grid mesh containing 25 nodes and 40 edges.");
        let mut nodes = Vec::new();
        for r in 0..5 {
            for c in 0..5 {
                let pos = Vec2::new((c as f32 - 2.0) * 80.0, (r as f32 - 2.0) * 80.0);
                let id = f.state.add_node(pos, Size2::new(30.0, 30.0));
                f.node_labels.insert(id, format!("N{}_{}", r + 1, c + 1));
                nodes.push(id);
            }
        }
        for r in 0..5 {
            for c in 0..5 {
                let idx = r * 5 + c;
                if c < 4 {
                    f.state.add_edge(nodes[idx], nodes[idx + 1], EdgeData::default());
                }
                if r < 4 {
                    f.state.add_edge(nodes[idx], nodes[idx + 5], EdgeData::default());
                }
            }
        }
        fixtures.push(f);
    }

    // 2. DIRECTED
    {
        // Small: A -> B, A -> C, B -> C
        let mut f = GraphFixture::new("Directed Small", "Feed-forward loop with 3 nodes.");
        let a = f.state.add_node(Vec2::new(0.0, -60.0), Size2::new(30.0, 30.0));
        let b = f.state.add_node(Vec2::new(50.0, 0.0), Size2::new(30.0, 30.0));
        let c = f.state.add_node(Vec2::new(-50.0, 60.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(a, c, EdgeData::default());
        f.state.add_edge(b, c, EdgeData::default());
        fixtures.push(f);

        // Medium: Process Flow
        let mut f = GraphFixture::new("Directed Medium (Process)", "Process flow loop containing 8 nodes.");
        let names = vec!["Start", "Step1", "Step2a", "Step2b", "Step3", "Approval", "End"];
        let mut nodes = HashMap::new();
        for (idx, name) in names.iter().enumerate() {
            let pos = Vec2::new((idx as f32 - 3.0) * 80.0, 0.0);
            let id = f.state.add_node(pos, Size2::new(45.0, 30.0));
            f.node_labels.insert(id, name.to_string());
            nodes.insert(*name, id);
        }
        let flow = vec![
            ("Start", "Step1"), ("Step1", "Step2a"), ("Step1", "Step2b"),
            ("Step2a", "Step3"), ("Step2b", "Step3"), ("Step3", "Approval"),
            ("Approval", "End"), ("Approval", "Step1")
        ];
        for (u, v) in flow {
            f.state.add_edge(nodes[u], nodes[v], EdgeData::default());
        }
        fixtures.push(f);

        // Large: Deep Cascade
        let mut f = GraphFixture::new("Directed Large (Cascade)", "Highly layered binary cascade flow network.");
        let mut nodes = Vec::new();
        for i in 0..32 {
            let pos = Vec2::new((i % 8 - 4) as f32 * 60.0, (i / 8 - 2) as f32 * 80.0);
            let id = f.state.add_node(pos, Size2::new(30.0, 30.0));
            f.node_labels.insert(id, format!("{}", i));
            nodes.push(id);
        }
        for i in 0..15 {
            f.state.add_edge(nodes[i], nodes[2 * i + 1], EdgeData::default());
            f.state.add_edge(nodes[i], nodes[2 * i + 2], EdgeData::default());
        }
        fixtures.push(f);
    }

    // 3. WEIGHTED
    {
        // Small: A - B [w=10], B - C [w=0.5], C - A [w=100]
        let mut f = GraphFixture::new("Weighted Small", "3-node network with strong/weak weights.");
        let a = f.state.add_node(Vec2::new(0.0, -50.0), Size2::new(30.0, 30.0));
        let b = f.state.add_node(Vec2::new(50.0, 50.0), Size2::new(30.0, 30.0));
        let c = f.state.add_node(Vec2::new(-50.0, 50.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        
        let e1 = f.state.add_edge(a, b, EdgeData::default());
        let e2 = f.state.add_edge(b, c, EdgeData::default());
        let e3 = f.state.add_edge(c, a, EdgeData::default());

        f.weights.insert(0, 10.0);
        f.weights.insert(1, 0.5);
        f.weights.insert(2, 100.0);
        f.edge_labels.insert(0, "w=10".to_string());
        f.edge_labels.insert(1, "w=0.5".to_string());
        f.edge_labels.insert(2, "w=100".to_string());
        let _ = (e1, e2, e3);
        fixtures.push(f);

        // Medium
        let mut f = GraphFixture::new("Weighted Medium", "Varying weights between 6 connected nodes.");
        let mut nodes = Vec::new();
        for i in 0..6 {
            let pos = Vec2::new(((i % 3) as f32 - 1.0) * 100.0, ((i / 3) as f32 - 0.5) * 100.0);
            let id = f.state.add_node(pos, Size2::new(30.0, 30.0));
            f.node_labels.insert(id, format!("N{}", i + 1));
            nodes.push(id);
        }
        let weighted_edges = vec![
            (0, 1, 5.0), (1, 2, 15.0), (2, 3, 2.0), (0, 3, 50.0),
            (1, 4, 8.0), (4, 3, 1.0), (2, 5, 20.0), (5, 4, 3.0)
        ];
        for (idx, &(u, v, w)) in weighted_edges.iter().enumerate() {
            f.state.add_edge(nodes[u], nodes[v], EdgeData::default());
            f.weights.insert(idx, w);
            f.edge_labels.insert(idx, format!("w={}", w));
        }
        fixtures.push(f);

        // Large (Hub Routing)
        let mut f = GraphFixture::new("Weighted Large (Hubs)", "Hub networks with backbones and local clusters.");
        let h1 = f.state.add_node(Vec2::new(-150.0, 0.0), Size2::new(40.0, 40.0));
        let h2 = f.state.add_node(Vec2::new(150.0, 0.0), Size2::new(40.0, 40.0));
        let h3 = f.state.add_node(Vec2::new(0.0, 150.0), Size2::new(40.0, 40.0));
        f.node_labels.insert(h1, "Hub1".to_string());
        f.node_labels.insert(h2, "Hub2".to_string());
        f.node_labels.insert(h3, "Hub3".to_string());

        let mut idx = 0;
        let mut add_hub_edge = |f: &mut GraphFixture<S>, u, v, w, name: &str| {
            f.state.add_edge(u, v, EdgeData::default());
            f.weights.insert(idx, w);
            f.edge_labels.insert(idx, format!("{}[w={}]", name, w));
            idx += 1;
        };

        // Hub backbones
        add_hub_edge(&mut f, h1, h2, 100.0, "Hub1-Hub2");
        add_hub_edge(&mut f, h2, h3, 150.0, "Hub2-Hub3");
        add_hub_edge(&mut f, h3, h1, 120.0, "Hub3-Hub1");

        // Hub 1 spokes
        let a = f.state.add_node(Vec2::new(-200.0, -50.0), Size2::new(30.0, 30.0));
        let b = f.state.add_node(Vec2::new(-220.0, 50.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        add_hub_edge(&mut f, h1, a, 10.0, "Hub1-A");
        add_hub_edge(&mut f, h1, b, 12.0, "Hub1-B");
        add_hub_edge(&mut f, a, b, 1.0, "A-B");

        // Hub 2 spokes
        let e = f.state.add_node(Vec2::new(200.0, -50.0), Size2::new(30.0, 30.0));
        let g = f.state.add_node(Vec2::new(220.0, 50.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(e, "E".to_string());
        f.node_labels.insert(g, "G".to_string());
        add_hub_edge(&mut f, h2, e, 5.0, "Hub2-E");
        add_hub_edge(&mut f, h2, g, 5.0, "Hub2-G");

        fixtures.push(f);
    }

    // 4. MULTIGRAPH
    {
        // Small: A -> B (e1), A -> B (e2), B -> A (e3)
        let mut f = GraphFixture::new("Multigraph Small", "Multiple parallel edges between two nodes.");
        let a = f.state.add_node(Vec2::new(-100.0, 0.0), Size2::new(30.0, 30.0));
        let b = f.state.add_node(Vec2::new(100.0, 0.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(b, a, EdgeData::default());
        f.edge_labels.insert(0, "e1".to_string());
        f.edge_labels.insert(1, "e2".to_string());
        f.edge_labels.insert(2, "e3".to_string());
        fixtures.push(f);

        // Medium
        let mut f = GraphFixture::new("Multigraph Medium", "Parallel pathways mapping traffic flow.");
        let a = f.state.add_node(Vec2::new(-100.0, -100.0), Size2::new(30.0, 30.0));
        let b = f.state.add_node(Vec2::new(100.0, -100.0), Size2::new(30.0, 30.0));
        let c = f.state.add_node(Vec2::new(100.0, 100.0), Size2::new(30.0, 30.0));
        let d = f.state.add_node(Vec2::new(-100.0, 100.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        f.node_labels.insert(d, "D".to_string());

        let multi_edges = vec![
            (a, b, "1"), (a, b, "2"), (a, b, "3"),
            (b, c, "4"), (b, c, "5"), (c, a, "6"),
            (c, d, "7"), (c, d, "8"), (d, a, "9"), (d, a, "10")
        ];
        for (idx, (u, v, label)) in multi_edges.into_iter().enumerate() {
            f.state.add_edge(u, v, EdgeData::default());
            f.edge_labels.insert(idx, format!("id={}", label));
        }
        fixtures.push(f);

        // Large
        let mut f = GraphFixture::new("Multigraph Large", "High density multigraph traffic routes.");
        let mut nodes = Vec::new();
        for i in 0..5 {
            let pos = Vec2::new(((i as f32) * 72.0 * 3.14 / 180.0).cos() * 80.0, ((i as f32) * 72.0 * 3.14 / 180.0).sin() * 80.0);
            let id = f.state.add_node(pos, Size2::new(30.0, 30.0));
            f.node_labels.insert(id, format!("N{}", i + 1));
            nodes.push(id);
        }
        let edges = vec![
            (0, 1, 1), (0, 1, 2), (0, 1, 3), (0, 1, 4), (0, 1, 5),
            (1, 2, 6), (1, 2, 7), (1, 2, 8),
            (2, 3, 9), (2, 3, 10),
            (3, 0, 11), (3, 0, 12),
            (0, 4, 13), (0, 4, 14),
            (4, 1, 15), (4, 1, 16),
            (4, 2, 17), (4, 2, 18),
            (4, 3, 19), (4, 3, 20)
        ];
        for (idx, &(u, v, id_val)) in edges.iter().enumerate() {
            f.state.add_edge(nodes[u], nodes[v], EdgeData::default());
            f.edge_labels.insert(idx, format!("id={}", id_val));
        }
        fixtures.push(f);
    }

    // 5. COMPOUND
    {
        // Small: Group1 { A, B }, Group1 -> C
        let mut f = GraphFixture::new("Compound Small", "A nested group with local sibling edges.");
        let g1 = f.state.add_node(Vec2::new(0.0, 0.0), Size2::new(100.0, 100.0));
        let a = f.state.add_node(Vec2::new(-20.0, 0.0), Size2::new(25.0, 25.0));
        let b = f.state.add_node(Vec2::new(20.0, 0.0), Size2::new(25.0, 25.0));
        let c = f.state.add_node(Vec2::new(120.0, 0.0), Size2::new(25.0, 25.0));
        f.node_labels.insert(g1, "Group1".to_string());
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());

        let _g1_idx = f.state.node_keys[g1];
        let a_idx = f.state.node_keys[a];
        let b_idx = f.state.node_keys[b];
        f.state.hierarchy.parent.set(a_idx, Some(g1));
        f.state.hierarchy.parent.set(b_idx, Some(g1));

        f.compound_groups.insert(g1, vec![a, b]);

        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(g1, c, EdgeData::default());
        fixtures.push(f);

        // Medium
        let mut f = GraphFixture::new("Compound Medium", "2 nested groups region networks connecting to external targets.");
        let reg_a = f.state.add_node(Vec2::new(-100.0, 0.0), Size2::new(150.0, 150.0));
        let reg_b = f.state.add_node(Vec2::new(100.0, 0.0), Size2::new(120.0, 120.0));
        let ext = f.state.add_node(Vec2::new(0.0, -150.0), Size2::new(30.0, 30.0));

        f.node_labels.insert(reg_a, "RegionA".to_string());
        f.node_labels.insert(reg_b, "RegionB".to_string());
        f.node_labels.insert(ext, "ExternalNode".to_string());

        let city1 = f.state.add_node(Vec2::new(-140.0, -20.0), Size2::new(25.0, 25.0));
        let city2 = f.state.add_node(Vec2::new(-100.0, 20.0), Size2::new(25.0, 25.0));
        let city3 = f.state.add_node(Vec2::new(-60.0, -20.0), Size2::new(25.0, 25.0));
        f.node_labels.insert(city1, "City1".to_string());
        f.node_labels.insert(city2, "City2".to_string());
        f.node_labels.insert(city3, "City3".to_string());

        let city4 = f.state.add_node(Vec2::new(80.0, 0.0), Size2::new(25.0, 25.0));
        let city5 = f.state.add_node(Vec2::new(120.0, 0.0), Size2::new(25.0, 25.0));
        f.node_labels.insert(city4, "City4".to_string());
        f.node_labels.insert(city5, "City5".to_string());

        let _r_a_idx = f.state.node_keys[reg_a];
        let _r_b_idx = f.state.node_keys[reg_b];
        f.state.hierarchy.parent.set(f.state.node_keys[city1], Some(reg_a));
        f.state.hierarchy.parent.set(f.state.node_keys[city2], Some(reg_a));
        f.state.hierarchy.parent.set(f.state.node_keys[city3], Some(reg_a));
        f.state.hierarchy.parent.set(f.state.node_keys[city4], Some(reg_b));
        f.state.hierarchy.parent.set(f.state.node_keys[city5], Some(reg_b));

        f.compound_groups.insert(reg_a, vec![city1, city2, city3]);
        f.compound_groups.insert(reg_b, vec![city4, city5]);

        f.state.add_edge(city1, city2, EdgeData::default());
        f.state.add_edge(city2, city4, EdgeData::default());
        f.state.add_edge(city3, reg_b, EdgeData::default());
        f.state.add_edge(reg_a, ext, EdgeData::default());
        fixtures.push(f);

        // Large (Deep nesting)
        let mut f = GraphFixture::new("Compound Large (Taxonomy)", "Hierarchy of deep nested country and continent nodes.");
        let global = f.state.add_node(Vec2::new(0.0, 0.0), Size2::new(400.0, 400.0));
        let na = f.state.add_node(Vec2::new(-100.0, 0.0), Size2::new(180.0, 180.0));
        let eu = f.state.add_node(Vec2::new(100.0, 0.0), Size2::new(180.0, 180.0));

        f.node_labels.insert(global, "Global".to_string());
        f.node_labels.insert(na, "NA".to_string());
        f.node_labels.insert(eu, "EU".to_string());

        let us = f.state.add_node(Vec2::new(-140.0, 0.0), Size2::new(80.0, 80.0));
        let can = f.state.add_node(Vec2::new(-60.0, 0.0), Size2::new(80.0, 80.0));
        f.node_labels.insert(us, "US".to_string());
        f.node_labels.insert(can, "CAN".to_string());

        let ny = f.state.add_node(Vec2::new(-150.0, -10.0), Size2::new(20.0, 20.0));
        let ca = f.state.add_node(Vec2::new(-130.0, 10.0), Size2::new(20.0, 20.0));
        f.node_labels.insert(ny, "NY".to_string());
        f.node_labels.insert(ca, "CA".to_string());

        // Set parents
        let set_parent = |f: &mut GraphFixture<S>, child, parent| {
            f.state.hierarchy.parent.set(f.state.node_keys[child], Some(parent));
        };
        set_parent(&mut f, na, global);
        set_parent(&mut f, eu, global);
        set_parent(&mut f, us, na);
        set_parent(&mut f, can, na);
        set_parent(&mut f, ny, us);
        set_parent(&mut f, ca, us);

        f.compound_groups.insert(global, vec![na, eu]);
        f.compound_groups.insert(na, vec![us, can]);
        f.compound_groups.insert(us, vec![ny, ca]);

        f.state.add_edge(ny, ca, EdgeData::default());
        fixtures.push(f);
    }

    // 6. HYPERGRAPH
    {
        // Small: E1: {A, B, C}, E2: {C, D}
        let mut f = GraphFixture::new("Hypergraph Small", "Hyperedges connecting multiple points via virtual centers.");
        let a = f.state.add_node(Vec2::new(-50.0, -50.0), Size2::new(30.0, 30.0));
        let b = f.state.add_node(Vec2::new(50.0, -50.0), Size2::new(30.0, 30.0));
        let c = f.state.add_node(Vec2::new(0.0, 50.0), Size2::new(30.0, 30.0));
        let d = f.state.add_node(Vec2::new(100.0, 50.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        f.node_labels.insert(d, "D".to_string());

        f.hyperedges.push(vec![a, b, c]);
        f.hyperedges.push(vec![c, d]);
        fixtures.push(f);

        // Medium
        let mut f = GraphFixture::new("Hypergraph Medium", "5 hyperedge loops intersecting across common vertices.");
        let mut nodes = Vec::new();
        for i in 0..9 {
            let pos = Vec2::new((i as f32 - 4.0) * 50.0, 0.0);
            let id = f.state.add_node(pos, Size2::new(25.0, 25.0));
            f.node_labels.insert(id, format!("N{}", i + 1));
            nodes.push(id);
        }
        f.hyperedges.push(vec![nodes[0], nodes[1], nodes[2]]);
        f.hyperedges.push(vec![nodes[2], nodes[3], nodes[4]]);
        f.hyperedges.push(vec![nodes[4], nodes[5], nodes[0]]);
        f.hyperedges.push(vec![nodes[6], nodes[7], nodes[8], nodes[0]]);
        fixtures.push(f);

        // Large
        let mut f = GraphFixture::new("Hypergraph Large", "Star and outlier rings connected via hypernodes.");
        let add_group = |f: &mut GraphFixture<S>, prefix: &str, count: usize| {
            let mut g = Vec::new();
            for i in 0..count {
                let id = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
                f.node_labels.insert(id, format!("{}{}", prefix, i + 1));
                g.push(id);
            }
            g
        };
        let a = add_group(&mut f, "A", 5);
        let b = add_group(&mut f, "B", 5);
        let c = add_group(&mut f, "C", 5);
        let d = add_group(&mut f, "D", 5);

        f.hyperedges.push(a);
        f.hyperedges.push(b);
        f.hyperedges.push(c);
        f.hyperedges.push(d);
        fixtures.push(f);
    }

    // 7. ATTRIBUTE NETWORK
    {
        // Small: A [color=red, size=10, type=user] -> B [color=blue, size=20, type=system]
        let mut f = GraphFixture::new("Attribute Small", "Nodes and edges enriched with custom attribute records.");
        let a = f.state.add_node(Vec2::new(-80.0, 0.0), Size2::new(30.0, 30.0));
        let b = f.state.add_node(Vec2::new(80.0, 0.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());

        let mut a_attrs = HashMap::new();
        a_attrs.insert("color".to_string(), "red".to_string());
        a_attrs.insert("size".to_string(), "10".to_string());
        a_attrs.insert("type".to_string(), "user".to_string());
        f.node_attributes.insert(a, a_attrs);

        let mut b_attrs = HashMap::new();
        b_attrs.insert("color".to_string(), "blue".to_string());
        b_attrs.insert("size".to_string(), "20".to_string());
        b_attrs.insert("type".to_string(), "system".to_string());
        f.node_attributes.insert(b, b_attrs);

        f.state.add_edge(a, b, EdgeData::default());
        let mut e_attrs = HashMap::new();
        e_attrs.insert("protocol".to_string(), "http".to_string());
        e_attrs.insert("secure".to_string(), "true".to_string());
        f.edge_attributes.insert(0, e_attrs);

        fixtures.push(f);

        // Medium: Shoppers to items (also bipartite attributes)
        let mut f = GraphFixture::new("Attribute Medium", "Regional clusters mapped to different regional offsets.");
        let u1 = f.state.add_node(Vec2::new(-100.0, -50.0), Size2::new(30.0, 30.0));
        let s1 = f.state.add_node(Vec2::new(100.0, -50.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(u1, "U1".to_string());
        f.node_labels.insert(s1, "S1".to_string());

        let mut u1_attrs = HashMap::new();
        u1_attrs.insert("age".to_string(), "25".to_string());
        u1_attrs.insert("tier".to_string(), "pro".to_string());
        f.node_attributes.insert(u1, u1_attrs);

        let mut s1_attrs = HashMap::new();
        s1_attrs.insert("cpu".to_string(), "80".to_string());
        s1_attrs.insert("region".to_string(), "us-east".to_string());
        f.node_attributes.insert(s1, s1_attrs);

        f.state.add_edge(u1, s1, EdgeData::default());
        fixtures.push(f);

        // Large (Log File Simulation)
        let mut f = GraphFixture::new("Attribute Large (Logs)", "System activity network tracking security threat profiles.");
        let ep1 = f.state.add_node(Vec2::new(0.0, -80.0), Size2::new(35.0, 35.0));
        let db = f.state.add_node(Vec2::new(0.0, 80.0), Size2::new(40.0, 40.0));
        f.node_labels.insert(ep1, "EP1".to_string());
        f.node_labels.insert(db, "DB".to_string());

        let mut ep_attrs = HashMap::new();
        ep_attrs.insert("service".to_string(), "login".to_string());
        f.node_attributes.insert(ep1, ep_attrs);

        let mut db_attrs = HashMap::new();
        db_attrs.insert("state".to_string(), "active".to_string());
        f.node_attributes.insert(db, db_attrs);

        let mut edge_idx = 0;
        for i in 1..20 {
            let threat = if i % 4 == 0 { "high" } else { "low" };
            let auth = if i % 4 == 0 { "false" } else { "true" };
            let pos = Vec2::new((i as f32 - 10.0) * 30.0, -180.0);
            let ip = f.state.add_node(pos, Size2::new(25.0, 25.0));
            f.node_labels.insert(ip, format!("IP{}", i));

            let mut ip_attrs = HashMap::new();
            ip_attrs.insert("threat".to_string(), threat.to_string());
            f.node_attributes.insert(ip, ip_attrs);

            f.state.add_edge(ip, ep1, EdgeData::default());
            let mut e_attrs = HashMap::new();
            e_attrs.insert("auth".to_string(), auth.to_string());
            f.edge_attributes.insert(edge_idx, e_attrs);
            edge_idx += 1;
        }
        fixtures.push(f);
    }

    // 8. CHART NODES
    {
        // Small: N1 [chart=pie] -> N2 [chart=bar]
        let mut f = GraphFixture::new("Chart Nodes Small", "Nodes carrying metric datasets for chart visualizations.");
        let n1 = f.state.add_node(Vec2::new(-80.0, 0.0), Size2::new(40.0, 40.0));
        let n2 = f.state.add_node(Vec2::new(80.0, 0.0), Size2::new(40.0, 40.0));
        f.node_labels.insert(n1, "N1".to_string());
        f.node_labels.insert(n2, "N2".to_string());

        let mut c1 = HashMap::new();
        c1.insert("apple".to_string(), 50.0);
        c1.insert("banana".to_string(), 50.0);
        f.chart_data.insert(n1, c1);

        let mut c2 = HashMap::new();
        c2.insert("Q1".to_string(), 10.0);
        c2.insert("Q2".to_string(), 20.0);
        f.chart_data.insert(n2, c2);

        f.state.add_edge(n1, n2, EdgeData::default());
        fixtures.push(f);

        // Medium
        let mut f = GraphFixture::new("Chart Nodes Medium", "System workload profiles with time-series line metrics.");
        let s_a = f.state.add_node(Vec2::new(-100.0, 50.0), Size2::new(40.0, 40.0));
        let lb = f.state.add_node(Vec2::new(0.0, 0.0), Size2::new(45.0, 45.0));
        f.node_labels.insert(s_a, "ServerA".to_string());
        f.node_labels.insert(lb, "LoadBalancer".to_string());

        let mut c_a = HashMap::new();
        c_a.insert("t1".to_string(), 10.0);
        c_a.insert("t2".to_string(), 15.0);
        f.chart_data.insert(s_a, c_a);

        let mut c_lb = HashMap::new();
        c_lb.insert("cpu".to_string(), 80.0);
        c_lb.insert("mem".to_string(), 60.0);
        f.chart_data.insert(lb, c_lb);

        f.state.add_edge(s_a, lb, EdgeData::default());
        fixtures.push(f);

        // Large
        let mut f = GraphFixture::new("Chart Nodes Large (Dashboard)", "Datacenter dashboard mapping load lines across core hubs.");
        let core = f.state.add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 50.0));
        f.node_labels.insert(core, "Core".to_string());

        let mut core_c = HashMap::new();
        core_c.insert("1pm".to_string(), 50.0);
        core_c.insert("2pm".to_string(), 80.0);
        f.chart_data.insert(core, core_c);

        for i in 1..10 {
            let angle = (i as f32) * std::f32::consts::TAU / 9.0;
            let pos = Vec2::new(angle.cos() * 120.0, angle.sin() * 120.0);
            let dc = f.state.add_node(pos, Size2::new(40.0, 40.0));
            f.node_labels.insert(dc, format!("Datacenter{}", i));

            let mut dc_c = HashMap::new();
            dc_c.insert("up".to_string(), 95.0 + (i % 5) as f32);
            dc_c.insert("down".to_string(), 5.0 - (i % 5) as f32);
            f.chart_data.insert(dc, dc_c);

            f.state.add_edge(dc, core, EdgeData::default());
        }
        fixtures.push(f);
    }

    // 9. SPARSE
    {
        // Small: Nodes: {A, B, C, D, E}, Edges: A - B, C - D
        let mut f = GraphFixture::new("Sparse Small", "Disconnected components with several isolated nodes.");
        let mut nodes = Vec::new();
        for name in &["A", "B", "C", "D", "E"] {
            let id = f.state.add_node(Vec2::default(), Size2::new(30.0, 30.0));
            f.node_labels.insert(id, name.to_string());
            nodes.push(id);
        }
        f.state.add_edge(nodes[0], nodes[1], EdgeData::default());
        f.state.add_edge(nodes[2], nodes[3], EdgeData::default());
        fixtures.push(f);

        // Medium
        let mut f = GraphFixture::new("Sparse Medium", "15 nodes with only 5 scattered links.");
        let mut nodes = Vec::new();
        for i in 0..15 {
            let id = f.state.add_node(Vec2::default(), Size2::new(30.0, 30.0));
            f.node_labels.insert(id, format!("{}", i + 1));
            nodes.push(id);
        }
        f.state.add_edge(nodes[0], nodes[1], EdgeData::default());
        f.state.add_edge(nodes[3], nodes[4], EdgeData::default());
        f.state.add_edge(nodes[7], nodes[8], EdgeData::default());
        f.state.add_edge(nodes[8], nodes[9], EdgeData::default());
        fixtures.push(f);

        // Large (Forest of small trees)
        let mut f = GraphFixture::new("Sparse Large (Forest)", "50 nodes forming a sparse forest of small tree structures.");
        let mut nodes = Vec::new();
        for i in 0..50 {
            let id = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(id, format!("N{}", i + 1));
            nodes.push(id);
        }
        f.state.add_edge(nodes[0], nodes[1], EdgeData::default());
        f.state.add_edge(nodes[0], nodes[2], EdgeData::default());
        f.state.add_edge(nodes[4], nodes[5], EdgeData::default());
        f.state.add_edge(nodes[6], nodes[7], EdgeData::default());
        f.state.add_edge(nodes[6], nodes[8], EdgeData::default());
        f.state.add_edge(nodes[6], nodes[9], EdgeData::default());
        f.state.add_edge(nodes[14], nodes[15], EdgeData::default());
        fixtures.push(f);
    }

    // 10. DENSE
    {
        // Small: Clique K4
        let mut f = GraphFixture::new("Dense Small (Clique K4)", "4 fully connected nodes (6 links total).");
        let mut nodes = Vec::new();
        for name in &["A", "B", "C", "D"] {
            let id = f.state.add_node(Vec2::default(), Size2::new(30.0, 30.0));
            f.node_labels.insert(id, name.to_string());
            nodes.push(id);
        }
        for i in 0..4 {
            for j in (i + 1)..4 {
                f.state.add_edge(nodes[i], nodes[j], EdgeData::default());
            }
        }
        fixtures.push(f);

        // Medium: Clique K8
        let mut f = GraphFixture::new("Dense Medium (Clique K8)", "8 fully connected nodes (28 links total).");
        let mut nodes = Vec::new();
        for i in 0..8 {
            let id = f.state.add_node(Vec2::default(), Size2::new(30.0, 30.0));
            f.node_labels.insert(id, format!("{}", i + 1));
            nodes.push(id);
        }
        for i in 0..8 {
            for j in (i + 1)..8 {
                f.state.add_edge(nodes[i], nodes[j], EdgeData::default());
            }
        }
        fixtures.push(f);

        // Large (Near-Clique Core)
        let mut f = GraphFixture::new("Dense Large", "20 nodes with heavy internal dense complete bipartite components.");
        let mut a_nodes = Vec::new();
        let mut b_nodes = Vec::new();
        for i in 0..10 {
            let aid = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(aid, format!("A{}", i + 1));
            a_nodes.push(aid);

            let bid = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(bid, format!("B{}", i + 1));
            b_nodes.push(bid);
        }
        // Connect all A nodes together
        for i in 0..10 {
            for j in (i + 1)..10 {
                f.state.add_edge(a_nodes[i], a_nodes[j], EdgeData::default());
            }
        }
        // Connect all B nodes together
        for i in 0..10 {
            for j in (i + 1)..10 {
                f.state.add_edge(b_nodes[i], b_nodes[j], EdgeData::default());
            }
        }
        // Connect all A nodes to all B nodes
        for i in 0..10 {
            for j in 0..10 {
                f.state.add_edge(a_nodes[i], b_nodes[j], EdgeData::default());
            }
        }
        fixtures.push(f);
    }

    // 11. DISCONNECTED
    {
        // Small: A-B, C-D (Two separate components)
        let mut f = GraphFixture::new("Disconnected Small", "2 isolated component pairs.");
        let a = f.state.add_node(Vec2::new(-80.0, 0.0), Size2::new(30.0, 30.0));
        let b = f.state.add_node(Vec2::new(-40.0, 0.0), Size2::new(30.0, 30.0));
        let c = f.state.add_node(Vec2::new(40.0, 0.0), Size2::new(30.0, 30.0));
        let d = f.state.add_node(Vec2::new(80.0, 0.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        f.node_labels.insert(d, "D".to_string());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(c, d, EdgeData::default());
        fixtures.push(f);

        // Medium
        let mut f = GraphFixture::new("Disconnected Medium", "4 separate components containing cyclic loops and singletons.");
        let add_node = |f: &mut GraphFixture<S>, name: &str| {
            let id = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(id, name.to_string());
            id
        };
        // Component 1: 1-2-3 cycle
        let n1 = add_node(&mut f, "1");
        let n2 = add_node(&mut f, "2");
        let n3 = add_node(&mut f, "3");
        f.state.add_edge(n1, n2, EdgeData::default());
        f.state.add_edge(n2, n3, EdgeData::default());
        f.state.add_edge(n3, n1, EdgeData::default());
        // Component 2: 4-5-6-7 cycle
        let n4 = add_node(&mut f, "4");
        let n5 = add_node(&mut f, "5");
        let n6 = add_node(&mut f, "6");
        let n7 = add_node(&mut f, "7");
        f.state.add_edge(n4, n5, EdgeData::default());
        f.state.add_edge(n5, n6, EdgeData::default());
        f.state.add_edge(n6, n7, EdgeData::default());
        f.state.add_edge(n7, n4, EdgeData::default());
        // Component 3: isolated 8
        add_node(&mut f, "8");
        // Component 4: 9-10 pair
        let n9 = add_node(&mut f, "9");
        let n10 = add_node(&mut f, "10");
        f.state.add_edge(n9, n10, EdgeData::default());
        fixtures.push(f);

        // Large
        let mut f = GraphFixture::new("Disconnected Large (Islands)", "Three large connected islands and floating singletons.");
        // Island A: 10 node circle
        let mut island_a = Vec::new();
        for i in 0..10 {
            let id = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(id, format!("A{}", i + 1));
            island_a.push(id);
        }
        for i in 0..10 {
            f.state.add_edge(island_a[i], island_a[(i + 1) % 10], EdgeData::default());
        }
        // Island B: star hub
        let hub = f.state.add_node(Vec2::default(), Size2::new(35.0, 35.0));
        f.node_labels.insert(hub, "B_Hub".to_string());
        for i in 0..10 {
            let leaf = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(leaf, format!("B{}", i + 1));
            f.state.add_edge(hub, leaf, EdgeData::default());
        }
        // Floatings
        for i in 0..10 {
            let id = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(id, format!("X{}", i + 1));
        }
        fixtures.push(f);
    }

    // 12. ACYCLIC
    {
        // Small: Root -> L1, Root -> R1, L1 -> L2
        let mut f = GraphFixture::new("Acyclic Small (Tree)", "Simple hierarchical tree with 4 nodes.");
        let root = f.state.add_node(Vec2::new(0.0, -80.0), Size2::new(30.0, 30.0));
        let l1 = f.state.add_node(Vec2::new(-60.0, 0.0), Size2::new(30.0, 30.0));
        let r1 = f.state.add_node(Vec2::new(60.0, 0.0), Size2::new(30.0, 30.0));
        let l2 = f.state.add_node(Vec2::new(-100.0, 80.0), Size2::new(30.0, 30.0));

        f.node_labels.insert(root, "Root".to_string());
        f.node_labels.insert(l1, "L1".to_string());
        f.node_labels.insert(r1, "R1".to_string());
        f.node_labels.insert(l2, "L2".to_string());

        f.state.hierarchy.parent.set(f.state.node_keys[l1], Some(root));
        f.state.hierarchy.parent.set(f.state.node_keys[r1], Some(root));
        f.state.hierarchy.parent.set(f.state.node_keys[l2], Some(l1));

        f.state.add_edge(root, l1, EdgeData::default());
        f.state.add_edge(root, r1, EdgeData::default());
        f.state.add_edge(l1, l2, EdgeData::default());
        fixtures.push(f);

        // Medium (Org Chart)
        let mut f = GraphFixture::new("Acyclic Medium (Org Chart)", "Hierarchical corporate reporting org structure.");
        let add_tree_node = |f: &mut GraphFixture<S>, name: &str, parent: Option<NodeId>| {
            let id = f.state.add_node(Vec2::default(), Size2::new(45.0, 30.0));
            f.node_labels.insert(id, name.to_string());
            if let Some(p) = parent {
                f.state.hierarchy.parent.set(f.state.node_keys[id], Some(p));
                f.state.add_edge(p, id, EdgeData::default());
            }
            id
        };
        let ceo = add_tree_node(&mut f, "CEO", None);
        let vp_sales = add_tree_node(&mut f, "VP_Sales", Some(ceo));
        let vp_eng = add_tree_node(&mut f, "VP_Eng", Some(ceo));
        let dir_m = add_tree_node(&mut f, "Dir_M", Some(vp_sales));
        let dir_s = add_tree_node(&mut f, "Dir_S", Some(vp_sales));
        let dev1 = add_tree_node(&mut f, "Dev1", Some(vp_eng));
        let dev2 = add_tree_node(&mut f, "Dev2", Some(vp_eng));
        let _ = (dir_m, dir_s, dev1, dev2);
        fixtures.push(f);

        // Large (Dependency Tree)
        let mut f = GraphFixture::new("Acyclic Large (Taxonomy)", "Hierarchical classification tree of taxonomic groups.");
        let animalia = f.state.add_node(Vec2::default(), Size2::new(45.0, 30.0));
        f.node_labels.insert(animalia, "Animalia".to_string());
        
        let chordata = add_tree_node(&mut f, "Chordata", Some(animalia));
        let mammalia = add_tree_node(&mut f, "Mammalia", Some(chordata));
        let primates = add_tree_node(&mut f, "Primates", Some(mammalia));
        let hominidae = add_tree_node(&mut f, "Hominidae", Some(primates));
        let homo = add_tree_node(&mut f, "Homo", Some(hominidae));
        let sapiens = add_tree_node(&mut f, "Sapiens", Some(homo));

        let carnivora = add_tree_node(&mut f, "Carnivora", Some(mammalia));
        let felidae = add_tree_node(&mut f, "Felidae", Some(carnivora));
        let panthera = add_tree_node(&mut f, "Panthera", Some(felidae));
        let leo = add_tree_node(&mut f, "Leo", Some(panthera));

        let arthropoda = add_tree_node(&mut f, "Arthropoda", Some(animalia));
        let insecta = add_tree_node(&mut f, "Insecta", Some(arthropoda));

        let _ = (sapiens, leo, insecta);
        fixtures.push(f);
    }

    // 13. CYCLIC
    {
        // Small: A -> B, B -> C, C -> A
        let mut f = GraphFixture::new("Cyclic Small (Cycle)", "3-node directed cycle loop.");
        let a = f.state.add_node(Vec2::new(0.0, -50.0), Size2::new(30.0, 30.0));
        let b = f.state.add_node(Vec2::new(50.0, 50.0), Size2::new(30.0, 30.0));
        let c = f.state.add_node(Vec2::new(-50.0, 50.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(b, c, EdgeData::default());
        f.state.add_edge(c, a, EdgeData::default());
        fixtures.push(f);

        // Medium
        let mut f = GraphFixture::new("Cyclic Medium (Interlocked)", "Three interconnected directed cycle rings.");
        let mut nodes = Vec::new();
        for i in 0..8 {
            let pos = Vec2::new(((i as f32) * 45.0 * 3.14 / 180.0).cos() * 80.0, ((i as f32) * 45.0 * 3.14 / 180.0).sin() * 80.0);
            let id = f.state.add_node(pos, Size2::new(30.0, 30.0));
            f.node_labels.insert(id, format!("N{}", i + 1));
            nodes.push(id);
        }
        let edges = vec![
            (0, 1), (1, 2), (2, 3), (3, 0), // Ring 1
            (1, 4), (4, 5), (5, 1),         // Ring 2
            (3, 6), (6, 7), (7, 3)          // Ring 3
        ];
        for (u, v) in edges {
            f.state.add_edge(nodes[u], nodes[v], EdgeData::default());
        }
        fixtures.push(f);

        // Large (Metabolic loops)
        let mut f = GraphFixture::new("Cyclic Large (Metabolic)", "Metabolic pathway featuring positive/negative feedback loops.");
        let mut nodes = Vec::new();
        for i in 0..20 {
            let pos = Vec2::new((i as f32 - 10.0) * 20.0, ((i % 2) as f32 - 0.5) * 60.0);
            let id = f.state.add_node(pos, Size2::new(30.0, 30.0));
            f.node_labels.insert(id, format!("M{}", i + 1));
            nodes.push(id);
        }
        // Chain core cyclic loop
        for i in 0..10 {
            f.state.add_edge(nodes[i], nodes[(i + 1) % 10], EdgeData::default());
        }
        // Sub-loop
        f.state.add_edge(nodes[1], nodes[10], EdgeData::default());
        f.state.add_edge(nodes[10], nodes[11], EdgeData::default());
        f.state.add_edge(nodes[11], nodes[1], EdgeData::default());
        // Feedback
        f.state.add_edge(nodes[9], nodes[0], EdgeData::default());
        fixtures.push(f);
    }

    // 14. SCALE-FREE
    {
        // Small: Hub - A, Hub - B, Hub - C, Hub - D
        let mut f = GraphFixture::new("Scale-Free Small (Star)", "Single large hub node with 4 peripheral leaf spokes.");
        let hub = f.state.add_node(Vec2::new(0.0, 0.0), Size2::new(45.0, 45.0));
        f.node_labels.insert(hub, "Hub".to_string());
        for name in &["A", "B", "C", "D"] {
            let leaf = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(leaf, name.to_string());
            f.state.add_edge(hub, leaf, EdgeData::default());
        }
        fixtures.push(f);

        // Medium
        let mut f = GraphFixture::new("Scale-Free Medium (Hubs)", "Two interconnected major hubs with local clusters.");
        let shub = f.state.add_node(Vec2::new(-80.0, 0.0), Size2::new(50.0, 50.0));
        let hub_b = f.state.add_node(Vec2::new(80.0, 0.0), Size2::new(40.0, 40.0));
        f.node_labels.insert(shub, "SuperHub".to_string());
        f.node_labels.insert(hub_b, "HubB".to_string());

        f.state.add_edge(shub, hub_b, EdgeData::default());

        for i in 1..6 {
            let leaf = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(leaf, format!("Node{}", i));
            f.state.add_edge(shub, leaf, EdgeData::default());
        }
        for i in 6..9 {
            let leaf = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(leaf, format!("Node{}", i));
            f.state.add_edge(hub_b, leaf, EdgeData::default());
        }
        fixtures.push(f);

        // Large (Social Network Influencers)
        let mut f = GraphFixture::new("Scale-Free Large (Social)", "Social network simulation showing power-law degree distribution.");
        let inf1 = f.state.add_node(Vec2::new(-150.0, 0.0), Size2::new(55.0, 55.0));
        let inf2 = f.state.add_node(Vec2::new(150.0, 0.0), Size2::new(50.0, 50.0));
        f.node_labels.insert(inf1, "Influencer1".to_string());
        f.node_labels.insert(inf2, "Influencer2".to_string());
        f.state.add_edge(inf1, inf2, EdgeData::default());

        for i in 1..25 {
            let user = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(user, format!("User{}", i));
            f.state.add_edge(inf1, user, EdgeData::default());
        }
        for i in 25..40 {
            let user = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(user, format!("User{}", i));
            f.state.add_edge(inf2, user, EdgeData::default());
        }
        fixtures.push(f);
    }

    // 15. BIPARTITE
    {
        // Small: SetU: {U1, U2}, SetV: {V1, V2}. Edges: U1-V1, U1-V2, U2-V1
        let mut f = GraphFixture::new("Bipartite Small", "Simple 2-vs-2 bipartite matching partition.");
        let u1 = f.state.add_node(Vec2::new(-80.0, -40.0), Size2::new(30.0, 30.0));
        let u2 = f.state.add_node(Vec2::new(-80.0, 40.0), Size2::new(30.0, 30.0));
        let v1 = f.state.add_node(Vec2::new(80.0, -40.0), Size2::new(30.0, 30.0));
        let v2 = f.state.add_node(Vec2::new(80.0, 40.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(u1, "U1".to_string());
        f.node_labels.insert(u2, "U2".to_string());
        f.node_labels.insert(v1, "V1".to_string());
        f.node_labels.insert(v2, "V2".to_string());

        f.state.add_edge(u1, v1, EdgeData::default());
        f.state.add_edge(u1, v2, EdgeData::default());
        f.state.add_edge(u2, v1, EdgeData::default());
        fixtures.push(f);

        // Medium (Shopper to Product)
        let mut f = GraphFixture::new("Bipartite Medium (Shoppers)", "Bipartite graph mapping Shoppers to purchased Products.");
        let mut shoppers = Vec::new();
        let mut products = Vec::new();
        for i in 1..6 {
            let sid = f.state.add_node(Vec2::new(-100.0, (i - 3) as f32 * 50.0), Size2::new(30.0, 30.0));
            f.node_labels.insert(sid, format!("S{}", i));
            shoppers.push(sid);

            let pid = f.state.add_node(Vec2::new(100.0, (i - 3) as f32 * 50.0), Size2::new(30.0, 30.0));
            f.node_labels.insert(pid, format!("P{}", i));
            products.push(pid);
        }
        let purchases = vec![
            (0, 0), (0, 2), (1, 0), (1, 1), (1, 4),
            (2, 3), (3, 0), (3, 3), (4, 2), (4, 4)
        ];
        for (u, v) in purchases {
            f.state.add_edge(shoppers[u], products[v], EdgeData::default());
        }
        fixtures.push(f);

        // Large (Tripartite Layers)
        let mut f = GraphFixture::new("Bipartite Large (Tripartite)", "Machine learning network mapping Input -> Hidden -> Output layers.");
        let mut inputs = Vec::new();
        let mut hiddens = Vec::new();
        let mut outputs = Vec::new();

        for i in 1..11 {
            let id = f.state.add_node(Vec2::new(-200.0, (i - 5) as f32 * 40.0), Size2::new(25.0, 25.0));
            f.node_labels.insert(id, format!("I{}", i));
            inputs.push(id);
        }
        for i in 1..11 {
            let id = f.state.add_node(Vec2::new(0.0, (i - 5) as f32 * 40.0), Size2::new(25.0, 25.0));
            f.node_labels.insert(id, format!("H{}", i));
            hiddens.push(id);
        }
        for i in 1..6 {
            let id = f.state.add_node(Vec2::new(200.0, (i - 3) as f32 * 60.0), Size2::new(25.0, 25.0));
            f.node_labels.insert(id, format!("O{}", i));
            outputs.push(id);
        }

        // Full bipartite Input -> Hidden
        for &u in &inputs {
            for &v in &hiddens {
                f.state.add_edge(u, v, EdgeData::default());
            }
        }
        // Full bipartite Hidden -> Output
        for &u in &hiddens {
            for &v in &outputs {
                f.state.add_edge(u, v, EdgeData::default());
            }
        }
        fixtures.push(f);
    }

    fixtures
}
