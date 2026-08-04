use gpui::{bounds, point, size, Bounds};
use graphene_core::{GraphState, Size2, Vec2};
use graphene_gpui::render::draw_pipeline::Viewport;
use graphene_gpui::render::graph_canvas::{GraphNodeElement, CanvasConfig};
use graphene_style::{ComputedStyle, NodeShape};
use std::time::Instant;

const SCALES: &[usize] = &[10, 100, 1000, 10000];

fn build_test_graph(node_count: usize) -> GraphState<ComputedStyle> {
    let mut state = GraphState::<ComputedStyle>::new();
    for i in 0..node_count {
        let angle = (i as f32) * 0.1;
        let r = 50.0 + (i as f32).sqrt() * 10.0;
        let pos = Vec2::new(r * angle.cos(), r * angle.sin());
        state.add_node(pos, Size2::new(40.0, 40.0));
    }
    state
}

#[test]
fn test_headless_gpui_viewport_and_culling_performance() {
    println!("\n=== GPUI Headless Viewport Culling & Element Construction Benchmark ===");
    let screen_bounds: Bounds<f32> = Bounds {
        origin: point(0.0, 0.0),
        size: size(1920.0, 1080.0),
    };

    for &n in SCALES {
        let state = build_test_graph(n);
        let mut viewport = Viewport::new(screen_bounds);
        viewport.fit_to_graph(&state);

        // 1. Viewport Culling Benchmark
        let start_cull = Instant::now();
        let mut visible_count = 0;
        for i in 0..n {
            let pos = *state.positions.get(i);
            let size = *state.sizes.get(i);
            if viewport.is_visible(pos, size) {
                visible_count += 1;
            }
        }
        let cull_duration = start_cull.elapsed().as_secs_f64() * 1000.0;

        // 2. Element Construction Benchmark (Immediate mode layout allocation)
        let start_elem = Instant::now();
        let mut elements = Vec::with_capacity(visible_count);
        for i in 0..n {
            let pos = *state.positions.get(i);
            let size = *state.sizes.get(i);
            if viewport.is_visible(pos, size) {
                let screen_p = viewport.model_to_screen(pos);
                let elem = GraphNodeElement {
                    id: gpui::SharedString::from(format!("node_{}", i)),
                    screen_x: screen_p.x,
                    screen_y: screen_p.y,
                    width: size.w * viewport.zoom,
                    height: size.h * viewport.zoom,
                    border_width: 2.0,
                    border_color: gpui::rgba(0x000000ff),
                    fill_color: gpui::rgba(0x3b82f6ff),
                    shape: NodeShape::Rectangle,
                    text_color: gpui::rgba(0xffffffff),
                    font_size: 12.0,
                    label: format!("N{}", i),
                };
                elements.push(elem);
            }
        }
        let elem_duration = start_elem.elapsed().as_secs_f64() * 1000.0;

        println!(
            "Scale N = {:5} | Visible: {:5} | Cull Time: {:7.3} ms | Element Build: {:7.3} ms",
            n, visible_count, cull_duration, elem_duration
        );

        assert!(visible_count <= n, "Visible count cannot exceed total nodes");
    }
}
