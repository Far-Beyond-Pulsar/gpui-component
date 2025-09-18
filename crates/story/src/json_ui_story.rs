use gpui::{
    actions, px, App, AppContext as _, Context, Entity, Focusable, InteractiveElement, IntoElement,
    ParentElement as _, Render, Styled as _, View, Window,
};

use gpui_component::{
    button::Button, h_flex, input::TextInput, json_ui::{create_json_canvas_view, JsonCanvas}, v_flex, IconName,
};

use crate::section;

actions!(json_ui_story, [ReloadUI, LoadExample]);

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExampleType {
    Basic,
    Complex,
    Interactive,
}

pub struct JsonUIStory {
    focus_handle: gpui::FocusHandle,
    current_example: ExampleType,
    json_path_input: Entity<TextInput>,
    current_json_path: String,
    json_canvas: Option<View<JsonCanvas>>,
}

impl JsonUIStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let json_path_input = cx.new(|cx| {
            TextInput::new(cx)
                .placeholder("Enter JSON file path...")
                .value("crates/story/assets/json_ui_examples/complex.json")
        });

        let initial_path = "crates/story/assets/json_ui_examples/complex.json";
        let canvas = create_json_canvas_view(initial_path, window);

        cx.new(|cx| Self {
            focus_handle: cx.focus_handle(),
            current_example: ExampleType::Complex,
            json_path_input,
            current_json_path: initial_path.to_string(),
            json_canvas: Some(canvas),
        })
    }

    fn get_example_path(&self, example_type: ExampleType) -> String {
        match example_type {
            ExampleType::Basic => "crates/story/assets/json_ui_examples/basic.json".to_string(),
            ExampleType::Complex => "crates/story/assets/json_ui_examples/complex.json".to_string(),
            ExampleType::Interactive => "crates/story/assets/json_ui_examples/interactive.json".to_string(),
        }
    }

    fn load_example(&mut self, example_type: ExampleType, window: &mut Window, cx: &mut Context<Self>) {
        self.current_example = example_type;
        let new_path = self.get_example_path(example_type);

        // Update the input field
        self.json_path_input.update(cx, |input, cx| {
            input.set_text(new_path.clone(), cx);
        });

        // Create new canvas with the example file
        self.current_json_path = new_path.clone();
        self.json_canvas = Some(create_json_canvas_view(&new_path, window));

        cx.notify();
    }

    fn reload_ui(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Get the current path from the input
        let path = self.json_path_input.read(cx).text(cx).to_string();
        self.current_json_path = path.clone();

        // Create new canvas with the specified path
        self.json_canvas = Some(create_json_canvas_view(&path, window));

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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current_example = self.current_example;

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
                                    .child("JSON File Path:")
                                    .child(
                                        self.json_path_input
                                            .clone()
                                            .w(px(300.0))
                                    )
                                    .child(
                                        Button::new("reload-btn")
                                            .icon(IconName::RotateCw)
                                            .label("Reload")
                                            .on_click(cx.listener(|view, _, window, cx| {
                                                view.reload_ui(window, cx);
                                            }))
                                    )
                            )
                            .child(
                                h_flex()
                                    .gap_3()
                                    .child("Load Example:")
                                    .child(
                                        Button::new("basic-example")
                                            .label("Basic")
                                            .when(current_example == ExampleType::Basic, |btn| btn.primary())
                                            .on_click(cx.listener(|view, _, window, cx| {
                                                view.load_example(ExampleType::Basic, window, cx);
                                            }))
                                    )
                                    .child(
                                        Button::new("complex-example")
                                            .label("Complex")
                                            .when(current_example == ExampleType::Complex, |btn| btn.primary())
                                            .on_click(cx.listener(|view, _, window, cx| {
                                                view.load_example(ExampleType::Complex, window, cx);
                                            }))
                                    )
                                    .child(
                                        Button::new("interactive-example")
                                            .label("Interactive")
                                            .when(current_example == ExampleType::Interactive, |btn| btn.primary())
                                            .on_click(cx.listener(|view, _, window, cx| {
                                                view.load_example(ExampleType::Interactive, window, cx);
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
                                    .child(IconName::Play)
                                    .child("Rendered UI:")
                                    .child(
                                        h_flex()
                                            .gap_1()
                                            .items_center()
                                            .child(IconName::Zap)
                                            .child("Hot Reload Active")
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
                                            .child(IconName::FileJson)
                                            .child(format!("File: {}", self.current_json_path))
                                    )
                                    .when_some(self.json_canvas.clone(), |div, canvas| {
                                        div.child(canvas)
                                    })
                                    .when(self.json_canvas.is_none(), |div| {
                                        div.child(
                                            v_flex()
                                                .gap_2()
                                                .child("⚠️ File not found or invalid JSON")
                                                .child("Check the file path and try reloading")
                                        )
                                    })
                            )
                    )
            )
            .child(
                section("Hot Reload Instructions")
                    .child(
                        v_flex()
                            .gap_3()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(IconName::Info)
                                    .child("How to test hot reload:")
                            )
                            .child("1. Load one of the example JSON files above")
                            .child("2. Open the JSON file in your text editor:")
                            .child(
                                v_flex()
                                    .p_2()
                                    .bg(gpui::gray())
                                    .rounded_md()
                                    .text_xs()
                                    .font_mono()
                                    .child("• crates/story/assets/json_ui_examples/basic.json")
                                    .child("• crates/story/assets/json_ui_examples/complex.json")
                                    .child("• crates/story/assets/json_ui_examples/interactive.json")
                                    .child("• crates/story/assets/json_ui_examples/header_component.json")
                                    .child("• crates/story/assets/json_ui_examples/card_component.json")
                            )
                            .child("3. Make changes to the JSON (try changing colors, text, or adding components)")
                            .child("4. Save the file and watch the UI update automatically! 🔥")
                            .child("")
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(IconName::Lightbulb)
                                    .child("Try these changes:")
                            )
                            .child("• Change 'backgroundColor': 'blue' to 'backgroundColor': 'red'")
                            .child("• Edit text content in any component")
                            .child("• Modify the 'title' or 'content' props in referenced components")
                            .child("• Add new children to any component")
                    )
            )
            .child(
                section("Features & Architecture")
                    .child(
                        v_flex()
                            .gap_3()
                            .child("✅ JSON-based UI definition with clean syntax")
                            .child("✅ Component references with $ref for modularity")
                            .child("✅ Property interpolation with ${variables}")
                            .child("✅ Real-time hot reload with file watching")
                            .child("✅ Nested component structures")
                            .child("✅ Web-dev-like experience in native apps")
                            .child("✅ Cross-file dependency tracking")
                            .child("✅ Automatic cache invalidation on file changes")
                    )
            )
    }
}