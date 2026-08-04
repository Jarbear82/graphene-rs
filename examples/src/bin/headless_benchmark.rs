use graphene_core::{EdgeData, GraphState, NodeId, Size2, Vec2};
use graphene_layout::{
    BipartiteLayout, CircleLayout, CircularAdvancedLayout, CollisionForceDirectedLayout,
    ConcentricHubLayout, CoseLayout, ForceDirectedLayout, FA2Settings,
    GridSortedLayout, KamadaKawaiLayout, Layout, MdsLayout, ReingoldTilfordLayout,
    SugiyamaLayout, TutteBarycentricLayout, WeightedForceDirectedLayout, force_atlas2_step, FA2Node, FA2Edge
};
use graphene_style::ComputedStyle;
use std::collections::HashMap;
use std::time::Instant;

/// Scale factors for performance benchmark
const SCALE_FACTORS: &[usize] = &[10, 100, 1000, 10000];

/// Maximum node limit for O(N^3) matrix algorithms (Kamada-Kawai, MDS, Tutte)
const O_N3_LIMIT: usize = 1000;

/// Maximum node limit for un-optimized O(N^2) algorithms (ForceDirected, Collision, Sugiyama)
const O_N2_LIMIT: usize = 1000;

fn create_synthetic_graph(node_count: usize) -> GraphState<ComputedStyle> {
    let mut state = GraphState::<ComputedStyle>::new();
    let mut nodes = Vec::with_capacity(node_count);

    // 1. Add nodes
    for i in 0..node_count {
        let angle = (i as f32) * 0.1;
        let r = 50.0 + (i as f32).sqrt() * 10.0;
        let pos = Vec2::new(r * angle.cos(), r * angle.sin());
        let size = Size2::new(40.0, 40.0);
        let id = state.add_node(pos, size);
        nodes.push(id);
    }

    // 2. Add scale-free / hub-connected topology
    for i in 0..node_count {
        // Connect to primary hubs
        let hub1 = 0;
        let hub2 = i / 10;
        state.add_edge(nodes[i], nodes[hub1], EdgeData::default());
        if hub2 != hub1 && hub2 < node_count {
            state.add_edge(nodes[i], nodes[hub2], EdgeData::default());
        }
        // Connect sequential neighbors
        if i + 1 < node_count {
            state.add_edge(nodes[i], nodes[i + 1], EdgeData::default());
        }
    }

    state
}

struct BenchmarkResult {
    algo_name: String,
    scale: usize,
    duration_ms: f64,
    status: String,
}

