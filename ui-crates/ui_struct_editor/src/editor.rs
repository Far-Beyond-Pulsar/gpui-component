use gpui::{prelude::*, *};
use ui::{
    h_flex, v_flex, button::{Button, ButtonVariants}, label::Label, divider::Divider,
    ActiveTheme, Sizable, Size, StyleSized, StyledExt, Disableable, IconName,
};
use ui_types_common::*;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum StructEditorEvent {
    StructSaved(String),
    StructClosed,
    CodeGenerated(String),
}

pub struct StructEditor {
    /// Current struct being edited
    asset: StructAsset,
    /// Path to the .struct.json file
    file_path: Option<PathBuf>,
    /// Project root path
    project_root: PathBuf,
    /// Whether the struct has unsaved changes
    is_dirty: bool,
    /// Index manager
    index_manager: IndexManager,
    /// Current type index
    type_index: TypeIndex,
    /// Focus handle
    focus_handle: FocusHandle,
    /// Editing field name
    editing_field_name: Option<String>,
    /// Editing description
    editing_description: bool,
}

impl StructEditor {
    pub fn new(project_root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let index_manager = IndexManager::new(project_root.clone());
        let type_index = index_manager.load_index().unwrap_or_default();

        let asset = StructAsset {
            schema_version: 1,
            type_kind: TypeKind::Struct,
            name: "new_struct".to_string(),
            display_name: "NewStruct".to_string(),
            description: None,
            fields: vec![],
            visibility: Visibility::Public,
            meta: serde_json::Value::Null,
        };

        Self {
            asset,
            file_path: None,
            project_root,
            is_dirty: true,
            index_manager,
            type_index,
            focus_handle: cx.focus_handle(),
            editing_field_name: None,
            editing_description: false,
        }
    }

    pub fn open(file_path: PathBuf, project_root: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> anyhow::Result<Self> {
        let index_manager = IndexManager::new(project_root.clone());
        let type_index = index_manager.load_index().unwrap_or_default();

        // Load the struct asset
        let json_content = std::fs::read_to_string(&file_path)?;
        let asset: StructAsset = serde_json::from_str(&json_content)?;

        Ok(Self {
            asset,
            file_path: Some(file_path),
            project_root,
            is_dirty: false,
            index_manager,
            type_index,
            focus_handle: cx.focus_handle(),
            editing_field_name: None,
            editing_description: false,
        })
    }

    pub fn add_field(&mut self, cx: &mut Context<Self>) {
        let field_num = self.asset.fields.len() + 1;
        let new_field = StructField {
            name: format!("field_{}", field_num),
            type_ref: TypeRef::primitive("i32"),
            visibility: Visibility::Public,
            doc: None,
        };

        self.asset.fields.push(new_field);
        self.is_dirty = true;
        cx.notify();
    }

    pub fn remove_field(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.asset.fields.len() {
            self.asset.fields.remove(index);
            self.is_dirty = true;
            cx.notify();
        }
    }

    pub fn save(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        // Validate the struct
        validate_struct(&self.asset, &self.type_index)?;

        // Determine file path
        let file_path = if let Some(path) = &self.file_path {
            path.clone()
        } else {
            // Create new file
            self.index_manager.ensure_type_dir(TypeKind::Struct, &self.asset.name)?;
            self.index_manager.get_json_path(TypeKind::Struct, &self.asset.name)
        };

        // Save JSON
        let json = serde_json::to_string_pretty(&self.asset)?;
        std::fs::write(&file_path, json)?;

        // Generate Rust code
        let rust_code = generate_struct(&self.asset)?;
        let rs_path = self.index_manager.get_rs_path(TypeKind::Struct, &self.asset.name);
        std::fs::write(&rs_path, rust_code)?;

        // Update index
        let mut index = self.index_manager.load_index().unwrap_or_default();
        let mut entry = TypeIndexEntry::new(
            TypeKind::Struct,
            self.asset.name.clone(),
            self.asset.display_name.clone(),
        );

        if let Some(existing) = index.get(TypeKind::Struct, &self.asset.name) {
            entry.version = existing.version + 1;
        }

        index.upsert(TypeKind::Struct, entry)?;
        self.index_manager.save_index(&mut index)?;

        self.is_dirty = false;
        self.file_path = Some(file_path);
        self.type_index = index;

        cx.emit(StructEditorEvent::StructSaved(self.asset.name.clone()));
        cx.notify();

        Ok(())
    }
}

impl EventEmitter<StructEditorEvent> for StructEditor {}

impl Focusable for StructEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for StructEditor {
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
                    .child(self.render_editor_panel(window, cx))
                    .child(Divider::vertical())
                    .child(self.render_preview_panel(window, cx))
            )
    }
}

impl StructEditor {
    fn render_header(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(
                        div()
                            .text_lg()
                            .child("📦")
                    )
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
                        Button::new("save-struct")
                            .primary()
                            .label("Save")
                            .disabled(!self.is_dirty)
                            .on_click(cx.listener(|editor, _, _, cx| {
                                if let Err(e) = editor.save(cx) {
                                    eprintln!("Failed to save struct: {}", e);
                                }
                            }))
                    )
            )
    }

    fn render_editor_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .overflow_hidden()
            .p_4()
            .gap_4()
            .child(self.render_basic_info(window, cx))
            .child(Divider::horizontal())
            .child(self.render_fields_section(window, cx))
    }

    fn render_basic_info(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Name (snake_case)")
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .bg(cx.theme().muted.opacity(0.3))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(cx.theme().border)
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(&self.asset.name)
                    )
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Display Name (PascalCase)")
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .bg(cx.theme().muted.opacity(0.3))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(cx.theme().border)
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(&self.asset.display_name)
                    )
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Description")
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .bg(cx.theme().muted.opacity(0.3))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(cx.theme().border)
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(self.asset.description.as_deref().unwrap_or("No description"))
                    )
            )
    }

    fn render_fields_section(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Fields")
                    )
                    .child(
                        Button::new("add-field")
                            .small()
                            .label("+ Add Field")
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.add_field(cx);
                            }))
                    )
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_2()
                    .overflow_y_scroll()
                    .children(
                        self.asset.fields.iter().enumerate().map(|(idx, field)| {
                            self.render_field(idx, field, window, cx)
                        })
                    )
                    .when(self.asset.fields.is_empty(), |this| {
                        this.child(
                            div()
                                .p_4()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child("No fields. Click 'Add Field' to add one.")
                        )
                    })
            )
    }

    fn render_field(&self, idx: usize, field: &StructField, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let idx_for_remove = idx;

        h_flex()
            .w_full()
            .p_3()
            .gap_3()
            .bg(cx.theme().muted.opacity(0.2))
            .rounded(px(6.0))
            .border_1()
            .border_color(cx.theme().border.opacity(0.3))
            .child(
                v_flex()
                    .flex_1()
                    .gap_2()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child(&field.name)
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().accent)
                                    .child(render_type_ref(&field.type_ref))
                            )
                    )
                    .when_some(field.doc.as_ref(), |this, doc| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(doc)
                        )
                    })
            )
            .child(
                Button::new(format!("remove-field-{}", idx))
                    .small()
                    .label("×")
                    .on_click(cx.listener(move |editor, _, _, cx| {
                        editor.remove_field(idx_for_remove, cx);
                    }))
            )
    }

    fn render_preview_panel(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(400.0))
            .bg(cx.theme().secondary.opacity(0.5))
            .p_4()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
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

    fn generate_preview_code(&self) -> String {
        generate_struct(&self.asset).unwrap_or_else(|e| {
            format!("// Error generating code:\n// {}", e)
        })
    }
}
