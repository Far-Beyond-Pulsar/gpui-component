use gpui::*;
use ui::{
    div, h_flex, v_flex, button::Button, label::Label, divider::Divider,
    table::Table, ActiveTheme, Sizable, Size, StyleSized, StyledExt,
};
use crate::{
    database::DatabaseManager,
    table_view::DataTableView,
    query_editor::QueryEditorView,
    reflection::TypeSchema,
};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EditorTab {
    TableData,
    QueryEditor,
}

pub struct DataTableEditor {
    db: DatabaseManager,
    current_table: Option<String>,
    available_tables: Vec<String>,
    current_tab: EditorTab,
    table_view: Option<Entity<Table<DataTableView>>>,
    query_editor: Option<Entity<QueryEditorView>>,
    database_path: Option<PathBuf>,
}

impl DataTableEditor {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let db = DatabaseManager::in_memory().expect("Failed to create in-memory database");

        Self {
            db,
            current_table: None,
            available_tables: Vec::new(),
            current_tab: EditorTab::TableData,
            table_view: None,
            query_editor: None,
            database_path: None,
        }
    }

    pub fn open_database(path: PathBuf, _cx: &mut ViewContext<Self>) -> anyhow::Result<Self> {
        let db = DatabaseManager::new(&path)?;
        let available_tables = db.list_tables()?;

        Ok(Self {
            db,
            current_table: None,
            available_tables,
            current_tab: EditorTab::TableData,
            table_view: None,
            query_editor: None,
            database_path: Some(path),
        })
    }

    pub fn register_type_schema(&mut self, schema: TypeSchema) -> anyhow::Result<()> {
        self.db.register_type(schema)?;
        self.available_tables = self.db.list_tables()?;
        Ok(())
    }

    pub fn select_table(&mut self, table_name: String, window: &mut Window, cx: &mut ViewContext<Self>) -> anyhow::Result<()> {
        self.current_table = Some(table_name.clone());

        let delegate = DataTableView::new(self.db.clone(), table_name)?;
        self.table_view = Some(cx.new_view(|cx| Table::new(delegate, window, cx)));

        Ok(())
    }

    pub fn add_new_row(&mut self, cx: &mut ViewContext<Self>) -> anyhow::Result<()> {
        if let Some(table_view) = &self.table_view {
            table_view.update(cx, |table, _| {
                if let Err(e) = table.delegate_mut().add_new_row() {
                    eprintln!("Failed to add row: {}", e);
                }
            });
        }
        Ok(())
    }

    pub fn delete_selected_row(&mut self, cx: &mut ViewContext<Self>) -> anyhow::Result<()> {
        if let Some(table_view) = &self.table_view {
            table_view.update(cx, |table, _| {
                if let Err(e) = table.delegate_mut().delete_row(0) {
                    eprintln!("Failed to delete row: {}", e);
                }
            });
        }
        Ok(())
    }

    pub fn refresh_data(&mut self, cx: &mut ViewContext<Self>) -> anyhow::Result<()> {
        if let Some(table_view) = &self.table_view {
            table_view.update(cx, |table, cx| {
                if let Err(e) = table.delegate_mut().refresh_rows(0, 100) {
                    eprintln!("Failed to refresh rows: {}", e);
                }
                cx.notify();
            });
        }
        Ok(())
    }

    fn render_toolbar(&self, cx: &ViewContext<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .gap_2()
            .p_2()
            .bg(cx.theme().muted.opacity(0.3))
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("add-row")
                    .label("Add Row")
                    .small()
                    .primary()
                    .disabled(self.current_table.is_none())
            )
            .child(
                Button::new("delete-row")
                    .label("Delete Row")
                    .small()
                    .outline()
                    .disabled(self.current_table.is_none())
            )
            .child(Divider::vertical().h_6())
            .child(
                Button::new("refresh")
                    .label("Refresh")
                    .small()
                    .outline()
                    .disabled(self.current_table.is_none())
            )
            .child(Divider::vertical().h_6())
            .when_some(self.current_table.as_ref(), |this, table_name| {
                this.child(
                    Label::new(format!("Table: {}", table_name))
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                )
            })
    }

    fn render_sidebar(&self, cx: &ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .w_64()
            .h_full()
            .bg(cx.theme().muted.opacity(0.2))
            .border_r_1()
            .border_color(cx.theme().border)
            .gap_2()
            .p_2()
            .child(
                Label::new("Tables")
                    .text_sm()
                    .font_semibold()
                    .px_2()
            )
            .child(Divider::horizontal())
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .overflow_y_scroll()
                    .children(self.available_tables.iter().map(|table| {
                        let is_selected = self.current_table.as_ref() == Some(table);
                        div()
                            .w_full()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .text_sm()
                            .when(is_selected, |this| {
                                this.bg(cx.theme().accent)
                                    .text_color(cx.theme().accent_foreground)
                            })
                            .when(!is_selected, |this| {
                                this.hover(|this| this.bg(cx.theme().muted))
                            })
                            .child(table.clone())
                    }))
            )
    }

    fn render_tabs(&self, cx: &ViewContext<Self>) -> impl IntoElement {
        h_flex()
            .gap_1()
            .p_2()
            .bg(cx.theme().muted.opacity(0.2))
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("tab-data")
                    .label("Data")
                    .small()
                    .when(self.current_tab == EditorTab::TableData, |this| this.primary())
                    .when(self.current_tab != EditorTab::TableData, |this| this.ghost())
            )
            .child(
                Button::new("tab-query")
                    .label("Query Editor")
                    .small()
                    .when(self.current_tab == EditorTab::QueryEditor, |this| this.primary())
                    .when(self.current_tab != EditorTab::QueryEditor, |this| this.ghost())
            )
    }

    fn render_content(&self, cx: &ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .w_full()
            .when(self.current_tab == EditorTab::TableData, |this| {
                this.when_some(self.table_view.as_ref(), |this, table_view| {
                    this.child(table_view.clone())
                })
                .when(self.table_view.is_none(), |this| {
                    this.child(
                        v_flex()
                            .size_full()
                            .items_center()
                            .justify_center()
                            .child(
                                Label::new("Select a table to view data")
                                    .text_color(cx.theme().muted_foreground)
                            )
                    )
                })
            })
            .when(self.current_tab == EditorTab::QueryEditor, |this| {
                this.when_some(self.query_editor.as_ref(), |this, query_editor| {
                    this.child(query_editor.clone())
                })
                .when(self.query_editor.is_none(), |this| {
                    this.child(
                        v_flex()
                            .size_full()
                            .items_center()
                            .justify_center()
                            .child(
                                Label::new("Query editor not initialized")
                                    .text_color(cx.theme().muted_foreground)
                            )
                    )
                })
            })
    }
}

impl Render for DataTableEditor {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .w_full()
                    .child(self.render_sidebar(cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .h_full()
                            .child(self.render_tabs(cx))
                            .child(self.render_content(cx))
                    )
            )
    }
}

pub fn create_data_table_editor(cx: &mut App) -> Entity<DataTableEditor> {
    cx.new(|cx| DataTableEditor::new(cx))
}

pub fn create_data_table_editor_with_db(
    path: PathBuf,
    cx: &mut App,
) -> anyhow::Result<Entity<DataTableEditor>> {
    Ok(cx.new(|cx| DataTableEditor::open_database(path, cx).unwrap()))
}
