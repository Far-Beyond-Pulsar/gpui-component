//! Menu operations - node picker (command palette-style)

use gpui::*;
use ui_common::command_palette::GenericPalette;
use super::core::BlueprintEditorPanel;
use super::super::node_palette::NodePalette;
use super::super::BlueprintNode;

impl BlueprintEditorPanel {
    /// Show node picker at graph position
    pub fn show_node_picker(
        &mut self,
        graph_pos: Point<f32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Create palette delegate
        let palette = NodePalette::new(graph_pos);
        let picker = cx.new(|cx| GenericPalette::new(palette, window, cx));

        cx.subscribe(&picker, move |panel, picker, _event: &DismissEvent, cx| {
            // Check if a node was selected
            let node_to_add = picker.update(cx, |picker_state, _cx| {
                picker_state.delegate_mut().take_selected_node()
            });

            if let Some((node_def, pos)) = node_to_add {
                // Create and add the node
                let node = BlueprintNode::from_definition(&node_def, pos);
                panel.add_node(node, cx);
            }

            panel.dismiss_node_picker(cx);
        })
        .detach();

        self.node_picker = Some(picker);
        cx.notify();
    }

    /// Dismiss node picker
    pub fn dismiss_node_picker(&mut self, cx: &mut Context<Self>) {
        self.node_picker = None;
        cx.notify();
    }
}
