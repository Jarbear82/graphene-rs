use crate::app::DemoApp;
use crate::theme::{Theme, LAYOUT_NAMES};
use gpui::{
    px, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::Input;

impl DemoApp {
    pub fn render_sidebar_left(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .flex_col()
            .h_full()
            .bg(theme.panel_bg)
            .border_r(px(1.0))
            .border_color(theme.border)
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("1. SELECT GRAPH FIXTURE"),
                    )
                    .child(
                        gpui::div()
                            .id("preset-scroll-container")
                            .flex()
                            .flex_col()
                            .h(px(150.0))
                            .overflow_y_scroll()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .bg(theme.bg)
                            .rounded_md()
                            .children(self.fixtures.iter().enumerate().map(|(idx, f)| {
                                let is_selected = self.selected_fixture_idx == idx;
                                gpui::div()
                                    .id(SharedString::from(format!("preset-{}", idx)))
                                    .p_2()
                                    .border_b(px(1.0))
                                    .border_color(theme.border)
                                    .bg(if is_selected {
                                        theme.accent
                                    } else {
                                        gpui::rgba(0)
                                    })
                                    .text_color(if is_selected {
                                        theme.panel_bg
                                    } else {
                                        theme.text
                                    })
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.load_preset(idx, window, cx);
                                    }))
                                    .child(f.name.clone())
                            })),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("2. LAYOUT ENGINE"),
                    )
                    .child(
                        gpui::div()
                            .id("layout-scroll-container")
                            .flex()
                            .flex_col()
                            .h(px(150.0))
                            .overflow_y_scroll()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .bg(theme.bg)
                            .rounded_md()
                            .children(LAYOUT_NAMES.iter().map(|&name| {
                                let is_selected = self.selected_layout == name;
                                gpui::div()
                                    .id(SharedString::from(format!("layout-{}", name)))
                                    .p_2()
                                    .border_b(px(1.0))
                                    .border_color(theme.border)
                                    .bg(if is_selected {
                                        theme.accent
                                    } else {
                                        gpui::rgba(0)
                                    })
                                    .text_color(if is_selected {
                                        theme.panel_bg
                                    } else {
                                        theme.text
                                    })
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, _| {
                                        this.selected_layout = name.to_string();
                                    }))
                                    .child(name)
                            })),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("3. LIVE PHYSICS ENGINE"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text)
                                    .text_size(px(11.0))
                                    .child(if self.physics_enabled {
                                        format!(
                                            "Status: Active (Temp: {:.2})",
                                            self.physics_temperature
                                        )
                                    } else {
                                        "Status: Disabled".to_string()
                                    }),
                            )
                            .child(
                                Button::new("toggle-physics-btn")
                                    .label(if self.physics_enabled {
                                        "DISABLE"
                                    } else {
                                        "ENABLE"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.physics_enabled = !this.physics_enabled;
                                        if this.physics_enabled {
                                            this.physics_temperature = 10.0;
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text)
                                    .text_size(px(11.0))
                                    .child(if self.use_barnes_hut {
                                        "Barnes-Hut: ON"
                                    } else {
                                        "Barnes-Hut: OFF"
                                    }),
                            )
                            .child(
                                Button::new("toggle-barnes-hut-btn")
                                    .label(if self.use_barnes_hut {
                                        "CLASSIC"
                                    } else {
                                        "BARNES-HUT"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.use_barnes_hut = !this.use_barnes_hut;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("4. FONT & TEXT CONFIG"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("Max Label Length"),
                            )
                            .child(
                                gpui::div()
                                    .w(px(50.0))
                                    .child(Input::new(&self.input_max_len)),
                            ),
                    ),
            )
            .child(
                Button::new("run-layout-btn")
                    .primary()
                    .label("RUN LAYOUT")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.trigger_layout(cx);
                    })),
            )
            .child(
                gpui::div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("fit-view-btn")
                            .label("FIT VIEW")
                            .on_click(cx.listener(|this, _, _, _| {
                                this.fit_view();
                            })),
                    )
                    .child(
                        Button::new("reset-zoom-btn")
                            .label("RESET")
                            .on_click(cx.listener(|this, _, _, _| {
                                this.viewport.offset = graphene_core::Vec2::default();
                                this.viewport.zoom = 1.0;
                            })),
                    ),
            )
    }
}
