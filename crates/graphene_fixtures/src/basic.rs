use super::GraphFixture;
use graphene_core::{EdgeData, Size2, Vec2};
use std::collections::HashMap;

pub fn add_basic_fixtures<S: Copy + Default>(fixtures: &mut Vec<GraphFixture<S>>) {
    // 1. UNDIRECTED
    {
        // Small: A - B, B - C, C - A
        let mut f = GraphFixture::new("Undirected Small (Cycle)", "3-node simple cycle.");
        f.is_directed = false;
        let a = f
            .state
            .add_node(Vec2::new(0.0, -50.0), Size2::new(30.0, 30.0));
        let b = f
            .state
            .add_node(Vec2::new(50.0, 50.0), Size2::new(30.0, 30.0));
        let c = f
            .state
            .add_node(Vec2::new(-50.0, 50.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(b, c, EdgeData::default());
        f.state.add_edge(c, a, EdgeData::default());
        fixtures.push(f);

        // Medium: Petersen Graph structure
        let mut f = GraphFixture::new(
            "Undirected Medium (Petersen)",
            "10 nodes, 15 edges Petersen graph structure.",
        );
        f.is_directed = false;
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
            (0, 1),
            (1, 2),
            (2, 3),
            (3, 4),
            (4, 0), // Outer cycle
            (0, 5),
            (1, 6),
            (2, 7),
            (3, 8),
            (4, 9), // Spoke edges
            (5, 7),
            (7, 9),
            (9, 6),
            (6, 8),
            (8, 5), // Inner star
        ];
        for (u, v) in edges {
            f.state.add_edge(nodes[u], nodes[v], EdgeData::default());
        }
        fixtures.push(f);

        // Large: Grid-like Mesh
        let mut f = GraphFixture::new(
            "Undirected Large (Grid)",
            "5x5 grid mesh containing 25 nodes and 40 edges.",
        );
        f.is_directed = false;
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
                    f.state
                        .add_edge(nodes[idx], nodes[idx + 1], EdgeData::default());
                }
                if r < 4 {
                    f.state
                        .add_edge(nodes[idx], nodes[idx + 5], EdgeData::default());
                }
            }
        }
        fixtures.push(f);
    }

    // 2. DIRECTED
    {
        // Small: A -> B, A -> C, B -> C
        let mut f = GraphFixture::new("Directed Small", "Feed-forward loop with 3 nodes.");
        let a = f
            .state
            .add_node(Vec2::new(0.0, -60.0), Size2::new(30.0, 30.0));
        let b = f
            .state
            .add_node(Vec2::new(50.0, 0.0), Size2::new(30.0, 30.0));
        let c = f
            .state
            .add_node(Vec2::new(-50.0, 60.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(a, c, EdgeData::default());
        f.state.add_edge(b, c, EdgeData::default());
        fixtures.push(f);

        // Medium: Process Flow
        let mut f = GraphFixture::new(
            "Directed Medium (Process)",
            "Process flow loop containing 8 nodes.",
        );
        let names = vec![
            "Start", "Step1", "Step2a", "Step2b", "Step3", "Approval", "End",
        ];
        let mut nodes = HashMap::new();
        for (idx, name) in names.iter().enumerate() {
            let pos = Vec2::new((idx as f32 - 3.0) * 80.0, 0.0);
            let id = f.state.add_node(pos, Size2::new(45.0, 30.0));
            f.node_labels.insert(id, name.to_string());
            nodes.insert(*name, id);
        }
        let flow = vec![
            ("Start", "Step1"),
            ("Step1", "Step2a"),
            ("Step1", "Step2b"),
            ("Step2a", "Step3"),
            ("Step2b", "Step3"),
            ("Step3", "Approval"),
            ("Approval", "End"),
            ("Approval", "Step1"),
        ];
        for (u, v) in flow {
            f.state.add_edge(nodes[u], nodes[v], EdgeData::default());
        }
        fixtures.push(f);

        // Large: Deep Cascade
        let mut f = GraphFixture::new(
            "Directed Large (Cascade)",
            "Highly layered binary cascade flow network.",
        );
        let mut nodes = Vec::new();
        for i in 0..32 {
            let pos = Vec2::new((i % 8 - 4) as f32 * 60.0, (i / 8 - 2) as f32 * 80.0);
            let id = f.state.add_node(pos, Size2::new(30.0, 30.0));
            f.node_labels.insert(id, format!("{}", i));
            nodes.push(id);
        }
        for i in 0..15 {
            f.state
                .add_edge(nodes[i], nodes[2 * i + 1], EdgeData::default());
            f.state
                .add_edge(nodes[i], nodes[2 * i + 2], EdgeData::default());
        }
        fixtures.push(f);
    }

    // 3. WEIGHTED
    {
        let mut f = GraphFixture::new("Weighted Small", "3-node network with strong/weak weights.");
        let a = f
            .state
            .add_node(Vec2::new(0.0, -50.0), Size2::new(30.0, 30.0));
        let b = f
            .state
            .add_node(Vec2::new(50.0, 50.0), Size2::new(30.0, 30.0));
        let c = f
            .state
            .add_node(Vec2::new(-50.0, 50.0), Size2::new(30.0, 30.0));
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
        let mut f = GraphFixture::new(
            "Weighted Medium",
            "Varying weights between 6 connected nodes.",
        );
        let mut nodes = Vec::new();
        for i in 0..6 {
            let pos = Vec2::new(
                ((i % 3) as f32 - 1.0) * 100.0,
                ((i / 3) as f32 - 0.5) * 100.0,
            );
            let id = f.state.add_node(pos, Size2::new(30.0, 30.0));
            f.node_labels.insert(id, format!("N{}", i + 1));
            nodes.push(id);
        }
        let weighted_edges = vec![
            (0, 1, 5.0),
            (1, 2, 15.0),
            (2, 3, 2.0),
            (0, 3, 50.0),
            (1, 4, 8.0),
            (4, 3, 1.0),
            (2, 5, 20.0),
            (5, 4, 3.0),
        ];
        for (idx, &(u, v, w)) in weighted_edges.iter().enumerate() {
            f.state.add_edge(nodes[u], nodes[v], EdgeData::default());
            f.weights.insert(idx, w);
            f.edge_labels.insert(idx, format!("w={}", w));
        }
        fixtures.push(f);

        // Large (Hub Routing)
        let mut f = GraphFixture::new(
            "Weighted Large (Hubs)",
            "Hub networks with backbones and local clusters.",
        );
        let h1 = f
            .state
            .add_node(Vec2::new(-150.0, 0.0), Size2::new(40.0, 40.0));
        let h2 = f
            .state
            .add_node(Vec2::new(150.0, 0.0), Size2::new(40.0, 40.0));
        let h3 = f
            .state
            .add_node(Vec2::new(0.0, 150.0), Size2::new(40.0, 40.0));
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

        add_hub_edge(&mut f, h1, h2, 100.0, "Hub1-Hub2");
        add_hub_edge(&mut f, h2, h3, 150.0, "Hub2-Hub3");
        add_hub_edge(&mut f, h3, h1, 120.0, "Hub3-Hub1");

        let a = f
            .state
            .add_node(Vec2::new(-200.0, -50.0), Size2::new(30.0, 30.0));
        let b = f
            .state
            .add_node(Vec2::new(-220.0, 50.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        add_hub_edge(&mut f, h1, a, 10.0, "Hub1-A");
        add_hub_edge(&mut f, h1, b, 12.0, "Hub1-B");
        add_hub_edge(&mut f, a, b, 1.0, "A-B");

        let e = f
            .state
            .add_node(Vec2::new(200.0, -50.0), Size2::new(30.0, 30.0));
        let g = f
            .state
            .add_node(Vec2::new(220.0, 50.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(e, "E".to_string());
        f.node_labels.insert(g, "G".to_string());
        add_hub_edge(&mut f, h2, e, 5.0, "Hub2-E");
        add_hub_edge(&mut f, h2, g, 5.0, "Hub2-G");

        fixtures.push(f);
    }

    // 4. MULTIGRAPH
    {
        let mut f = GraphFixture::new(
            "Multigraph Small",
            "Multiple parallel edges between two nodes.",
        );
        let a = f
            .state
            .add_node(Vec2::new(-100.0, 0.0), Size2::new(30.0, 30.0));
        let b = f
            .state
            .add_node(Vec2::new(100.0, 0.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(b, a, EdgeData::default());
        f.edge_labels.insert(0, "e1".to_string());
        f.edge_labels.insert(1, "e2".to_string());
        f.edge_labels.insert(2, "e3".to_string());
        fixtures.push(f);
    }

    // 5. COMPOUND & HYPERGRAPH
    {
        let mut f = GraphFixture::new("Compound Small", "Parent group with 2 child nodes.");
        let p = f
            .state
            .add_node(Vec2::new(0.0, 0.0), Size2::new(120.0, 80.0));
        let c1 = f
            .state
            .add_node(Vec2::new(-30.0, 0.0), Size2::new(30.0, 30.0));
        let c2 = f
            .state
            .add_node(Vec2::new(30.0, 0.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(p, "Parent".to_string());
        f.node_labels.insert(c1, "Child1".to_string());
        f.node_labels.insert(c2, "Child2".to_string());
        f.state.reparent_node(c1, Some(p));
        f.state.reparent_node(c2, Some(p));
        f.compound_groups.insert(p, vec![c1, c2]);
        f.state.add_edge(c1, c2, EdgeData::default());
        fixtures.push(f);

        let mut f = GraphFixture::new("Hypergraph Small", "Hyperedge spanning 3 nodes.");
        let n1 = f
            .state
            .add_node(Vec2::new(-60.0, -40.0), Size2::new(30.0, 30.0));
        let n2 = f
            .state
            .add_node(Vec2::new(60.0, -40.0), Size2::new(30.0, 30.0));
        let n3 = f
            .state
            .add_node(Vec2::new(0.0, 40.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(n1, "N1".to_string());
        f.node_labels.insert(n2, "N2".to_string());
        f.node_labels.insert(n3, "N3".to_string());
        f.hyperedges.push(vec![n1, n2, n3]);
        fixtures.push(f);
    }
}
