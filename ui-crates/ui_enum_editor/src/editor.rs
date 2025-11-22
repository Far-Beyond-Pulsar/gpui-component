use gpui::{prelude::*, *};
use ui::{h_flex, v_flex, button::{Button, ButtonVariants}, divider::Divider, ActiveTheme, StyledExt};
use ui_types_common::*;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum EnumEditorEvent {
    EnumSaved(String),
    EnumClosed,
}

pub struct EnumEditor {
    asset: EnumAsset,
    file_path: Option<PathBuf>,
    project_root: PathBuf,
    is_dirty: bool,
    index_manager: IndexManager,
    type_index: TypeIndex,
    focus_handle: FocusHandle,
}

impl EnumEditor {
    pub fn new(project_root: PathBuf, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let index_manager = IndexManager::new(project_root.clone());
        let type_index = index_manager.load_index().unwrap_or_default();

        let asset = EnumAsset {
            schema_version: 1,
            type_kind: TypeKind::Enum,
            name: "new_enum".to_string(),
            display_name: "NewEnum".to_string(),
            description: None,
            variants: vec![],
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
        }
    }

    pub fn open(file_path: PathBuf, project_root: PathBuf, _window: &mut Window, cx: &mut Context<Self>) -> anyhow::Result<Self> {
        let index_manager = IndexManager::new(project_root.clone());
        let type_index = index_manager.load_index().unwrap_or_default();

        let json_content = std::fs::read_to_string(&file_path)?;
        let asset: EnumAsset = serde_json::from_str(&json_content)?;

        Ok(Self {
            asset,
            file_path: Some(file_path),
            project_root,
            is_dirty: false,
            index_manager,
            type_index,
            focus_handle: cx.focus_handle(),
        })
    }

    pub fn add_variant(&mut self, cx: &mut Context<Self>) {
        let variant_num = self.asset.variants.len() + 1;
        let new_variant = EnumVariant {
            name: format!("Variant{}", variant_num),
            payload: None,
            doc: None,
        };

        self.asset.variants.push(new_variant);
        self.is_dirty = true;
        cx.notify();
    }

    pub fn save(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        validate_enum(&self.asset, &self.type_index)?;

        let file_path = if let Some(path) = &self.file_path {
            path.clone()
        } else {
            self.index_manager.ensure_type_dir(TypeKind::Enum, &self.asset.name)?;
            self.index_manager.get_json_path(TypeKind::Enum, &self.asset.name)
        };

        let json = serde_json::to_string_pretty(&self.asset)?;
        std::fs::write(&file_path, json)?;

        let rust_code = generate_enum(&self.asset)?;
        let rs_path = self.index_manager.get_rs_path(TypeKind::Enum, &self.asset.name);
        std::fs::write(&rs_path, rust_code)?;

        let mut index = self.index_manager.load_index().unwrap_or_default();
        let entry = TypeIndexEntry::new(TypeKind::Enum, self.asset.name.clone(), self.asset.display_name.clone());
        index.upsert(TypeKind::Enum, entry)?;
        self.index_manager.save_index(&mut index)?;

        self.is_dirty = false;
        self.file_path = Some(file_path);
        self.type_index = index;

        cx.emit(EnumEditorEvent::EnumSaved(self.asset.name.clone()));
        cx.notify();

        Ok(())
    }
}

impl EventEmitter<EnumEditorEvent> for EnumEditor {}

impl Focusable for EnumEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EnumEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
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
                            .child(div().text_lg().child("🎯"))
                            .child(
                                div()
                                    .text_sm()
                                    .font_bold()
                                    .text_color(cx.theme().foreground)
                                    .child(&self.asset.display_name)
                            )
                    )
                    .child(
                        Button::new("save")
                            .primary()
                            .label("Save")
                            .disabled(!self.is_dirty)
                            .on_click(cx.listener(|editor, _, _, cx| {
                                if let Err(e) = editor.save(cx) {
                                    eprintln!("Failed to save enum: {}", e);
                                }
                            }))
                    )
            )
            .child(Divider::horizontal())
            .child(
                v_flex()
                    .flex_1()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(format!("Enum Editor: {}", self.asset.name))
                    )
            )
    }
}
