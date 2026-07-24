use crate::app::DemoApp;
use crate::theme::Theme;
use gpui::{px, Context, IntoElement, ParentElement, SharedString, Styled};
use gpui_component::button::{Button, ButtonVariants};

impl DemoApp {
    pub fn render_analysis_panel(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let report_opt = self.analysis_report.as_ref();

        gpui::div()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .bg(theme.bg)
            .rounded_md()
            .border(px(1.0))
            .border_color(theme.border)
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("GRAPH ANALYSIS & METRICS"),
                    )
                    .child(
                        Button::new("re-analyze-btn")
                            .label("ANALYZE")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.run_analysis();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("toggle-directed-btn")
                            .label(if self.is_directed {
                                "DIRECTED: ON"
                            } else {
                                "DIRECTED: OFF"
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.is_directed = !this.is_directed;
                                this.run_analysis();
                                cx.notify();
                            })),
                    ),
            )
            .child(if let Some(report) = report_opt {
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child(format!(
                                        "Nodes: {} | Edges: {} | Density: {:.2}%",
                                        report.node_count,
                                        report.edge_count,
                                        report.density * 100.0
                                    )),
                            )
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child(format!(
                                        "Avg Deg: {:.2} | Reciprocity: {:.2}",
                                        report.average_degree, report.reciprocity
                                    )),
                            )
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child(format!(
                                        "Components: WCC={} / SCC={}",
                                        report.connected_components_count,
                                        report.strongly_connected_components_count
                                    )),
                            )
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child(format!(
                                        "Articulation Points: {} | Bridges: {}",
                                        report.articulation_point_count, report.bridge_count
                                    )),
                            ),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(11.0))
                            .child("HEATMAP OVERLAY"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                gpui::div().flex().gap_1().children(vec!["PageRank", "Betweenness"].into_iter().map(|m| {
                                    let is_active = self.active_heatmap.as_deref() == Some(m);
                                    let label = format!("{}", m);
                                    Button::new(SharedString::from(format!("heatmap-btn-{}", m)))
                                        .primary()
                                        .label(label)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if this.active_heatmap.as_deref() == Some(m) {
                                                this.active_heatmap = None;
                                            } else {
                                                this.active_heatmap = Some(m.to_string());
                                            }
                                            cx.notify();
                                        }))
                                })),
                            )
                            .child(
                                gpui::div().flex().gap_1().children(vec!["Degree", "Closeness"].into_iter().map(|m| {
                                    Button::new(SharedString::from(format!("heatmap-btn-{}", m)))
                                        .primary()
                                        .label(format!("{}", m))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            if this.active_heatmap.as_deref() == Some(m) {
                                                this.active_heatmap = None;
                                            } else {
                                                this.active_heatmap = Some(m.to_string());
                                            }
                                            cx.notify();
                                        }))
                                })),
                            )
                            .child(
                                Button::new("clear-heatmap-btn")
                                    .danger()
                                    .label("CLEAR HEATMAP")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.active_heatmap = None;
                                        cx.notify();
                                    })),
                            ),
                    )
            } else {
                gpui::div()
                    .text_color(theme.text_dim)
                    .text_size(px(11.0))
                    .child("Click ANALYZE to run graph analysis.")
            })
    }
}
