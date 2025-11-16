//! Rendering - GPUI render implementation

use gpui::*;
use gpui::prelude::*;
use ui::{dock::{Panel, PanelEvent, PanelState}, h_flex, v_flex, ActiveTheme};
use super::core::BlueprintEditorPanel;
use super::super::toolbar::ToolbarRenderer;
use super::super::{DuplicateNode, DeleteNode, CopyNode, PasteNode, DisconnectPin, OpenEngineLibraryRequest};

impl Panel for BlueprintEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Blueprint Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(div().text_sm().child("⚡"))
            .child(div().text_sm().child(if let Some(title) = &self.tab_title {
                title.clone()
            } else {
                "Blueprint Editor".to_string()
            }))
            .into_any_element()
    }

    fn dump(&self, _cx: &App) -> PanelState {
        PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for BlueprintEditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for BlueprintEditorPanel {}
impl EventEmitter<OpenEngineLibraryRequest> for BlueprintEditorPanel {}

impl Render for BlueprintEditorPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use ui::resizable::{h_resizable, v_resizable, resizable_panel};
        use ui::tab::{Tab, TabBar};
        use ui::{button::{Button, ButtonVariants}, IconName};
        use super::super::{macros, variables, properties, node_graph::NodeGraphRenderer};
        
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .on_action(cx.listener(|panel, action: &DuplicateNode, _window, cx| {
                panel.duplicate_node(action.node_id.clone(), cx);
            }))
            .on_action(cx.listener(|panel, action: &DeleteNode, _window, cx| {
                panel.delete_node(action.node_id.clone(), cx);
            }))
            .on_action(cx.listener(|panel, action: &CopyNode, _window, cx| {
                panel.copy_node(action.node_id.clone(), cx);
            }))
            .on_action(cx.listener(|panel, _action: &PasteNode, _window, cx| {
                panel.paste_node(cx);
            }))
            .on_action(cx.listener(|panel, action: &DisconnectPin, _window, cx| {
                panel.disconnect_pin(action.node_id.clone(), action.pin_id.clone(), cx);
            }))
            .child(ToolbarRenderer::render(self, cx))
            .child(self.render_tab_bar(cx))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_row()
                    .min_h_0()
                    .child(
                        h_resizable("blueprint-editor-panels", self.resizable_state.clone())
                            .child(
                                // Left sidebar: Macros (top) + Variables (bottom)
                                resizable_panel()
                                    .size(px(280.))
                                    .size_range(px(200.)..px(400.))
                                    .child(
                                        v_resizable("left-sidebar-split", self.left_sidebar_resizable_state.clone())
                                            .child(
                                                resizable_panel()
                                                    .size(px(200.))
                                                    .size_range(px(150.)..px(500.))
                                                    .child(macros::MacrosRenderer::render(self, cx))
                                            )
                                            .child(
                                                resizable_panel()
                                                    .child(variables::VariablesRenderer::render(self, cx))
                                            )
                                    )
                            )
                            .child(
                                // Center: Node Graph
                                resizable_panel()
                                    .child(NodeGraphRenderer::render(self, cx))
                            )
                            .child(
                                // Right sidebar: Properties
                                resizable_panel()
                                    .size(px(250.))
                                    .size_range(px(200.)..px(400.))
                                    .child(properties::PropertiesRenderer::render(self, cx))
                            )
                    )
            )
            .when_some(self.node_creation_menu.clone(), |this, menu| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .w_full()
                        .h_full()
                        .occlude()
                        .child(div().absolute().child(menu))
                )
            })
            .when_some(self.hoverable_tooltip.clone(), |this, tooltip| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .w_full()
                        .h_full()
                        .child(div().absolute().child(tooltip))
                )
            })
    }
}

impl BlueprintEditorPanel {
    /// Render tab bar for graph navigation
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        use ui::tab::{Tab, TabBar};
        use ui::{button::{Button, ButtonVariants}, IconName};
        
        TabBar::new("graph-tabs")
            .w_full()
            .bg(cx.theme().secondary)
            .border_b_1()
            .border_color(cx.theme().border)
            .selected_index(self.active_tab_index)
            .on_click(cx.listener(|this, index: &usize, _window, cx| {
                this.switch_to_tab(*index, cx);
            }))
            .children(
                self.open_tabs.iter().enumerate().map(|(index, tab)| {
                    let mut tab_widget = Tab::new(tab.name.clone());
                    
                    // Add close button if not main tab
                    if !tab.is_main {
                        let tab_index = index;
                        tab_widget = tab_widget.child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    Button::new(("close-tab", index))
                                        .icon(IconName::Close)
                                        .ghost()
                                        .on_click(cx.listener(move |this, _, _window, cx| {
                                            this.close_tab(tab_index, cx);
                                        }))
                                )
                        );
                    }
                    
                    // Add dirty indicator
                    if tab.is_dirty {
                        tab_widget = tab_widget.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().accent)
                                .child("*")
                        );
                    }
                    
                    tab_widget
                })
            )
    }
}
