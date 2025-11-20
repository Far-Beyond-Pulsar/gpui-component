use gpui::{prelude::*, *};
use ui::{
    h_flex, v_flex, button::{Button, ButtonVariants}, label::Label, divider::Divider,
    table::Table, ActiveTheme, Sizable, Size, StyleSized, StyledExt, Disableable,
    dock::{Panel, PanelEvent, DockArea, DockItem, TabPanel}, IconName,
};
use crate::{
    database::DatabaseManager,
    table_view::DataTableView,
    query_editor::QueryEditorView,
    reflection::TypeSchema,
    workspace_panels::{TablePanelWrapper, QueryPanelWrapper, DatabaseBrowserPanel},
};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum DataTableEvent {
    TableOpened(String),
    TableClosed(usize),
    QueryExecuted(String),
    DataModified { table: String, row_id: i64 },
}

pub struct DataTableEditor {
    pub db: DatabaseManager,
    available_tables: Vec<String>,
    current_table: Option<String>,
    current_tab: EditorTab,
    table_view: Option<Entity<Table<DataTableView>>>,
    query_editor: Option<Entity<QueryEditorView>>,
    pub database_path: Option<PathBuf>,
    focus_handle: FocusHandle,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EditorTab {
    TableData,
    QueryEditor,
}

impl DataTableEditor {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let db = DatabaseManager::in_memory().expect("Failed to create in-memory database");

        Self {
            db,
            current_table: None,
            available_tables: Vec::new(),
            current_tab: EditorTab::TableData,
            table_view: None,
            query_editor: None,
            database_path: None,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn open_database(path: PathBuf, cx: &mut Context<Self>) -> anyhow::Result<Self> {
        let db = DatabaseManager::new(&path)?;
        
        // Auto-discover schemas from existing tables
        db.introspect_and_register_schemas()?;
        
        let available_tables = db.list_tables()?;

        Ok(Self {
            db,
            current_table: None,
            available_tables,
            current_tab: EditorTab::TableData,
            table_view: None,
            query_editor: None,
            database_path: Some(path),
            focus_handle: cx.focus_handle(),
        })
    }

    pub fn register_type_schema(&mut self, schema: TypeSchema) -> anyhow::Result<()> {
        self.db.register_type(schema)?;
        self.available_tables = self.db.list_tables()?;
        Ok(())
    }

    pub fn select_table(&mut self, table_name: String, window: &mut Window, cx: &mut Context<Self>) -> anyhow::Result<()> {
        // Check if schema exists for this table
        if self.db.get_schema(&table_name).is_none() {
            return Err(anyhow::anyhow!(
                "No schema registered for table '{}'", table_name
            ));
        }
        
        self.current_table = Some(table_name.clone());
        
        let delegate = DataTableView::new(self.db.clone(), table_name.clone())?;
        self.table_view = Some(cx.new(|cx| Table::new(delegate, window, cx)));
        
        cx.emit(DataTableEvent::TableOpened(table_name));
        cx.notify();
        
        Ok(())
    }
    
    pub fn open_query_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.query_editor.is_none() {
            self.query_editor = Some(cx.new(|cx| QueryEditorView::new(self.db.clone(), window, cx)));
        }
        self.current_tab = EditorTab::QueryEditor;
        cx.notify();
    }

    pub fn add_new_row(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        if let Some(table_view) = &self.table_view {
            table_view.update(cx, |table, cx| {
                if let Err(e) = table.delegate_mut().add_new_row() {
                    eprintln!("Failed to add row: {}", e);
                }
                cx.notify();
            });
        }
        Ok(())
    }

    pub fn delete_selected_row(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        if let Some(table_view) = &self.table_view {
            table_view.update(cx, |table, cx| {
                let delegate = table.delegate_mut();
                if let Some(selected_row) = delegate.state.selected_row {
                    if let Err(e) = delegate.delete_row(selected_row) {
                        eprintln!("Failed to delete row: {}", e);
                    } else {
                        delegate.state.selected_row = None;
                        cx.notify();
                    }
                }
            });
        }
        Ok(())
    }

