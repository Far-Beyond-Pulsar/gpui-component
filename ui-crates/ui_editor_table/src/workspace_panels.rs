//! Workspace panels for Data Table Editor
//! Each table and query gets its own dockable panel

use gpui::*;
use ui::{
    ActiveTheme, StyledExt, dock::{Panel, PanelEvent}, v_flex, table::Table,
};
use crate::{
    database::DatabaseManager,
    table_view::DataTableView,
    query_editor::QueryEditorView,
};

/// Individual Table Panel - wraps a single table view
pub struct TablePanelWrapper {
    table_name: String,
    pub table_view: Entity<Table<DataTableView>>,
    db: DatabaseManager,
    focus_handle: FocusHandle,
}

impl TablePanelWrapper {
    pub fn new(
        table_name: String,
        table_view: Entity<Table<DataTableView>>,
        db: DatabaseManager,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            table_name,
            table_view,
            db,
            focus_handle: cx.focus_handle(),
        }
    }
    
    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

impl EventEmitter<PanelEvent> for TablePanelWrapper {}

impl Render for TablePanelWrapper {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.table_view.clone())
    }
}

impl Focusable for TablePanelWrapper {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for TablePanelWrapper {
    fn panel_name(&self) -> &'static str {
        "data_table"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        self.table_name.clone().into_any_element()
    }
    
    fn dump(&self, _cx: &App) -> ui::dock::PanelState {
        ui::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

/// Query Panel - wraps a query editor
pub struct QueryPanelWrapper {
    query_name: String,
    query_view: Entity<QueryEditorView>,
    db: DatabaseManager,
    focus_handle: FocusHandle,
}

impl QueryPanelWrapper {
    pub fn new(
        query_name: String,
        query_view: Entity<QueryEditorView>,
        db: DatabaseManager,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            query_name,
            query_view,
            db,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl EventEmitter<PanelEvent> for QueryPanelWrapper {}

impl Render for QueryPanelWrapper {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.query_view.clone())
    }
}

impl Focusable for QueryPanelWrapper {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for QueryPanelWrapper {
    fn panel_name(&self) -> &'static str {
        "data_query"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        self.query_name.clone().into_any_element()
    }
    
    fn dump(&self, _cx: &App) -> ui::dock::PanelState {
        ui::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

/// Main Database Browser Panel - the sidebar with table list
pub struct DatabaseBrowserPanel {
    db: DatabaseManager,
    available_tables: Vec<String>,
    focus_handle: FocusHandle,
    on_table_selected: Option<Box<dyn Fn(String, &mut Window, &mut Context<Self>)>>,
}

impl DatabaseBrowserPanel {
    pub fn new(
        db: DatabaseManager,
        available_tables: Vec<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            db,
            available_tables,
            focus_handle: cx.focus_handle(),
            on_table_selected: None,
        }
    }
    
    pub fn on_table_selected(
        mut self,
        callback: impl Fn(String, &mut Window, &mut Context<Self>) + 'static,
    ) -> Self {
        self.on_table_selected = Some(Box::new(callback));
        self
    }
    
    pub fn refresh_tables(&mut self) -> anyhow::Result<()> {
        self.available_tables = self.db.list_tables()?;
        Ok(())
    }
}

impl EventEmitter<PanelEvent> for DatabaseBrowserPanel {}

impl Render for DatabaseBrowserPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .p_2()
            .bg(cx.theme().muted.opacity(0.2))
            .child(
                ui::label::Label::new("Tables")
                    .text_sm()
                    .font_semibold()
                    .px_2()
            )
            .child(ui::divider::Divider::horizontal())
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .children(self.available_tables.iter().enumerate().map(|(idx, table)| {
                        let table_name = table.clone();
                        div()
                            .id(("browser-table", idx))
                            .w_full()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .text_sm()
                            .cursor_pointer()
                            .on_click(cx.listener(move |panel, _, window, cx| {
                                if let Some(ref callback) = panel.on_table_selected {
                                    callback(table_name.clone(), window, cx);
                                }
                            }))
                            .hover(|this| this.bg(cx.theme().muted))
                            .child(table.clone())
                    }))
            )
    }
}

impl Focusable for DatabaseBrowserPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for DatabaseBrowserPanel {
    fn panel_name(&self) -> &'static str {
        "database_browser"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        "Database Browser".into_any_element()
    }
    
    fn dump(&self, _cx: &App) -> ui::dock::PanelState {
        ui::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}
