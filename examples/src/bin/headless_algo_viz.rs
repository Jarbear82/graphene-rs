use graphene_algo::{bfs, dijkstra};
use graphene_core::{EdgeData, GraphState, NodeId, Size2, Vec2};
use graphene_layout::{
    compute_hierarchical_edge_bundling, compute_multigraph_bezier_routing, BipartiteLayout,
    CircleLayout, CollisionForceDirectedLayout, ConcentricHubLayout, CoseLayout,
    ForceDirectedLayout, GridSortedLayout, KamadaKawaiLayout, Layout, MdsLayout,
    ReingoldTilfordLayout, SugiyamaLayout, WeightedForceDirectedLayout,
};
use graphene_style::ComputedStyle;
use std::collections::HashMap;

fn main() {
    println!("=== Graphene Headless Algorithm & Layout Visualizer ===");

    // Initialize state with default ComputedStyle
    let mut state = GraphState::<ComputedStyle>::new();

    // 1. Add nodes (V)
    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    let n2 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(10.0, 10.0));
    let n3 = state.add_node(Vec2::new(20.0, 20.0), Size2::new(10.0, 10.0));
    let n4 = state.add_node(Vec2::new(-30.0, 5.0), Size2::new(10.0, 10.0));

    // 2. Link edges (E)
    let e1 = state.add_edge(n1, n2, EdgeData::default());
    let e2 = state.add_edge(n2, n3, EdgeData::default());
    let e3 = state.add_edge(n3, n4, EdgeData::default());
    let e4 = state.add_edge(n4, n1, EdgeData::default()); // forms a cycle

    println!("Graph created successfully:");
    println!("  Nodes: {}", state.node_index_to_id.len());
    println!("  Edges: {}", state.edges.len());

    // 3. Compute Circle Layout
    println!("\n--- Computing CircleLayout ---");
    let mut circle = CircleLayout {
        radius: 100.0,
        center: Vec2::new(0.0, 0.0),
        animate: false,
    };
    circle.compute(&mut state);
    println!(
        "  Circle: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 4. Compute Force-Directed Layout
    println!("\n--- Computing ForceDirectedLayout (Simulation) ---");
    let mut force = ForceDirectedLayout::default();
    force.compute(&mut state);
    println!(
        "  Force: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 4b. Compute CoSE Layout
    println!("\n--- Computing CoseLayout (Compound Spring Embedder) ---");
    let mut cose = CoseLayout::default();
    cose.compute(&mut state);
    println!(
        "  CoSE: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 5. Compute Kamada-Kawai Layout
    println!("\n--- Computing KamadaKawaiLayout (Energy Minimization) ---");
    let mut kk = KamadaKawaiLayout::default();
    kk.compute(&mut state);
    println!(
        "  Kamada-Kawai: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 6. Compute Sugiyama Layout
    println!("\n--- Computing SugiyamaLayout (Layered Directed) ---");
    let mut sugi = SugiyamaLayout::default();
    sugi.compute(&mut state);
    println!(
        "  Sugiyama: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 7. Compute Reingold-Tilford Layout
    println!("\n--- Computing ReingoldTilfordLayout (Tidy Tree) ---");
    let mut rt = ReingoldTilfordLayout::default();
    rt.compute(&mut state);
    println!(
        "  Reingold-Tilford: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 8. Compute MDS Layout
    println!("\n--- Computing MdsLayout (Multidimensional Scaling) ---");
    let mut mds = MdsLayout::default();
    mds.compute(&mut state);
    println!(
        "  MDS: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 9. Compute Grid Sorted Layout
    println!("\n--- Computing GridSortedLayout (Connectivity sorted grid) ---");
    let mut grid_sorted = GridSortedLayout::default();
    grid_sorted.compute(&mut state);
    println!(
        "  Grid Sorted: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 10. Compute Concentric Hub Layout
    println!("\n--- Computing ConcentricHubLayout ---");
    let mut concentric_hub = ConcentricHubLayout::default();
    concentric_hub.compute(&mut state);
    println!(
        "  Concentric Hub: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 11. Compute Bipartite Column Layout
    println!("\n--- Computing BipartiteLayout ---");
    let node_indices: HashMap<NodeId, usize> = state
        .node_index_to_id
        .iter()
        .enumerate()
        .map(|(idx, &id)| (id, idx))
        .collect();
    let mut bipartite = BipartiteLayout {
        partition_fn: move |id: NodeId| {
            if *node_indices.get(&id).unwrap_or(&0) % 2 == 0 {
                0
            } else {
                1
            }
        },
        column_spacing: 120.0,
        vertical_spacing: 50.0,
    };
    bipartite.compute(&mut state);
    println!(
        "  Bipartite: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 12. Compute Weighted Force-Directed Layout
    println!("\n--- Computing WeightedForceDirectedLayout ---");
    let mut weighted_force = WeightedForceDirectedLayout {
        iterations: 100,
        gravity: 1.0,
        k_rep: 30.0,
        k_att: 30.0,
        weight_fn: |_edge_id| 2.0, // uniform weight 2.0
    };
    weighted_force.compute(&mut state);
    println!(
        "  Weighted Force: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 13. Compute Collision Free Layout
    println!("\n--- Computing CollisionForceDirectedLayout ---");
    let mut collision_force = CollisionForceDirectedLayout::default();
    collision_force.compute(&mut state);
    println!(
        "  Collision Force: Node 1 at ({:.2}, {:.2})",
        state.positions.get(0).x,
        state.positions.get(0).y
    );

    // 14. Compute Multigraph Bezier Routing
    println!("\n--- Computing Multigraph Bezier Routing ---");
    let routes = compute_multigraph_bezier_routing(&state, 20.0);
    println!("  Routing control points count: {}", routes.len());

    // 15. Compute Hierarchical Edge Bundling
    println!("\n--- Computing Hierarchical Edge Bundling ---");
    let bundles = compute_hierarchical_edge_bundling(&state, 0.8);
    println!("  Edge bundles count: {}", bundles.len());

    // BFS/Dijkstra check
    println!("\n--- Running Traversals ---");
    bfs(&state, n1, |id| {
        let _ = id;
    });
    let distances = dijkstra(&state, n1, |_| 10.0);
    println!(
        "  Dijkstra destination nodes calculated: {}",
        distances.len()
    );

    // Unused warnings cleanup
    let _ = (e1, e2, e3, e4);

    println!("\nAll layout algorithms and routing functions executed successfully!");
}
