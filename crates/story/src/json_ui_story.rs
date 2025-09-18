use gpui::*;

use gpui_component::{
    button::Button, h_flex, json_ui::JsonCanvas, v_flex, IconName,
};

use crate::section;

actions!(json_ui_story, [ReloadUI]);

pub struct JsonUIStory {
    focus_handle: gpui::FocusHandle,
    json_canvas: JsonCanvas,
}

impl JsonUIStory {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(cx))
    }

    fn new(cx: &mut Context<Self>) -> Self {

        let mut json_canvas = {
            // Dev mode: load from absolute path using CARGO_MANIFEST_DIR
            let json_path = format!(
                "{}/assets/json_ui_examples/complex.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let mut jc = JsonCanvas::new(&json_path);
            if let Err(e) = jc.load() {
                eprintln!("Error loading JSON UI from file: {}", e);
                eprintln!("Tried path: {}", json_path);
                eprintln!("Falling back to embedded JSON demo...");

                // Fallback to embedded JSON demo
                let fallback_json = r#"{
                    "type": "column",
                    "props": {
                        "padding": 20,
                        "backgroundColor": "black"
                    },
                    "children": [
                        {
                            "type": "h1",
                            "props": {
                                "color": "blue"
                            },
                            "children": ["JSON UI Demo (Fallback)"]
                        },
                        {
                            "type": "text",
                            "props": {
                                "content": "This is a fallback JSON UI demo since the file could not be loaded."
                            }
                        },
                        {
                            "type": "row",
                            "props": {
                                "margin": 10
                            },
                            "children": [
                                {
                                    "type": "button",
                                    "children": ["Demo Button"]
                                },
                                {
                                    "type": "input",
                                    "props": {
                                        "placeholder": "Test input..."
                                    }
                                }
                            ]
                        }
                    ]
                }"#;
                let _ = jc.load_from_string(fallback_json);
            }
            jc
        };

        if let Err(e) = json_canvas.start_hot_reload() {
            eprintln!("Error starting hot reload: {}", e);
        }

        Self {
            focus_handle: cx.focus_handle(),
            json_canvas,
        }
    }

    fn reload_ui(&mut self, _action: &ReloadUI, _window: &mut Window, cx: &mut Context<Self>) {
        if let Err(e) = self.json_canvas.reload() {
            eprintln!("Error reloading JSON UI: {}", e);
        }
        cx.notify();
    }
}

impl super::Story for JsonUIStory {
    fn title() -> &'static str {
        "JSON UI Canvas"
    }

    fn description() -> &'static str {
        "Dynamic UI system using JSON with hot reload capabilities."
    }

    fn closable() -> bool {
        false
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl Focusable for JsonUIStory {
    fn focus_handle(&self, _: &gpui::App) -> gpui::FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for JsonUIStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .on_action(cx.listener(Self::reload_ui))
            .child(
                section("JSON UI Canvas Demo")
                    .child(
                        v_flex()
                            .gap_4()
                            .child(
                                h_flex()
                                    .gap_3()
                                    .items_center()
                                    .child("JSON File Path: assets/json_ui_examples/complex.json")
                                    .child(
                                        Button::new("reload-btn")
                                            .icon(IconName::LoaderCircle)
                                            .label("Reload")
                                            .on_click(cx.listener(|_view, _event, _window, cx| {
                                                cx.dispatch_action(&ReloadUI);
                                            }))
                                    )
                            )
                    )
            )
            .child(
                section("Live JSON UI Preview with Hot Reload")
                    .child(
                        v_flex()
                            .gap_4()
                            .min_h(px(400.0))
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child("Rendered UI:")
                                    .child(
                                        h_flex()
                                            .gap_1()
                                            .items_center()
                                            .child("🔥 Hot Reload Active")
                                            .text_xs()
                                            .text_color(gpui::green())
                                    )
                            )
                            .child(
                                v_flex()
                                    .p_4()
                                    .border_1()
                                    .border_color(gpui::blue())
                                    .rounded_md()
                                    .gap_3()
                                    .child(
                                        h_flex()
                                            .gap_2()
                                            .items_center()
                                            .child("📄 File: complex.json")
                                    )
                                    .child(
                                        if self.json_canvas.is_loaded() {
                                            // Render the JSON UI content
                                            use gpui_component::json_ui::UiRenderer;
                                            if let Some(ui) = self.json_canvas.get_ui() {
                                                UiRenderer::render_component_generic(ui, cx)
                                            } else {
                                                div()
                                                    .p_4()
                                                    .child("Loading JSON UI...")
                                                    .into_any_element()
                                            }
                                        } else {
                                            div()
                                                .gap_2()
                                                .child("⚠️ File not found or invalid JSON")
                                                .child("Check the file path and try reloading")
                                                .into_any_element()
                                        }
                                    )
                            )
                    )
            )
            .child(
                section("How to Test Hot Reload")
                    .child(
                        v_flex()
                            .gap_3()
                            .child("1. Edit any of these JSON files:")
                            .child("   • assets/json_ui_examples/complex.json")
                            .child("   • assets/json_ui_examples/header_component.json")
                            .child("   • assets/json_ui_examples/card_component.json")
                            .child("2. Change colors, text, or add components")
                            .child("3. Save the file and click Reload to see changes")
                            .child("")
                            .child("✅ JSON-based UI definition")
                            .child("✅ Component references with $ref")
                            .child("✅ Property interpolation with ${variables}")
                            .child("✅ Hot reload capabilities")
                            .child("✅ Nested component structures")
                            .child("✅ Web-dev-like experience in native apps")
                    )
            )
    }
}