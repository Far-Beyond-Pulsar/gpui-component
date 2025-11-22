use gpui::{prelude::*, *};
use ui::{h_flex, v_flex, button::{Button, ButtonVariants}, divider::Divider, ActiveTheme, StyledExt};
use ui_types_common::*;
use std::path::PathBuf;
use crate::{type_block::{TypeBlock, TypeBlockView}, constructor_palette::ConstructorPalette};

#[derive(Clone, Debug)]
pub enum AliasEditorEvent {
    AliasSaved(String),
    AliasClosed,
}

pub struct AliasEditor {
    asset: AliasAsset,
    file_path: Option<PathBuf>,
    project_root: PathBuf,
    is_dirty: bool,
    index_manager: IndexManager,
    type_index: TypeIndex,
    focus_handle: FocusHandle,

    // Block-based editing
    root_block: Option<TypeBlock>,
    palette: ConstructorPalette,
    selected_palette_block: Option<String>,
}

impl AliasEditor {
    pub fn new(project_root: PathBuf, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let index_manager = IndexManager::new(project_root.clone());
        let type_index = index_manager.load_index().unwrap_or_default();

        let asset = AliasAsset {
            schema_version: 1,
            type_kind: TypeKind::Alias,
            name: "new_alias".to_string(),
            display_name: "NewAlias".to_string(),
            description: None,
            ast: TypeAstNode::Primitive { name: "()".to_string() },
            meta: serde_json::Value::Null,
        };

        let root_block = Some(TypeBlock::primitive("()"));

        Self {
            asset,
            file_path: None,
            project_root,
            is_dirty: true,
            index_manager,
            type_index,
            focus_handle: cx.focus_handle(),
            root_block,
            palette: ConstructorPalette::new(),
            selected_palette_block: None,
        }
    }

    pub fn open(file_path: PathBuf, project_root: PathBuf, _window: &mut Window, cx: &mut Context<Self>) -> anyhow::Result<Self> {
        let index_manager = IndexManager::new(project_root.clone());
        let type_index = index_manager.load_index().unwrap_or_default();

        let json_content = std::fs::read_to_string(&file_path)?;
        let asset: AliasAsset = serde_json::from_str(&json_content)?;

        // Convert AST to visual blocks
        let root_block = Some(TypeBlock::from_ast(&asset.ast));

        Ok(Self {
            asset,
            file_path: Some(file_path),
            project_root,
            is_dirty: false,
            index_manager,
            type_index,
            focus_handle: cx.focus_handle(),
            root_block,
            palette: ConstructorPalette::new(),
            selected_palette_block: None,
        })
    }

    pub fn set_root_block(&mut self, block: TypeBlock, cx: &mut Context<Self>) {
        self.root_block = Some(block);
        self.is_dirty = true;
        cx.notify();
    }

    pub fn add_primitive_block(&mut self, name: String, cx: &mut Context<Self>) {
        self.root_block = Some(TypeBlock::primitive(name));
        self.is_dirty = true;
        cx.notify();
    }

    pub fn add_constructor_block(&mut self, name: String, param_count: usize, cx: &mut Context<Self>) {
        self.root_block = Some(TypeBlock::constructor(name, param_count));
        self.is_dirty = true;
        cx.notify();
    }

    pub fn save(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        // Convert blocks back to AST
        if let Some(block) = &self.root_block {
            if let Some(ast) = block.to_ast() {
                self.asset.ast = ast;
            } else {
                anyhow::bail!("Cannot save: some type slots are empty");
            }
        } else {
            anyhow::bail!("Cannot save: no type defined");
        }

        validate_alias(&self.asset, &self.type_index)?;

        let file_path = if let Some(path) = &self.file_path {
            path.clone()
        } else {
            self.index_manager.ensure_type_dir(TypeKind::Alias, &self.asset.name)?;
            self.index_manager.get_json_path(TypeKind::Alias, &self.asset.name)
        };

        let json = serde_json::to_string_pretty(&self.asset)?;
        std::fs::write(&file_path, json)?;

        let rust_code = generate_alias(&self.asset)?;
        let rs_path = self.index_manager.get_rs_path(TypeKind::Alias, &self.asset.name);
        std::fs::write(&rs_path, rust_code)?;

        let mut index = self.index_manager.load_index().unwrap_or_default();
        let entry = TypeIndexEntry::new(TypeKind::Alias, self.asset.name.clone(), self.asset.display_name.clone());
        index.upsert(TypeKind::Alias, entry)?;
        self.index_manager.save_index(&mut index)?;

        self.is_dirty = false;
        self.file_path = Some(file_path);
        self.type_index = index;

        cx.emit(AliasEditorEvent::AliasSaved(self.asset.name.clone()));
        cx.notify();

        Ok(())
    }
}

