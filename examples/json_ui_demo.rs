use gpui::*;
use ui::json_ui::*;

struct JsonUiDemo {
    canvas: View<JsonCanvas>,
}

impl JsonUiDemo {
    fn new(cx: &mut ViewContext<Self>) -> Self {
        let canvas = create_json_canvas_view("examples/json_ui/app.json", cx.window_context());

        Self { canvas }
    }
}

impl Render for JsonUiDemo {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .child("JSON UI Demo")
            .child(self.canvas.clone())
    }
}

fn main() {
    App::new().run(|cx: &mut AppContext| {
        cx.open_window(WindowOptions::default(), |cx| {
            cx.new_view(|cx| JsonUiDemo::new(cx))
        })
        .unwrap();
    });
}