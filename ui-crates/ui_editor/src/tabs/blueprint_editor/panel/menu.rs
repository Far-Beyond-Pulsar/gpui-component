//! Menu operations - node picker (command palette-style)

use gpui::*;
use super::core::BlueprintEditorPanel;
use super::super::node_picker::{NodePicker, NodeSelected};
use super::super::{BlueprintNode, NodeDefinitions};

impl BlueprintEditorPanel {
    /// Show node picker at graph position
    pub fn show_node_picker(
        &mut self,
        graph_pos: Point<f32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let graph_pos_pixels = Point::new(px(graph_pos.x), px(graph_pos.y));
        let picker = cx.new(|cx| NodePicker::new(graph_pos_pixels, window, cx));
        
        cx.subscribe(&picker, move |panel, _picker, event: &NodeSelected, cx| {
            panel.on_node_selected(event, cx);
        })
        .detach();
        
        cx.subscribe(&picker, move |panel, _picker, _event: &DismissEvent, cx| {
            panel.dismiss_node_picker(cx);
        })
        .detach();

        self.node_picker = Some(picker);
        cx.notify();
    }

    /// Handle node selection from picker
    fn on_node_selected(&mut self, event: &NodeSelected, cx: &mut Context<Self>) {
        // Convert NodeInfo to BlueprintNode
        let node_definitions = NodeDefinitions::load();
        if let Some(def) = node_definitions.get_node_definition_by_name(&event.node.name) {
            let graph_pos = Point::new(event.position.0.0, event.position.0.0); // Convert from Pixels to f32
            let node = BlueprintNode::from_definition(def, graph_pos);
            self.add_node(node, cx);
        }
        
        self.dismiss_node_picker(cx);
    }

    /// Dismiss node picker
    pub fn dismiss_node_picker(&mut self, cx: &mut Context<Self>) {
        self.node_picker = None;
        cx.notify();
    }
}
