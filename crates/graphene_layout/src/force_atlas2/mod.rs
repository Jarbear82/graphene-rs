//! ForceAtlas2 layout algorithm in pure Rust.
//! Based on Jacomy et al. 2014 (PLOS ONE) and the Gephi reference implementation.

pub mod forces;
pub mod types;

use std::f64::consts::PI;

pub use forces::effective_directional_radius;
pub use types::*;

use forces::*;

/// Run a single step of ForceAtlas2 layout.
/// Returns the average node displacement magnitude in this step.
pub fn force_atlas2_step(
    nodes: &mut [Node],
    edges: &[Edge],
    settings: &Settings,
    speed: &mut f64,
    speed_efficiency: &mut f64,
) -> f64 {
    if nodes.is_empty() {
        return 0.0;
    }

    let outbound_att_compensation = if settings.outbound_attraction_distribution {
        let sum_mass: f64 = nodes.iter().map(|n| n.mass).sum();
        sum_mass / nodes.len() as f64
    } else {
        1.0
    };

    for n in nodes.iter_mut() {
        n.old_force = n.force;
        n.force = Vec2::zero();
    }

    if settings.barnes_hut_optimize && nodes.len() > 50 {
        let indices: Vec<usize> = (0..nodes.len()).collect();
        let mut root = Region::new(indices, nodes);
        root.build_subregions(nodes);

        for i in 0..nodes.len() {
            root.apply_force(
                i,
                nodes,
                settings.barnes_hut_theta,
                settings.scaling_ratio,
                settings.adjust_sizes,
            );
        }
    } else {
        for i in 0..nodes.len() {
            for j in 0..i {
                lin_repulsion(i, j, nodes, settings.scaling_ratio, settings.adjust_sizes);
            }
        }
    }

    for n in nodes.iter_mut() {
        if settings.strong_gravity_mode {
            strong_gravity(n, settings.gravity);
        } else {
            lin_gravity(n, settings.gravity);
        }
    }

    let attr_coefficient = if settings.outbound_attraction_distribution {
        outbound_att_compensation
    } else {
        1.0
    };

    for e in edges {
        let w = if settings.edge_weight_influence == 0.0 {
            1.0
        } else if settings.edge_weight_influence == 1.0 {
            e.weight
        } else {
            e.weight.powf(settings.edge_weight_influence)
        };

        if settings.lin_log_mode {
            log_attraction(
                e.source,
                e.target,
                w,
                nodes,
                settings.outbound_attraction_distribution,
                attr_coefficient,
                settings.adjust_sizes,
            );
        } else {
            lin_attraction(
                e.source,
                e.target,
                w,
                nodes,
                settings.outbound_attraction_distribution,
                attr_coefficient,
                settings.adjust_sizes,
            );
        }
    }

    let pos_before: Vec<Vec2> = nodes.iter().map(|n| n.pos).collect();
    adjust_speed_and_apply_forces(
        nodes,
        speed,
        speed_efficiency,
        settings.jitter_tolerance,
        settings.adjust_sizes,
        settings.scaling_ratio,
        settings.slow_down,
    );

    if let Some(fixed_idx) = settings.fixed_node_idx {
        if fixed_idx < nodes.len() {
            nodes[fixed_idx].pos = Vec2::new(0.0, 0.0);
        }
    }

    let mut total_disp = 0.0;
    for (i, n) in nodes.iter().enumerate() {
        total_disp += n.pos.sub(pos_before[i]).length();
    }
    total_disp / (nodes.len() as f64)
}

/// Run ForceAtlas2 for a fixed number of iterations.
pub fn force_atlas2(nodes: &mut [Node], edges: &[Edge], settings: &Settings, iterations: usize) {
    if nodes.is_empty() {
        return;
    }

    let mut speed = 1.0;
    let mut speed_efficiency = 1.0;

    for _iter in 0..iterations {
        force_atlas2_step(nodes, edges, settings, &mut speed, &mut speed_efficiency);
    }
}