    pub fn refresh_data(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
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

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_table = self.current_table.is_some();
        
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
                    .disabled(!has_table)
                    .on_click(cx.listener(|editor, _, _, cx| {
                        if let Err(e) = editor.add_new_row(cx) {
                            eprintln!("Failed to add row: {}", e);
                        }
                        cx.notify();
                    }))
            )
            .child(
                Button::new("delete-row")
                    .label("Delete Row")
                    .small()
                    .outline()
                    .disabled(!has_table)
                    .on_click(cx.listener(|editor, _, _, cx| {
                        if let Err(e) = editor.delete_selected_row(cx) {
                            eprintln!("Failed to delete row: {}", e);
                        }
                        cx.notify();
                    }))
            )
            .child(Divider::vertical().h_6())
            .child(
                Button::new("refresh")
                    .label("Refresh")
                    .small()
                    .outline()
                    .disabled(!has_table)
                    .on_click(cx.listener(|editor, _, _, cx| {
                        if let Err(e) = editor.refresh_data(cx) {
                            eprintln!("Failed to refresh: {}", e);
                        }
                        cx.notify();
                    }))
            )
            .child(Divider::vertical().h_6())
            .child(
                Button::new("new-query")
                    .label("New Query")
                    .small()
                    .outline()
                    .on_click(cx.listener(|editor, _, window, cx| {
                        editor.open_query_tab(window, cx);
                    }))
            )
    }

}

impl Render for DataTableEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .child(self.render_sidebar(cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .child(self.render_tabs(cx))
                            .child(self.render_content(cx))
                    )
            )
    }
}

impl Panel for DataTableEditor {
    fn panel_name(&self) -> &'static str {
        "Database Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        let title = if let Some(path) = &self.database_path {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Database")
                .to_string()
        } else {
            "Database".to_string()
        };

        div()
            .child(title)
            .into_any_element()
    }

    fn dump(&self, _cx: &App) -> ui::dock::PanelState {
        ui::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for DataTableEditor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for DataTableEditor {}
impl EventEmitter<DataTableEvent> for DataTableEditor {}

impl DataTableEditor {
    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .children(self.available_tables.iter().enumerate().map(|(idx, table)| {
                        let is_selected = self.current_table.as_ref() == Some(table);
                        let table_name = table.clone();
                        div()
                            .id(("table-item", idx))
                            .w_full()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .on_click(cx.listener(move |editor, _, window, cx| {
                                if let Err(e) = editor.select_table(table_name.clone(), window, cx) {
                                    eprintln!("Failed to select table: {}", e);
                                }
                            }))
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
    
    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .on_click(cx.listener(|editor, _, _, cx| {
                        editor.current_tab = EditorTab::TableData;
                        cx.notify();
                    }))
            )
            .child(
                Button::new("tab-query")
                    .label("Query Editor")
                    .small()
                    .when(self.current_tab == EditorTab::QueryEditor, |this| this.primary())
                    .when(self.current_tab != EditorTab::QueryEditor, |this| this.ghost())
                    .on_click(cx.listener(|editor, _, window, cx| {
                        editor.open_query_tab(window, cx);
                    }))
            )
    }
    
    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.current_tab {
            EditorTab::TableData => {
                if let Some(ref table_view) = self.table_view {
                    div()
                        .flex_1()
                        .w_full()
                        .size_full()
                        .child(table_view.clone())
                        .into_any_element()
                } else {
                    v_flex()
                        .flex_1()
                        .w_full()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .child(
                            Label::new("Select a table to view data")
                                .text_color(cx.theme().muted_foreground)
                        )
                        .into_any_element()
                }
            }
            EditorTab::QueryEditor => {
                if let Some(ref query_editor) = self.query_editor {
                    div()
                        .flex_1()
                        .w_full()
                        .child(query_editor.clone())
                        .into_any_element()
                } else {
                    v_flex()
                        .flex_1()
                        .w_full()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .child(
                            Label::new("Query editor not initialized")
                                .text_color(cx.theme().muted_foreground)
                        )
                        .into_any_element()
                }
            }
        }
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
