//! Menu operations - node creation menu and context menus

use gpui::*;
use super::core::BlueprintEditorPanel;
use super::super::node_creation_menu::{NodeCreationMenu, NodeCreationEvent};
use super::super::hoverable_tooltip::HoverableTooltip;
use super::{NODE_MENU_WIDTH, NODE_MENU_MAX_HEIGHT};
use smol::Timer;
use std::time::Duration;

impl BlueprintEditorPanel {
    /// Show node creation menu at screen position
    pub fn show_node_creation_menu(
        &mut self,
        window_pos: Point<Pixels>,
        graph_pos: Point<f32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Store BOTH positions:
        // - graph_pos: for placing the created node in the correct graph location  
        // - window_pos: for rendering the menu at the correct window location (in Pixels)

        // Create search input for the menu
        let search_input = cx.new(|cx| ui::input::InputState::new(window, cx));
        let panel_weak = cx.weak_entity();
        
        // Pass graph_pos for placing nodes, not window_pos for rendering
        let menu = cx.new(|cx| NodeCreationMenu::new(graph_pos, search_input, panel_weak, cx));
        let menu_entity = menu.clone();

        cx.subscribe_in(&menu, window, move |panel, _menu, event, _window, cx| {
            panel.on_node_creation_event(menu_entity.clone(), event, cx);
        })
        .detach();

        self.node_creation_menu = Some(menu);
        self.node_creation_menu_position = Some(graph_pos); // For placing nodes
        self.node_creation_menu_position_window = Some(window_pos); // For rendering menu
        cx.notify();
    }

    /// Handle node creation events
    fn on_node_creation_event(
        &mut self,
        _menu: Entity<NodeCreationMenu>,
        event: &NodeCreationEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            NodeCreationEvent::CreateNode(node) => {
                self.add_node(node.clone(), cx);
                self.dismiss_node_creation_menu(cx);
            }
            NodeCreationEvent::Dismiss => {
                self.dismiss_node_creation_menu(cx);
            }
        }
    }

    /// Dismiss node creation menu
    pub fn dismiss_node_creation_menu(&mut self, cx: &mut Context<Self>) {
        self.node_creation_menu = None;
        self.node_creation_menu_position = None;
        self.node_creation_menu_position_window = None;
        cx.notify();
    }

    /// Check if position is inside menu bounds
    pub fn is_position_inside_menu(&self, screen_pos: Point<f32>) -> bool {
        if let (Some(_), Some(position)) = (&self.node_creation_menu, &self.node_creation_menu_position) {
            let menu_left = position.x;
            let menu_top = position.y;
            let menu_right = menu_left + NODE_MENU_WIDTH;
            let menu_bottom = menu_top + NODE_MENU_MAX_HEIGHT;

            screen_pos.x >= menu_left
                && screen_pos.x <= menu_right
                && screen_pos.y >= menu_top
                && screen_pos.y <= menu_bottom
        } else {
            false
        }
    }

    /// Show hoverable tooltip with delay
    pub fn show_hoverable_tooltip(
        &mut self,
        content: String,
        position: Point<f32>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.pending_tooltip = Some((content.clone(), position));

        cx.spawn(async move |view, cx| {
            Timer::after(Duration::from_secs(2)).await;

            cx.update(|cx| {
                let _ = view.update(cx, |panel, cx| {
                    if let Some((pending_content, pending_pos)) = panel.pending_tooltip.take() {
                        let pixel_pos = Point::new(px(pending_pos.x), px(pending_pos.y));
                        panel.hoverable_tooltip = Some(HoverableTooltip::new(pending_content, pixel_pos, cx));
                        cx.notify();
                    }
                });
            })
            .ok();
        })
        .detach();
    }

    /// Hide hoverable tooltip
    pub fn hide_hoverable_tooltip(&mut self, cx: &mut Context<Self>) {
        self.hoverable_tooltip = None;
        self.pending_tooltip = None;
        cx.notify();
    }

    /// Update tooltip position
    pub fn update_tooltip_position(&mut self, position: Point<f32>, cx: &mut Context<Self>) {
        if let Some(tooltip) = &self.hoverable_tooltip {
            let pixel_pos = Point::new(px(position.x), px(position.y));
            tooltip.update(cx, |tooltip, cx| {
                tooltip.set_position(pixel_pos, cx);
            });
        }
    }

    /// Check if mouse is still over tooltip area
    pub fn check_tooltip_hover(&mut self, mouse_pos: Point<f32>, cx: &mut Context<Self>) {
        if let Some(tooltip) = &self.hoverable_tooltip {
            let pixel_pos = Point::new(px(mouse_pos.x), px(mouse_pos.y));
            tooltip.update(cx, |tooltip, cx| {
                tooltip.check_to_hide(pixel_pos, cx);
            });

            let is_open = tooltip.read(cx).open;
            if !is_open {
                self.hoverable_tooltip = None;
                cx.notify();
            }
        }
    }
}
