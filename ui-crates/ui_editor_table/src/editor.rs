use gpui::{prelude::*, *};
use ui::{
    h_flex, v_flex, button::{Button, ButtonVariants}, label::Label, divider::Divider,
    table::Table, ActiveTheme, Sizable, Size, StyleSized, StyledExt, Disableable,
    dock::{Panel, PanelEvent},
};
use crate::{
    database::DatabaseManager,
    table_view::DataTableView,
    query_editor::QueryEditorView,
    reflection::TypeSchema,
};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EditorTab {
    TableData,
    QueryEditor,
}

pub struct DataTableEditor {
    pub db: DatabaseManager,
    current_table: Option<String>,
    available_tables: Vec<String>,
    current_tab: EditorTab,
    table_view: Option<Entity<Table<DataTableView>>>,
    query_editor: Option<Entity<QueryEditorView>>,
    pub database_path: Option<PathBuf>,
    focus_handle: FocusHandle,
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
        eprintln!("Available tables after registration: {:?}", self.available_tables);
        Ok(())
    }

    pub fn select_table(&mut self, table_name: String, window: &mut Window, cx: &mut Context<Self>) -> anyhow::Result<()> {
        eprintln!("Selecting table: {}", table_name);
        eprintln!("Available tables: {:?}", self.available_tables);
        eprintln!("All schemas: {:?}", self.db.list_tables()?);
        
        // Check if schema exists for this table
        if self.db.get_schema(&table_name).is_none() {
            return Err(anyhow::anyhow!(
                "No schema registered for table '{}'. \n\
                 The database has a table named '{}' but no TypeSchema was registered for it.\n\
                 Either:\n\
                 1. Register a schema: editor.register_type_schema(TypeSchema::new(\"{}\"))\n\
                 2. Delete the old database file and recreate it with proper schemas\n\
                 3. The table was created manually and needs a schema definition",
                table_name, table_name, table_name
            ));
        }
        
        self.current_table = Some(table_name.clone());

        let delegate = DataTableView::new(self.db.clone(), table_name.clone())?;
        eprintln!("Created delegate for table: {}", table_name);
        self.table_view = Some(cx.new(|cx| Table::new(delegate, window, cx)));
        eprintln!("Table view created");

        Ok(())
    }

    pub fn add_new_row(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        if let Some(table_view) = &self.table_view {
            table_view.update(cx, |table, _| {
                if let Err(e) = table.delegate_mut().add_new_row() {
                    eprintln!("Failed to add row: {}", e);
                }
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
                    .disabled(self.current_table.is_none())
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
                    .disabled(self.current_table.is_none())
                    .on_click(cx.listener(|editor, _, _, cx| {
                        if let Err(e) = editor.refresh_data(cx) {
                            eprintln!("Failed to refresh: {}", e);
                        }
                        cx.notify();
                    }))
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
                                eprintln!(">>> CLICK HANDLER FIRED for table: {}", table_name);
                                match editor.select_table(table_name.clone(), window, cx) {
                                    Ok(_) => eprintln!(">>> select_table succeeded"),
                                    Err(e) => eprintln!(">>> select_table FAILED: {}", e),
                                }
                                cx.notify();
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
                    .on_click(cx.listener(|editor, _, _, cx| {
                        editor.current_tab = EditorTab::QueryEditor;
                        if editor.query_editor.is_none() {
                            editor.query_editor = Some(cx.new(|cx| {
                                QueryEditorView::new(editor.db.clone(), cx)
                            }));
                        }
                        cx.notify();
                    }))
            )
    }

    fn render_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        eprintln!("Rendering content - tab: {:?}, table_view is some: {}, current_table: {:?}", 
                  self.current_tab, self.table_view.is_some(), self.current_table);
        
        match self.current_tab {
            EditorTab::TableData => {
                if let Some(ref table_view) = self.table_view {
                    eprintln!("Rendering table view");
                    div()
                        .flex_1()
                        .w_full()
                        .size_full()
                        .child(table_view.clone())
                        .into_any_element()
                } else {
                    eprintln!("No table view, showing message");
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

impl Render for DataTableEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let toolbar = self.render_toolbar(cx);
        let sidebar = self.render_sidebar(cx);
        let tabs = self.render_tabs(cx);
        let content = self.render_content(cx);
        
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(toolbar)
            .child(
                h_flex()
                    .flex_1()
                    .w_full()
                    .child(sidebar)
                    .child(
                        v_flex()
                            .flex_1()
                            .h_full()
                            .child(tabs)
                            .child(content)
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
