use super::GraphFixture;
use graphene_core::{EdgeData, Size2, Vec2};
use std::collections::HashMap;

pub fn add_advanced_fixtures<S: Copy + Default>(fixtures: &mut Vec<GraphFixture<S>>) {
    // 6. ATTRIBUTE NETWORK
    {
        let mut f = GraphFixture::new(
            "Attribute Small",
            "Nodes and edges enriched with custom attribute records.",
        );
        let a = f
            .state
            .add_node(Vec2::new(-80.0, 0.0), Size2::new(30.0, 30.0));
        let b = f
            .state
            .add_node(Vec2::new(80.0, 0.0), Size2::new(30.0, 30.0));
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
    }

    // 7. CHART NODES
    {
        let mut f = GraphFixture::new(
            "Chart Nodes Small",
            "Nodes carrying metric datasets for chart visualizations.",
        );
        let n1 = f
            .state
            .add_node(Vec2::new(-80.0, 0.0), Size2::new(40.0, 40.0));
        let n2 = f
            .state
            .add_node(Vec2::new(80.0, 0.0), Size2::new(40.0, 40.0));
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
    }

    // 8. SPARSE
    {
        let mut f = GraphFixture::new(
            "Sparse Small",
            "Disconnected components with several isolated nodes.",
        );
        let mut nodes = Vec::new();
        for name in &["A", "B", "C", "D", "E"] {
            let id = f.state.add_node(Vec2::default(), Size2::new(30.0, 30.0));
            f.node_labels.insert(id, name.to_string());
            nodes.push(id);
        }
        f.state.add_edge(nodes[0], nodes[1], EdgeData::default());
        f.state.add_edge(nodes[2], nodes[3], EdgeData::default());
        fixtures.push(f);
    }

    // 9. DENSE
    {
        let mut f = GraphFixture::new(
            "Dense Small (Clique K4)",
            "4 fully connected nodes (6 links total).",
        );
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
    }

    // 10. DISCONNECTED
    {
        let mut f = GraphFixture::new("Disconnected Small", "2 isolated component pairs.");
        let a = f
            .state
            .add_node(Vec2::new(-80.0, 0.0), Size2::new(30.0, 30.0));
        let b = f
            .state
            .add_node(Vec2::new(-40.0, 0.0), Size2::new(30.0, 30.0));
        let c = f
            .state
            .add_node(Vec2::new(40.0, 0.0), Size2::new(30.0, 30.0));
        let d = f
            .state
            .add_node(Vec2::new(80.0, 0.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(a, "A".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        f.node_labels.insert(d, "D".to_string());
        f.state.add_edge(a, b, EdgeData::default());
        f.state.add_edge(c, d, EdgeData::default());
        fixtures.push(f);
    }

    // 11. ACYCLIC
    {
        let mut f = GraphFixture::new(
            "Acyclic Small (Tree)",
            "Simple hierarchical tree with 4 nodes.",
        );
        let root = f
            .state
            .add_node(Vec2::new(0.0, -80.0), Size2::new(30.0, 30.0));
        let l1 = f
            .state
            .add_node(Vec2::new(-60.0, 0.0), Size2::new(30.0, 30.0));
        let r1 = f
            .state
            .add_node(Vec2::new(60.0, 0.0), Size2::new(30.0, 30.0));
        let l2 = f
            .state
            .add_node(Vec2::new(-100.0, 80.0), Size2::new(30.0, 30.0));

        f.node_labels.insert(root, "Root".to_string());
        f.node_labels.insert(l1, "L1".to_string());
        f.node_labels.insert(r1, "R1".to_string());
        f.node_labels.insert(l2, "L2".to_string());

        f.state.reparent_node(l1, Some(root));
        f.state.reparent_node(r1, Some(root));
        f.state.reparent_node(l2, Some(l1));

        f.state.add_edge(root, l1, EdgeData::default());
        f.state.add_edge(root, r1, EdgeData::default());
        f.state.add_edge(l1, l2, EdgeData::default());
        fixtures.push(f);
    }

    // 12. CYCLIC
    {
        let mut f = GraphFixture::new("Cyclic Small (Cycle)", "3-node directed cycle loop.");
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
    }

    // 13. SCALE-FREE
    {
        let mut f = GraphFixture::new(
            "Scale-Free Small (Star)",
            "Single large hub node with 4 peripheral leaf spokes.",
        );
        let hub = f
            .state
            .add_node(Vec2::new(0.0, 0.0), Size2::new(45.0, 45.0));
        f.node_labels.insert(hub, "Hub".to_string());
        for name in &["A", "B", "C", "D"] {
            let leaf = f.state.add_node(Vec2::default(), Size2::new(25.0, 25.0));
            f.node_labels.insert(leaf, name.to_string());
            f.state.add_edge(hub, leaf, EdgeData::default());
        }
        fixtures.push(f);
    }

    // 14. BIPARTITE
    {
        let mut f = GraphFixture::new(
            "Bipartite Small",
            "Simple 2-vs-2 bipartite matching partition.",
        );
        let u1 = f
            .state
            .add_node(Vec2::new(-80.0, -40.0), Size2::new(30.0, 30.0));
        let u2 = f
            .state
            .add_node(Vec2::new(-80.0, 40.0), Size2::new(30.0, 30.0));
        let v1 = f
            .state
            .add_node(Vec2::new(80.0, -40.0), Size2::new(30.0, 30.0));
        let v2 = f
            .state
            .add_node(Vec2::new(80.0, 40.0), Size2::new(30.0, 30.0));
        f.node_labels.insert(u1, "U1".to_string());
        f.node_labels.insert(u2, "U2".to_string());
        f.node_labels.insert(v1, "V1".to_string());
        f.node_labels.insert(v2, "V2".to_string());

        f.state.add_edge(u1, v1, EdgeData::default());
        f.state.add_edge(u1, v2, EdgeData::default());
        f.state.add_edge(u2, v1, EdgeData::default());
        fixtures.push(f);
    }

    // 15. FILE SYSTEM TREE
    {
        let mut f = GraphFixture::new(
            "Workspace File Tree",
            "Real file system hierarchy read from the 'crates' directory.",
        );
        let root_path = std::path::Path::new("crates");
        if root_path.exists() {
            super::add_dir_to_fixture(&mut f, root_path, None, 3);
        } else {
            let root = f
                .state
                .add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 30.0));
            f.node_labels.insert(root, "Root".to_string());
            let src = f
                .state
                .add_node(Vec2::new(0.0, 0.0), Size2::new(40.0, 30.0));
            f.node_labels.insert(src, "src".to_string());
            f.state.add_edge(root, src, EdgeData::default());
        }
        fixtures.push(f);
    }
}
