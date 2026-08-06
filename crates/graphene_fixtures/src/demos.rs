use super::GraphFixture;
use graphene_core::{EdgeData, Size2, Vec2};

pub fn add_cytoscape_demos<S: Copy + Default>(fixtures: &mut Vec<GraphFixture<S>>) {
    // 1. COMPOUND NODES DEMO
    {
        let mut f = GraphFixture::new(
            "Demo: Compound Nodes",
            "Nested parent/child node hierarchy (Cytoscape Compound Demo).",
        );
        let b = f
            .state
            .add_node(Vec2::new(250.0, 85.0), Size2::new(180.0, 100.0));
        let a = f
            .state
            .add_node(Vec2::new(215.0, 85.0), Size2::new(40.0, 40.0));
        let c = f
            .state
            .add_node(Vec2::new(300.0, 85.0), Size2::new(40.0, 40.0));

        let d = f
            .state
            .add_node(Vec2::new(215.0, 175.0), Size2::new(40.0, 40.0));

        let e = f
            .state
            .add_node(Vec2::new(300.0, 175.0), Size2::new(80.0, 80.0));
        let fl = f
            .state
            .add_node(Vec2::new(300.0, 175.0), Size2::new(40.0, 40.0));

        f.node_labels.insert(b, "Parent B".to_string());
        f.node_labels.insert(a, "Node A".to_string());
        f.node_labels.insert(c, "Node C".to_string());
        f.node_labels.insert(d, "Node D".to_string());
        f.node_labels.insert(e, "Parent E".to_string());
        f.node_labels.insert(fl, "Node F".to_string());

        f.state.reparent_node(a, Some(b));
        f.state.reparent_node(c, Some(b));
        f.state.reparent_node(fl, Some(e));

        f.compound_groups.insert(b, vec![a, c]);
        f.compound_groups.insert(e, vec![fl]);

        f.state.add_edge(a, d, EdgeData::default());
        f.state.add_edge(e, b, EdgeData::default());

        fixtures.push(f);
    }

    // 2. ARCHITECTURE DEMO
    {
        let mut f = GraphFixture::new(
            "Demo: System Architecture",
            "Multi-level modular system architecture diagram with compound components.",
        );
        let cy = f
            .state
            .add_node(Vec2::new(0.0, 0.0), Size2::new(450.0, 350.0));
        let api = f
            .state
            .add_node(Vec2::new(-80.0, 0.0), Size2::new(200.0, 200.0));
        let ext = f
            .state
            .add_node(Vec2::new(180.0, 0.0), Size2::new(160.0, 300.0));
        let app = f
            .state
            .add_node(Vec2::new(0.0, 280.0), Size2::new(80.0, 50.0));

        f.node_labels.insert(cy, "Graphene Engine".to_string());
        f.node_labels.insert(api, "Core API".to_string());
        f.node_labels.insert(ext, "Extensions".to_string());
        f.node_labels.insert(app, "Client App".to_string());

        f.state.reparent_node(api, Some(cy));
        f.state.reparent_node(ext, Some(cy));

        // API Children
        let core = f
            .state
            .add_node(Vec2::new(-120.0, -40.0), Size2::new(60.0, 40.0));
        let eles = f
            .state
            .add_node(Vec2::new(-40.0, -40.0), Size2::new(60.0, 40.0));
        let style = f
            .state
            .add_node(Vec2::new(-120.0, 40.0), Size2::new(60.0, 40.0));
        let selector = f
            .state
            .add_node(Vec2::new(-40.0, 40.0), Size2::new(60.0, 40.0));

        f.node_labels.insert(core, "Core".to_string());
        f.node_labels.insert(eles, "Collection".to_string());
        f.node_labels.insert(style, "Style".to_string());
        f.node_labels.insert(selector, "Selector".to_string());

        for &child in &[core, eles, style, selector] {
            f.state.reparent_node(child, Some(api));
        }

        // Ext Children
        let layout = f
            .state
            .add_node(Vec2::new(180.0, -60.0), Size2::new(70.0, 35.0));
        let renderer = f
            .state
            .add_node(Vec2::new(180.0, 20.0), Size2::new(70.0, 35.0));
        let algo = f
            .state
            .add_node(Vec2::new(180.0, 100.0), Size2::new(70.0, 35.0));

        f.node_labels.insert(layout, "Layout".to_string());
        f.node_labels.insert(renderer, "Renderer".to_string());
        f.node_labels.insert(algo, "Algorithms".to_string());

        for &child in &[layout, renderer, algo] {
            f.state.reparent_node(child, Some(ext));
        }

        // Connections
        f.state.add_edge(core, eles, EdgeData::default());
        f.state.add_edge(core, style, EdgeData::default());
        f.state.add_edge(style, selector, EdgeData::default());
        f.state.add_edge(core, selector, EdgeData::default());
        f.state.add_edge(app, api, EdgeData::default());
        f.state.add_edge(app, ext, EdgeData::default());
        f.state.add_edge(layout, api, EdgeData::default());
        f.state.add_edge(renderer, api, EdgeData::default());

        fixtures.push(f);
    }

    // 3. ANIMATED BFS DEMO
    {
        let mut f = GraphFixture::new(
            "Demo: Animated BFS Traversal",
            "5-node weighted graph for Breadth-First Search step-by-step traversal.",
        );
        let a = f
            .state
            .add_node(Vec2::new(0.0, -100.0), Size2::new(40.0, 40.0));
        let b = f
            .state
            .add_node(Vec2::new(100.0, -40.0), Size2::new(40.0, 40.0));
        let c = f
            .state
            .add_node(Vec2::new(80.0, 80.0), Size2::new(40.0, 40.0));
        let d = f
            .state
            .add_node(Vec2::new(-80.0, 80.0), Size2::new(40.0, 40.0));
        let e = f
            .state
            .add_node(Vec2::new(-100.0, -40.0), Size2::new(40.0, 40.0));

        f.node_labels.insert(a, "Start (A)".to_string());
        f.node_labels.insert(b, "B".to_string());
        f.node_labels.insert(c, "C".to_string());
        f.node_labels.insert(d, "D".to_string());
        f.node_labels.insert(e, "E".to_string());

        let edges = vec![
            (a, e, 1.0),
            (a, b, 3.0),
            (b, e, 4.0),
            (b, c, 5.0),
            (c, e, 6.0),
            (c, d, 2.0),
            (d, e, 7.0),
        ];

        for (idx, (u, v, w)) in edges.into_iter().enumerate() {
            f.state.add_edge(u, v, EdgeData::default());
            f.weights.insert(idx, w);
            f.edge_labels.insert(idx, format!("w={:.0}", w));
        }

        fixtures.push(f);
    }

    // 4. EDGE ROUTING TYPES DEMO
    {
        let mut f = GraphFixture::new(
            "Demo: Edge Routing Styles",
            "Showcases Straight, Bezier, Taxi (Manhattan), and Unbundled Bezier curves.",
        );

        let s1 = f
            .state
            .add_node(Vec2::new(-150.0, -100.0), Size2::new(40.0, 30.0));
        let t1 = f
            .state
            .add_node(Vec2::new(150.0, -100.0), Size2::new(40.0, 30.0));
        f.node_labels.insert(s1, "Straight Src".to_string());
        f.node_labels.insert(t1, "Straight Tgt".to_string());
        let e1 = f.state.add_edge(s1, t1, EdgeData::default());
        f.edge_labels.insert(0, "Straight".to_string());

        let s2 = f
            .state
            .add_node(Vec2::new(-150.0, 0.0), Size2::new(40.0, 30.0));
        let t2 = f
            .state
            .add_node(Vec2::new(150.0, 0.0), Size2::new(40.0, 30.0));
        f.node_labels.insert(s2, "Taxi Src".to_string());
        f.node_labels.insert(t2, "Taxi Tgt".to_string());
        let e2 = f.state.add_edge(s2, t2, EdgeData::default());
        f.edge_labels.insert(1, "Taxi Grid".to_string());

        let s3 = f
            .state
            .add_node(Vec2::new(-150.0, 100.0), Size2::new(40.0, 30.0));
        let t3 = f
            .state
            .add_node(Vec2::new(150.0, 100.0), Size2::new(40.0, 30.0));
        f.node_labels.insert(s3, "Bezier Src".to_string());
        f.node_labels.insert(t3, "Bezier Tgt".to_string());
        let e3 = f.state.add_edge(s3, t3, EdgeData::default());
        f.edge_labels.insert(2, "Curved Bezier".to_string());

        let _ = (e1, e2, e3);
        fixtures.push(f);
    }

    // 5. WINE & CHEESE PAIRING DEMO (Bipartite)
    {
        let mut f = GraphFixture::new(
            "Demo: Wine & Cheese Pairing Map",
            "Bipartite relationship graph mapping wines to complementary cheeses.",
        );
        f.is_directed = false;

        let wines = vec!["Chardonnay", "Pinot Noir", "Cabernet Sauvignon", "Riesling"];
        let cheeses = vec!["Brie", "Aged Cheddar", "Gouda", "Blue Cheese", "Camembert"];

        let mut wine_ids = Vec::new();
        let mut cheese_ids = Vec::new();

        for (idx, name) in wines.iter().enumerate() {
            let pos = Vec2::new(-120.0, (idx as f32 - 1.5) * 60.0);
            let id = f.state.add_node(pos, Size2::new(60.0, 30.0));
            f.node_labels.insert(id, name.to_string());
            wine_ids.push(id);
        }

        for (idx, name) in cheeses.iter().enumerate() {
            let pos = Vec2::new(120.0, (idx as f32 - 2.0) * 50.0);
            let id = f.state.add_node(pos, Size2::new(60.0, 30.0));
            f.node_labels.insert(id, name.to_string());
            cheese_ids.push(id);
        }

        let pairings = vec![
            (0, 0),
            (0, 4), // Chardonnay -> Brie, Camembert
            (1, 0),
            (1, 2), // Pinot Noir -> Brie, Gouda
            (2, 1),
            (2, 3), // Cabernet -> Cheddar, Blue Cheese
            (3, 2),
            (3, 4), // Riesling -> Gouda, Camembert
        ];

        for (w_idx, c_idx) in pairings {
            f.state
                .add_edge(wine_ids[w_idx], cheese_ids[c_idx], EdgeData::default());
        }

        fixtures.push(f);
    }

    // 6. TOKYO RAILWAYS TOPOLOGY DEMO
    {
        let mut f = GraphFixture::new(
            "Demo: Tokyo Railways Network",
            "Topology loop representing Yamanote and Chuo railway lines.",
        );
        f.is_directed = false;

        let yamanote_stations = vec![
            ("Tokyo", Vec2::new(120.0, 0.0)),
            ("Akihabara", Vec2::new(100.0, -80.0)),
            ("Ueno", Vec2::new(70.0, -130.0)),
            ("Ikebukuro", Vec2::new(-70.0, -130.0)),
            ("Shinjuku", Vec2::new(-120.0, 0.0)),
            ("Shibuya", Vec2::new(-100.0, 80.0)),
            ("Shinagawa", Vec2::new(0.0, 140.0)),
        ];

        let mut station_ids = Vec::new();
        for (name, pos) in yamanote_stations {
            let id = f.state.add_node(pos, Size2::new(45.0, 30.0));
            f.node_labels.insert(id, name.to_string());
            station_ids.push(id);
        }

        // Loop ring
        let n = station_ids.len();
        for i in 0..n {
            f.state.add_edge(
                station_ids[i],
                station_ids[(i + 1) % n],
                EdgeData::default(),
            );
        }

        // Chuo Line shortcut (Tokyo <-> Shinjuku)
        f.state
            .add_edge(station_ids[0], station_ids[4], EdgeData::default());

        fixtures.push(f);
    }

    // 7. GENE NETWORK DEMO (fCoSE)
    {
        let mut f = GraphFixture::new(
            "Demo: Gene Expression Network (fCoSE)",
            "Biological signaling pathway inside nested cellular compartments.",
        );

        let cell = f
            .state
            .add_node(Vec2::new(0.0, 0.0), Size2::new(300.0, 220.0));
        let nucleus = f
            .state
            .add_node(Vec2::new(-40.0, 0.0), Size2::new(140.0, 120.0));

        f.node_labels.insert(cell, "Cell Membrane".to_string());
        f.node_labels.insert(nucleus, "Nucleus".to_string());
        f.state.reparent_node(nucleus, Some(cell));

        let g1 = f
            .state
            .add_node(Vec2::new(-70.0, -20.0), Size2::new(30.0, 25.0));
        let g2 = f
            .state
            .add_node(Vec2::new(-10.0, 20.0), Size2::new(30.0, 25.0));
        f.node_labels.insert(g1, "TP53".to_string());
        f.node_labels.insert(g2, "BRCA1".to_string());

        f.state.reparent_node(g1, Some(nucleus));
        f.state.reparent_node(g2, Some(nucleus));

        let r1 = f
            .state
            .add_node(Vec2::new(80.0, -50.0), Size2::new(30.0, 25.0));
        let r2 = f
            .state
            .add_node(Vec2::new(80.0, 40.0), Size2::new(30.0, 25.0));
        f.node_labels.insert(r1, "EGFR".to_string());
        f.node_labels.insert(r2, "MAPK1".to_string());

        f.state.reparent_node(r1, Some(cell));
        f.state.reparent_node(r2, Some(cell));

        f.state.add_edge(r1, r2, EdgeData::default());
        f.state.add_edge(r2, g1, EdgeData::default());
        f.state.add_edge(g1, g2, EdgeData::default());

        fixtures.push(f);
    }

    // 8. HIGH-ELEMENT PERFORMANCE DEMO (1000 nodes, 1500 edges)
    {
        let mut f = GraphFixture::new(
            "Demo: Performance Stress Test (1000 Elements)",
            "Large-scale mesh network to benchmark physics and rendering performance.",
        );
        f.is_directed = false;

        let count = 1000;
        let mut nodes = Vec::with_capacity(count);
        let side = (count as f32).sqrt().ceil() as usize;

        for i in 0..count {
            let r = i / side;
            let c = i % side;
            let pos = Vec2::new(
                (c as f32 - side as f32 / 2.0) * 40.0,
                (r as f32 - side as f32 / 2.0) * 40.0,
            );
            let id = f.state.add_node(pos, Size2::new(20.0, 20.0));
            f.node_labels.insert(id, format!("N{}", i));
            nodes.push(id);
        }

        for i in 0..count {
            let r = i / side;
            let c = i % side;
            if c + 1 < side && i + 1 < count {
                f.state
                    .add_edge(nodes[i], nodes[i + 1], EdgeData::default());
            }
            if r + 1 < side && i + side < count {
                f.state
                    .add_edge(nodes[i], nodes[i + side], EdgeData::default());
            }
        }

        fixtures.push(f);
    }
}