impl EventEmitter<AliasEditorEvent> for AliasEditor {}

impl Focusable for AliasEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AliasEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_header(window, cx))
            .child(Divider::horizontal())
            .child(
                h_flex()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.palette.render(window))
                    .child(Divider::vertical())
                    .child(self.render_workspace(window, cx))
                    .child(Divider::vertical())
                    .child(self.render_preview_panel(window, cx))
            )
    }
}

impl AliasEditor {
    fn render_header(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .px_4()
            .py_3()
            .bg(cx.theme().secondary)
            .items_center()
            .justify_between()
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(div().text_lg().child("🔗"))
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .text_color(cx.theme().foreground)
                            .child(&self.asset.display_name)
                    )
                    .when(self.is_dirty, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("• unsaved")
                        )
                    })
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("clear-canvas")
                            .label("Clear")
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.root_block = Some(TypeBlock::primitive("()"));
                                editor.is_dirty = true;
                                cx.notify();
                            }))
                    )
                    .child(
                        Button::new("save")
                            .primary()
                            .label("Save")
                            .disabled(!self.is_dirty)
                            .on_click(cx.listener(|editor, _, _, cx| {
                                if let Err(e) = editor.save(cx) {
                                    eprintln!("Failed to save alias: {}", e);
                                }
                            }))
                    )
            )
    }

    fn render_workspace(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .bg(cx.theme().background.darken(0.02))
            .p_6()
            .gap_4()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Type Canvas (drag blocks from left palette)")
            )
            .child(
                // The main type block visualization
                div()
                    .p_6()
                    .bg(cx.theme().background)
                    .rounded(px(12.0))
                    .border_2()
                    .border_color(cx.theme().border)
                    .shadow_lg()
                    .min_w(px(300.0))
                    .child(
                        if let Some(block) = &self.root_block {
                            div().child(TypeBlockView::new(block.clone(), "root-block").render(window))
                        } else {
                            div()
                                .px_6()
                                .py_4()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child("No type defined")
                        }
                    )
            )
            .child(
                // Quick action buttons
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("add-box")
                            .label("+ Box<T>")
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.add_constructor_block("Box".to_string(), 1, cx);
                            }))
                    )
                    .child(
                        Button::new("add-arc")
                            .label("+ Arc<T>")
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.add_constructor_block("Arc".to_string(), 1, cx);
                            }))
                    )
                    .child(
                        Button::new("add-vec")
                            .label("+ Vec<T>")
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.add_constructor_block("Vec".to_string(), 1, cx);
                            }))
                    )
                    .child(
                        Button::new("add-result")
                            .label("+ Result<T, E>")
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.add_constructor_block("Result".to_string(), 2, cx);
                            }))
                    )
            )
    }

    fn render_preview_panel(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(350.0))
            .bg(cx.theme().secondary.opacity(0.5))
            .p_4()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child("Preview")
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Type Expression")
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .bg(cx.theme().background)
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(cx.theme().border)
                            .text_sm()
                            .font_family("monospace")
                            .text_color(cx.theme().accent)
                            .child(self.render_type_expression())
                    )
            )
            .child(Divider::horizontal())
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Generated Rust Code")
            )
            .child(
                div()
                    .flex_1()
                    .p_3()
                    .bg(cx.theme().background)
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(cx.theme().border)
                    .overflow_y_scroll()
                    .child(
                        div()
                            .text_xs()
                            .font_family("monospace")
                            .text_color(cx.theme().foreground)
                            .child(self.generate_preview_code())
                    )
            )
    }

    fn render_type_expression(&self) -> String {
        if let Some(block) = &self.root_block {
            if let Some(ast) = block.to_ast() {
                render_ast_node(&ast)
            } else {
                "Incomplete type (fill all slots)".to_string()
            }
        } else {
            "()".to_string()
        }
    }

    fn generate_preview_code(&self) -> String {
        // Update asset with current blocks
        let mut preview_asset = self.asset.clone();

        if let Some(block) = &self.root_block {
            if let Some(ast) = block.to_ast() {
                preview_asset.ast = ast;
            } else {
                return "// Incomplete type\n// Fill all type slots to see generated code".to_string();
            }
        }

        generate_alias(&preview_asset).unwrap_or_else(|e| {
            format!("// Error generating code:\n// {}", e)
        })
    }
}
