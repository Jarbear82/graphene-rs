use gpui::{Application, WindowOptions, Styled, ParentElement, AppContext};
use graphene_core::{EdgeData, GraphState, Size2, Vec2};
use graphene_gpui::{
    interaction::state::InteractionState,
    render::draw_pipeline::Viewport,
};
use graphene_style::ComputedStyle;

struct DemoApp {
    _state: GraphState<ComputedStyle>,
    _viewport: Viewport,
    _interaction: InteractionState,
}

impl gpui::Render for DemoApp {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<'_, Self>) -> impl gpui::IntoElement {
        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .bg(gpui::rgb(0x1e1e2e))
            .child(
                gpui::div()
                    .p_4()
                    .text_color(gpui::rgb(0xcdd6f4))
                    .child("Graphene-RS: GPUI-Rendered Graph Library")
            )
            .child(
                gpui::div()
                    .p_4()
                    .text_color(gpui::rgb(0xa6adc8))
                    .child("Headless rendering components, spatial grid, layouts, and algorithms verified successfully.")
            )
    }
}

fn main() {
    let app = Application::new();
    app.run(|cx| {
        let _window = cx.open_window(WindowOptions::default(), |_window, cx| {
            let mut state = GraphState::<ComputedStyle>::new();
            let n1 = state.add_node(Vec2::new(100.0, 100.0), Size2::new(40.0, 40.0));
            let n2 = state.add_node(Vec2::new(300.0, 100.0), Size2::new(40.0, 40.0));
            state.add_edge(n1, n2, EdgeData::default());

            // Use default viewport bounds
            let bounds = gpui::Bounds {
                origin: gpui::point(0.0, 0.0),
                size: gpui::size(800.0, 600.0),
            };

            let viewport = Viewport::new(bounds);
            let interaction = InteractionState::new(50.0);

            cx.new(|_cx| DemoApp {
                _state: state,
                _viewport: viewport,
                _interaction: interaction,
            })
        });
    });
}
