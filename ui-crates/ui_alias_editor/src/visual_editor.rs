use gpui::{*, prelude::FluentBuilder, actions};
use ui::{v_flex, h_flex, ActiveTheme, StyledExt, dock::{Panel, PanelEvent}, button::{Button, ButtonVariant, ButtonVariants}, divider::Divider};
use ui_types_common::{AliasAsset, TypeAstNode};
use std::path::PathBuf;
use crate::{TypeBlock, BlockCanvas, ConstructorPalette};

actions!(visual_alias_editor, [Save, TogglePalette]);

/// Visual block-based type alias editor with Scratch-style interface
pub struct VisualAliasEditor {
    file_path: Option<PathBuf>,
    name: String,
    display_name: String,
    description: String,
    
    /// Canvas for composing type blocks
    canvas: BlockCanvas,
    
    /// Palette for selecting types
    palette: ConstructorPalette,
    
    /// Whether palette is visible
    show_palette: bool,
    
    /// Error message to display
    error_message: Option<String>,
    
    /// Code preview panel visible
    show_preview: bool,
    
    focus_handle: FocusHandle,
}

impl VisualAliasEditor {
    pub fn new_with_file(file_path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Try to load the alias data
        let (name, display_name, description, root_block, error_message) =
            match std::fs::read_to_string(&file_path) {
                Ok(json_content) => {
                    match serde_json::from_str::<AliasAsset>(&json_content) {
                        Ok(asset) => (
                            asset.name.clone(),
                            asset.display_name.clone(),
                            asset.description.unwrap_or_default(),
                            Some(TypeBlock::from_ast(&asset.ast)),
                            None,
                        ),
                        Err(e) => (
                            String::new(),
                            "New Alias".to_string(),
                            String::new(),
                            None,
                            Some(format!("Failed to parse: {}", e)),
                        ),
                    }
                }
                Err(_) => {
                    // New file
                    (
                        String::new(),
                        "New Alias".to_string(),
                        String::new(),
                        None,
                        None,
                    )
                }
            };

        let canvas = if let Some(block) = root_block {
            BlockCanvas::with_root(block)
        } else {
            BlockCanvas::new()
        };

        Self {
            file_path: Some(file_path),
            name,
            display_name,
            description,
            canvas,
            palette: ConstructorPalette::new(),
            show_palette: true,
            error_message,
            show_preview: true,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn file_path(&self) -> Option<PathBuf> {
        self.file_path.clone()
    }

    fn save(&mut self, _: &Save, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(file_path) = &self.file_path {
            if let Some(root_block) = self.canvas.root_block() {
                if let Some(ast) = root_block.to_ast() {
                    let asset = AliasAsset {
                        schema_version: 1,
                        type_kind: ui_types_common::TypeKind::Alias,
                        name: self.name.clone(),
                        display_name: self.display_name.clone(),
                        description: if self.description.is_empty() {
                            None
                        } else {
                            Some(self.description.clone())
                        },
                        ast,
                        meta: serde_json::Value::Object(serde_json::Map::new()),
                    };

                    match serde_json::to_string_pretty(&asset) {
                        Ok(json) => {
                            if let Err(e) = std::fs::write(file_path, json) {
                                self.error_message = Some(format!("Failed to save: {}", e));
                            } else {
                                self.error_message = None;
                                // TODO: Generate Rust code and update type index
                                eprintln!("✅ Saved type alias to {:?}", file_path);
                            }
                        }
                        Err(e) => {
                            self.error_message = Some(format!("Failed to serialize: {}", e));
                        }
                    }
                } else {
                    self.error_message = Some("Type has empty slots - fill all slots before saving".to_string());
                }
            } else {
                self.error_message = Some("Cannot save empty type".to_string());
            }
        }
        cx.notify();
    }

    fn toggle_palette(&mut self, _: &TogglePalette, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_palette = !self.show_palette;
        cx.notify();
    }

    /// Render code preview panel
    fn render_preview(&self, cx: &App) -> impl IntoElement {
        let code = if let Some(root) = self.canvas.root_block() {
            if let Some(ast) = root.to_ast() {
                self.generate_preview_code(&ast)
            } else {
                "// Fill all slots to see generated code".to_string()
            }
        } else {
            "// Drag types to the canvas to start".to_string()
        };

        v_flex()
            .w(px(350.0))
            .h_full()
            .bg(cx.theme().sidebar)
            .border_l_2()
            .border_color(cx.theme().border)
            .child(
                // Header
                h_flex()
                    .w_full()
                    .px_4()
                    .py_3()
                    .bg(cx.theme().secondary)
                    .border_b_2()
                    .border_color(cx.theme().border)
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .text_color(cx.theme().foreground)
                            .child("📋 Code Preview")
                    )
            )
            .child(
                // Code display
                v_flex()
                    .flex_1()
                    .overflow_y_scroll()
                    .p_4()
                    .child(
                        div()
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(code)
                    )
            )
    }

    fn generate_preview_code(&self, ast: &TypeAstNode) -> String {
        let type_str = self.ast_to_rust_string(ast);
        
        format!(
            "// Auto-generated Rust type alias\n\
             pub type {} = {};\n\n\
             // Usage example:\n\
             // let value: {} = ...;",
            self.display_name,
            type_str,
            self.display_name
        )
    }

    fn ast_to_rust_string(&self, ast: &TypeAstNode) -> String {
        match ast {
            TypeAstNode::Primitive { name } => name.clone(),
            TypeAstNode::Path { path } => path.clone(),
            TypeAstNode::AliasRef { alias } => alias.clone(),
            TypeAstNode::Constructor { name, params, .. } => {
                let params_str = params
                    .iter()
                    .map(|p| self.ast_to_rust_string(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", name, params_str)
            }
            TypeAstNode::Tuple { elements } => {
                let elements_str = elements
                    .iter()
                    .map(|e| self.ast_to_rust_string(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", elements_str)
            }
            TypeAstNode::FnPointer { params, return_type } => {
                let params_str = params
                    .iter()
                    .map(|p| self.ast_to_rust_string(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fn({}) -> {}", params_str, self.ast_to_rust_string(return_type))
            }
        }
    }
}

impl Render for VisualAliasEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Top toolbar
                h_flex()
                    .w_full()
                    .px_4()
                    .py_3()
                    .gap_4()
                    .bg(cx.theme().secondary.opacity(0.5))
                    .border_b_2()
                    .border_color(cx.theme().border)
                    .items_center()
                    .child(
                        // Icon and title
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(div().text_xl().child("🔗"))
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child(if !self.display_name.is_empty() {
                                        self.display_name.clone()
                                    } else {
                                        "New Type Alias".to_string()
                                    })
                            )
                    )
                    .child(
                        // Spacer
                        div().flex_1()
                    )
                    .child(
                        // Action buttons
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("toggle_palette_btn")
                                    .with_variant(if self.show_palette {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    })
                                    .child(if self.show_palette { "🎨 Hide Library" } else { "🎨 Show Library" })
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.toggle_palette(&TogglePalette, window, cx);
                                    }))
                            )
                            .child(
                                Button::new("toggle_preview_btn")
                                    .with_variant(if self.show_preview {
                                        ButtonVariant::Secondary
                                    } else {
                                        ButtonVariant::Ghost
                                    })
                                    .child(if self.show_preview { "📋 Hide Preview" } else { "📋 Show Preview" })
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.show_preview = !this.show_preview;
                                        cx.notify();
                                    }))
                            )
                            .child(Divider::vertical().h(px(24.0)))
                            .child(
                                Button::new("save_btn")
                                    .with_variant(ButtonVariant::Primary)
                                    .child("💾 Save")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.save(&Save, window, cx);
                                    }))
                            )
                    )
            )
            .child(
                // Main content area - three-panel layout
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .when(self.show_palette, |this| {
                        this.child(self.palette.render(_window))
                    })
                    .child(
                        // Center canvas
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .p_4()
                                    .gap_4()
                                    .when(self.error_message.is_some(), |this| {
                                        let error = self.error_message.as_ref().unwrap();
                                        this.child(
                                            div()
                                                .w_full()
                                                .p_4()
                                                .bg(hsla(0.0, 0.8, 0.5, 0.1))
                                                .border_2()
                                                .border_color(hsla(0.0, 0.8, 0.6, 1.0))
                                                .rounded(px(8.0))
                                                .child(
                                                    h_flex()
                                                        .gap_2()
                                                        .items_center()
                                                        .child(
                                                            div()
                                                                .text_base()
                                                                .child("⚠️")
                                                        )
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .text_color(hsla(0.0, 0.8, 0.5, 1.0))
                                                                .child(error.clone())
                                                        )
                                                )
                                        )
                                    })
                                    .child(
                                        // Canvas
                                        div()
                                            .flex_1()
                                            .child(self.canvas.render(_window))
                                    )
                            )
                    )
                    .when(self.show_preview, |this| {
                        this.child(self.render_preview(cx.app()))
                    })
            )
            .when(!self.name.is_empty() && !self.description.is_empty(), |this| {
                // Bottom info bar
                this.child(
                    h_flex()
                        .w_full()
                        .px_4()
                        .py_2()
                        .gap_4()
                        .bg(cx.theme().secondary.opacity(0.3))
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("name: {}", &self.name))
                        )
                        .child(Divider::vertical().h(px(12.0)))
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(&self.description)
                        )
                )
            })
    }
}

impl Focusable for VisualAliasEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for VisualAliasEditor {}

impl Panel for VisualAliasEditor {
    fn panel_name(&self) -> &'static str {
        "Visual Type Alias Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> gpui::AnyElement {
        if !self.display_name.is_empty() {
            format!("🔗 {}", self.display_name)
        } else {
            "🔗 New Type Alias".to_string()
        }
        .into_any_element()
    }

    fn dump(&self, _cx: &App) -> ui::dock::PanelState {
        ui::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}