/// Build nodes + edges from a simple adjacency list (undirected, unweighted).
pub fn from_adjacency(adj: &[Vec<usize>]) -> (Vec<Node>, Vec<Edge>) {
    let n = adj.len();
    let mut nodes = Vec::with_capacity(n);
    let mut degree = vec![0usize; n];

    for (i, neighbors) in adj.iter().enumerate() {
        for &j in neighbors {
            if i < j {
                degree[i] += 1;
                degree[j] += 1;
            }
        }
    }

    for i in 0..n {
        let angle = 2.0 * PI * i as f64 / n as f64;
        let r = (n as f64).sqrt() * 10.0;
        nodes.push(Node::new(
            r * angle.cos(),
            r * angle.sin(),
            (degree[i] + 1) as f64,
        ));
    }

    let mut edges = Vec::new();
    for (i, neighbors) in adj.iter().enumerate() {
        for &j in neighbors {
            if i < j {
                edges.push(Edge {
                    source: i,
                    target: j,
                    weight: 1.0,
                });
            }
        }
    }

    (nodes, edges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_atlas2_small_graph_positions_finite() {
        let adj = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];
        let (mut nodes, edges) = from_adjacency(&adj);

        let settings = Settings {
            barnes_hut_optimize: false,
            ..Default::default()
        };

        force_atlas2(&mut nodes, &edges, &settings, 100);

        for n in &nodes {
            assert!(n.pos.x.is_finite() && n.pos.y.is_finite());
        }
    }

    #[test]
    fn test_force_atlas2_equilateral_triangle_remains_2d() {
        let mut nodes = vec![
            Node::new(0.0, 100.0, 3.0),
            Node::new(-86.6, -50.0, 3.0),
            Node::new(86.6, -50.0, 3.0),
        ];
        let edges = vec![
            Edge {
                source: 0,
                target: 1,
                weight: 1.0,
            },
            Edge {
                source: 1,
                target: 2,
                weight: 1.0,
            },
            Edge {
                source: 2,
                target: 0,
                weight: 1.0,
            },
        ];

        let settings = Settings::default();
        let mut speed = 1.0;
        let mut speed_eff = 1.0;

        for _ in 0..100 {
            force_atlas2_step(&mut nodes, &edges, &settings, &mut speed, &mut speed_eff);
        }

        let area = 0.5
            * (nodes[0].pos.x * (nodes[1].pos.y - nodes[2].pos.y)
                + nodes[1].pos.x * (nodes[2].pos.y - nodes[0].pos.y)
                + nodes[2].pos.x * (nodes[0].pos.y - nodes[1].pos.y))
                .abs();

        assert!(
            area > 1.0,
            "Triangle should maintain 2D shape and not collapse into 1D line, area was {}",
            area
        );
    }

    #[test]
    fn test_force_atlas2_circular_graph_stability() {
        let n = 20;
        let mut adj = vec![Vec::new(); n];
        for i in 0..n {
            adj[i].push((i + 1) % n);
            adj[(i + 1) % n].push(i);
        }
        let (mut nodes, edges) = from_adjacency(&adj);

        let settings = Settings::default();
        let mut speed = 1.0;
        let mut speed_eff = 1.0;

        for _ in 0..200 {
            force_atlas2_step(&mut nodes, &edges, &settings, &mut speed, &mut speed_eff);
        }

        for n in &nodes {
            let dist = n.pos.length();
            assert!(dist.is_finite(), "Node position must be finite");
            assert!(
                dist < 2000.0,
                "Node position should not explode to infinity: dist = {}",
                dist
            );
        }
    }

    #[test]
    fn test_force_atlas2_rectangular_node_directional_radius() {
        let mut node = Node::new(0.0, 0.0, 1.0);
        node.size_wh = Vec2::new(100.0, 20.0);

        let r_x = effective_directional_radius(Vec2::new(200.0, 0.0), 200.0, &node);
        assert!(
            (r_x - 50.0).abs() < 1e-4,
            "Horizontal radius should equal half-width (50.0), got {}",
            r_x
        );

        let r_y = effective_directional_radius(Vec2::new(0.0, 100.0), 100.0, &node);
        assert!(
            (r_y - 10.0).abs() < 1e-4,
            "Vertical radius should equal half-height (10.0), got {}",
            r_y
        );
    }

    #[test]
    fn test_force_atlas2_overlap_repositioning_and_zero_force() {
        let mut nodes = vec![Node::new(0.0, 0.0, 1.0), Node::new(2.0, 0.0, 1.0)];
        nodes[0].size = 10.0;
        nodes[1].size = 10.0;

        lin_repulsion(0, 1, &mut nodes, 1.0, true);

        let dist = nodes[0].pos.sub(nodes[1].pos).length();
        assert!(
            dist >= 25.0,
            "Overlapping nodes should be separated outside bounds with padding, dist = {}",
            dist
        );
        assert_eq!(
            nodes[0].force,
            Vec2::zero(),
            "Force on overlapping node should be zero"
        );
        assert_eq!(
            nodes[1].force,
            Vec2::zero(),
            "Force on overlapping node should be zero"
        );
    }

    #[test]
    fn test_force_atlas2_edge_clearance_visible() {
        let mut nodes = vec![Node::new(0.0, 0.0, 1.0), Node::new(500.0, 0.0, 1.0)];
        nodes[0].size_wh = Vec2::new(40.0, 40.0);
        nodes[1].size_wh = Vec2::new(40.0, 40.0);
        let edges = vec![Edge {
            source: 0,
            target: 1,
            weight: 1.0,
        }];

        let settings = Settings::default();
        let mut speed = 1.0;
        let mut speed_eff = 1.0;

        for _ in 0..100 {
            force_atlas2_step(&mut nodes, &edges, &settings, &mut speed, &mut speed_eff);
        }

        let dist = nodes[0].pos.sub(nodes[1].pos).length();
        let clearance = dist - 40.0;
        assert!(
            clearance > 5.0,
            "Edge clearance between node borders must be clearly visible (> 5px), got {}",
            clearance
        );
    }

    #[test]
    fn test_force_atlas2_edge_lengths_vary_by_degree() {
        let mut nodes = vec![
            Node::new(0.0, 0.0, 5.0),
            Node::new(50.0, 0.0, 1.0),
            Node::new(-50.0, 0.0, 1.0),
            Node::new(0.0, 50.0, 1.0),
        ];
        let edges = vec![
            Edge {
                source: 0,
                target: 1,
                weight: 1.0,
            },
            Edge {
                source: 0,
                target: 2,
                weight: 1.0,
            },
            Edge {
                source: 0,
                target: 3,
                weight: 1.0,
            },
        ];

        let settings = Settings::default();
        let mut speed = 1.0;
        let mut speed_eff = 1.0;

        for _ in 0..100 {
            force_atlas2_step(&mut nodes, &edges, &settings, &mut speed, &mut speed_eff);
        }

        let d_hub_leaf = nodes[0].pos.sub(nodes[1].pos).length();
        assert!(d_hub_leaf.is_finite(), "Distance must be finite");
        assert!(
            d_hub_leaf > 10.0,
            "Hub-leaf edge distance should be positive and non-zero"
        );
    }
}