fn main() {
    println!("============================================================");
    println!("        GRAPHENE-RS MULTI-SCALE PERFORMANCE BENCHMARK        ");
    println!("============================================================");

    let mut results: Vec<BenchmarkResult> = Vec::new();

    for &n in SCALE_FACTORS {
        println!("\n>>> Benchmarking Scale: N = {} nodes", n);
        let state_template = create_synthetic_graph(n);

        // 1. CircleLayout - O(N)
        {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = CircleLayout {
                radius: 500.0,
                center: Vec2::new(0.0, 0.0),
                animate: false,
            };
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "CircleLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] CircleLayout: {:.3} ms", duration);
        }

        // 2. GridSortedLayout - O(N log N)
        {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = GridSortedLayout::default();
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "GridSortedLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] GridSortedLayout: {:.3} ms", duration);
        }

        // 3. BipartiteLayout - O(N)
        {
            let mut state = state_template.clone();
            let start = Instant::now();
            let node_indices: HashMap<NodeId, usize> = state
                .node_index_to_id
                .iter()
                .enumerate()
                .map(|(idx, &id)| (id, idx))
                .collect();
            let mut layout = BipartiteLayout {
                partition_fn: move |id: NodeId| {
                    if *node_indices.get(&id).unwrap_or(&0) % 2 == 0 { 0 } else { 1 }
                },
                column_spacing: 120.0,
                vertical_spacing: 50.0,
            };
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "BipartiteLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] BipartiteLayout: {:.3} ms", duration);
        }

        // 4. ConcentricHubLayout - O(N log N)
        {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = ConcentricHubLayout::default();
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "ConcentricHubLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] ConcentricHubLayout: {:.3} ms", duration);
        }

        // 5. CircularAdvancedLayout - O(N log N) / O(N * E^2)
        if n <= 1000 {
            let mut state = state_template.clone();
            let start = Instant::now();
            let layout = CircularAdvancedLayout::default();
            layout.apply(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "CircularAdvancedLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] CircularAdvancedLayout: {:.3} ms", duration);
        } else {
            results.push(BenchmarkResult {
                algo_name: "CircularAdvancedLayout".to_string(),
                scale: n,
                duration_ms: 0.0,
                status: "SKIPPED (O(N^2) Limit)".to_string(),
            });
            println!("  [SKIPPED] CircularAdvancedLayout (Exceeds O(N^2) Limit)");
        }

        // 6. ReingoldTilfordLayout - O(N)
        {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = ReingoldTilfordLayout::default();
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "ReingoldTilfordLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] ReingoldTilfordLayout: {:.3} ms", duration);
        }

        // 7. ForceAtlas2 (50 Ticks) - O(N log N) via Barnes-Hut
        {
            let mut state = state_template.clone();
            let start = Instant::now();
            let n_nodes = state.node_index_to_id.len();
            let mut fa2_nodes: Vec<FA2Node> = (0..n_nodes)
                .map(|i| {
                    let p = *state.positions.get(i);
                    FA2Node::new(p.x as f64, p.y as f64, 1.0)
                })
                .collect();
            let fa2_edges: Vec<FA2Edge> = (0..state.edges.len())
                .map(|i| {
                    let src = *state.edge_sources.get(i);
                    let tgt = *state.edge_targets.get(i);
                    let u = state.node_keys.get(src).copied().unwrap_or(0);
                    let v = state.node_keys.get(tgt).copied().unwrap_or(0);
                    FA2Edge { source: u, target: v, weight: 1.0 }
                })
                .collect();

            let mut settings = FA2Settings::infer_settings(n_nodes, state.edges.len(), 20.0);
            let mut speed = 1.0;
            let mut speed_eff = 1.0;

            for _step in 0..50 {
                force_atlas2_step(&mut fa2_nodes, &fa2_edges, &settings, &mut speed, &mut speed_eff);
            }

            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "ForceAtlas2 (50 Ticks)".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] ForceAtlas2 (50 Ticks): {:.3} ms", duration);
        }

        // 8. CoseLayout - O(N log N) / O(N^2)
        if n <= 10000 {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = CoseLayout::default();
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "CoseLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] CoseLayout: {:.3} ms", duration);
        }

        // 9. SugiyamaLayout - O(V*E + V^2)
        if n <= O_N2_LIMIT {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = SugiyamaLayout::default();
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "SugiyamaLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] SugiyamaLayout: {:.3} ms", duration);
        } else {
            results.push(BenchmarkResult {
                algo_name: "SugiyamaLayout".to_string(),
                scale: n,
                duration_ms: 0.0,
                status: "SKIPPED (O(N^2) Limit)".to_string(),
            });
            println!("  [SKIPPED] SugiyamaLayout (Exceeds O(N^2) Limit)");
        }

        // 10. ForceDirectedLayout - O(N^2)
        if n <= O_N2_LIMIT {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = ForceDirectedLayout::default();
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "ForceDirectedLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] ForceDirectedLayout: {:.3} ms", duration);
        } else {
            results.push(BenchmarkResult {
                algo_name: "ForceDirectedLayout".to_string(),
                scale: n,
                duration_ms: 0.0,
                status: "SKIPPED (O(N^2) Limit)".to_string(),
            });
            println!("  [SKIPPED] ForceDirectedLayout (Exceeds O(N^2) Limit)");
        }

        // 11. CollisionForceDirectedLayout - O(N^2)
        if n <= O_N2_LIMIT {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = CollisionForceDirectedLayout::default();
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "CollisionForceDirectedLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] CollisionForceDirectedLayout: {:.3} ms", duration);
        } else {
            results.push(BenchmarkResult {
                algo_name: "CollisionForceDirectedLayout".to_string(),
                scale: n,
                duration_ms: 0.0,
                status: "SKIPPED (O(N^2) Limit)".to_string(),
            });
            println!("  [SKIPPED] CollisionForceDirectedLayout (Exceeds O(N^2) Limit)");
        }

        // 12. KamadaKawaiLayout - O(N^3)
        if n <= O_N3_LIMIT {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = KamadaKawaiLayout::default();
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "KamadaKawaiLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] KamadaKawaiLayout: {:.3} ms", duration);
        } else {
            results.push(BenchmarkResult {
                algo_name: "KamadaKawaiLayout".to_string(),
                scale: n,
                duration_ms: 0.0,
                status: "SKIPPED (O(N^3) Limit)".to_string(),
            });
            println!("  [SKIPPED] KamadaKawaiLayout (Exceeds O(N^3) Limit)");
        }

        // 13. MdsLayout - O(N^3)
        if n <= O_N3_LIMIT {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = MdsLayout::default();
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "MdsLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] MdsLayout: {:.3} ms", duration);
        } else {
            results.push(BenchmarkResult {
                algo_name: "MdsLayout".to_string(),
                scale: n,
                duration_ms: 0.0,
                status: "SKIPPED (O(N^3) Limit)".to_string(),
            });
            println!("  [SKIPPED] MdsLayout (Exceeds O(N^3) Limit)");
        }

        // 14. TutteBarycentricLayout - O(N^3)
        if n <= O_N3_LIMIT {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = TutteBarycentricLayout::default();
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "TutteBarycentricLayout".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] TutteBarycentricLayout: {:.3} ms", duration);
        } else {
            results.push(BenchmarkResult {
                algo_name: "TutteBarycentricLayout".to_string(),
                scale: n,
                duration_ms: 0.0,
                status: "SKIPPED (O(N^3) Limit)".to_string(),
            });
            println!("  [SKIPPED] TutteBarycentricLayout (Exceeds O(N^3) Limit)");
        }

        // 15. WeightedForceDirectedLayout - O(N^2)
        if n <= O_N2_LIMIT {
            let mut state = state_template.clone();
            let start = Instant::now();
            let mut layout = WeightedForceDirectedLayout {
                iterations: 50,
                gravity: 1.0,
                k_rep: 30.0,
                k_att: 30.0,
                weight_fn: |_| 1.0,
            };
            layout.compute(&mut state);
            let duration = start.elapsed().as_secs_f64() * 1000.0;
            results.push(BenchmarkResult {
                algo_name: "WeightedForceDirected".to_string(),
                scale: n,
                duration_ms: duration,
                status: "OK".to_string(),
            });
            println!("  [OK] WeightedForceDirected: {:.3} ms", duration);
        } else {
            results.push(BenchmarkResult {
                algo_name: "WeightedForceDirected".to_string(),
                scale: n,
                duration_ms: 0.0,
                status: "SKIPPED (O(N^2) Limit)".to_string(),
            });
            println!("  [SKIPPED] WeightedForceDirected (Exceeds O(N^2) Limit)");
        }
    }

    // Output Formatted Markdown Table
    println!("\n============================================================");
    println!("                FINAL BENCHMARK SUMMARY TABLE               ");
    println!("============================================================");
    println!("| Algorithm Name | N = 10 | N = 100 | N = 1,000 | N = 10,000 |");
    println!("|---|---|---|---|---|");

    let unique_algos: Vec<String> = results
        .iter()
        .map(|r| r.algo_name.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for algo in unique_algos {
        let mut line = format!("| **{}** ", algo);
        for &n in SCALE_FACTORS {
            if let Some(r) = results.iter().find(|res| res.algo_name == algo && res.scale == n) {
                if r.status == "OK" {
                    line.push_str(&format!("| {:.2} ms ", r.duration_ms));
                } else {
                    line.push_str("| *SKIPPED* ");
                }
            } else {
                line.push_str("| N/A ");
            }
        }
        line.push('|');
        println!("{}", line);
    }
}
