use gpui::*;
use gpui_component::{
    button::Button,
    h_flex,
    ActiveTheme as _, StyledExt,
    IconName,
};

use super::panel::BlueprintEditorPanel;

pub struct ToolbarRenderer;

impl ToolbarRenderer {
    pub fn render(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        // For now, use a simple toolbar layout since we don't have the shared Toolbar component
        h_flex()
            .w_full()
            .p_3()
            .bg(cx.theme().secondary)
            .border_b_1()
            .border_color(cx.theme().border)
            .gap_2()
            .child(
                Button::new("add_node")
                    .icon(IconName::Plus)
                    .tooltip("Add Node (A)")
                    .on_click(cx.listener(|_panel, _, _window, _cx| {
                        // TODO: Open node selection dialog
                    }))
            )
            .child(
                Button::new("duplicate")
                    .icon(IconName::Copy)
                    .tooltip("Duplicate Node (Ctrl+D)")
                    .on_click(cx.listener(|_panel, _, _window, _cx| {
                        // TODO: Duplicate selected nodes
                    }))
            )
            .child(
                Button::new("delete")
                    .icon(IconName::Delete)
                    .tooltip("Delete Node (Del)")
                    .on_click(cx.listener(|_panel, _, _window, _cx| {
                        // TODO: Delete selected nodes
                    }))
            )
            .child(
                div()
                    .w_px()
                    .h_6()
                    .bg(cx.theme().border)
                    .mx_2()
            )
            .child(
                Button::new("zoom_in")
                    .icon(IconName::Plus)
                    .tooltip("Zoom In (+)")
                    .on_click(cx.listener(|panel, _, _window, cx| {
                        let graph = panel.get_graph_mut();
                        graph.zoom_level = (graph.zoom_level * 1.2).min(3.0);
                        cx.notify();
                    }))
            )
            .child(
                Button::new("zoom_out")
                    .icon(IconName::Minus)
                    .tooltip("Zoom Out (-)")
                    .on_click(cx.listener(|panel, _, _window, cx| {
                        let graph = panel.get_graph_mut();
                        graph.zoom_level = (graph.zoom_level / 1.2).max(0.2);
                        cx.notify();
                    }))
            )
            .child(
                Button::new("fit_view")
                    .icon(IconName::CircleCheck)
                    .tooltip("Fit to View (F)")
                    .on_click(cx.listener(|panel, _, _window, cx| {
                        let graph = panel.get_graph_mut();
                        graph.zoom_level = 1.0;
                        graph.pan_offset = Point::new(0.0, 0.0);
                        cx.notify();
                    }))
            )
            .child(
                div().flex_1() // Spacer
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("Zoom: {:.0}%", panel.get_graph().zoom_level * 100.0))
            )
    }
}
