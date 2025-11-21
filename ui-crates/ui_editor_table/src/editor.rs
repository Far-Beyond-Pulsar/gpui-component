use gpui::{prelude::*, *};
use ui::{
    h_flex, v_flex, button::{Button, ButtonVariants}, label::Label, divider::Divider,
    table::Table, ActiveTheme, Sizable, Size, StyleSized, StyledExt, Disableable,
    dock::{Panel, PanelEvent, DockChannel}, IconName,
};
use crate::{
    database::DatabaseManager,
    table_view::DataTableView,
    query_editor::QueryEditorView,
    reflection::TypeSchema,
    workspace_panels::{TablePanelWrapper, QueryPanelWrapper, WelcomePanelWrapper},
};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub enum DataTableEvent {
    TableOpened(String),
    TableClosed(usize),
    QueryExecuted(String),
    DataModified { table: String, row_id: i64 },
}

#[derive(Clone, Debug)]
enum TabType {
    Table { name: String, view: Entity<Table<DataTableView>> },
    Query { name: String, view: Entity<QueryEditorView> },
}

struct EditorTab {
    id: usize,
    tab_type: TabType,
}

pub struct DataTableEditor {
    pub db: DatabaseManager,
    available_tables: Vec<String>,
    open_tabs: Vec<EditorTab>,
    active_tab_idx: Option<usize>,
    next_tab_id: usize,
    pub database_path: Option<PathBuf>,
    focus_handle: FocusHandle,
    /// Internal workspace for draggable tabs
    workspace: Option<Entity<ui::workspace::Workspace>>,
    /// Track if workspace has been initialized
    workspace_initialized: bool,
}

impl DataTableEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let db = DatabaseManager::in_memory().expect("Failed to create in-memory database");

        // Create internal workspace for table/query tabs
        let workspace = cx.new(|cx| {
            ui::workspace::Workspace::new_with_channel(
                "table-editor-workspace",
                DockChannel(5), // Unique channel for table editor
                window,
                cx
            )
        });

        Self {
            db,
            available_tables: Vec::new(),
            open_tabs: Vec::new(),
            active_tab_idx: None,
            next_tab_id: 0,
            database_path: None,
            focus_handle: cx.focus_handle(),
            workspace: Some(workspace),
            workspace_initialized: false,
        }
    }

    pub fn open_database(path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> anyhow::Result<Self> {
        let db = DatabaseManager::new(&path)?;
        
        // Auto-discover schemas from existing tables
        db.introspect_and_register_schemas()?;
        
        let available_tables = db.list_tables()?;

        // Create internal workspace for table/query tabs
        let workspace = cx.new(|cx| {
            ui::workspace::Workspace::new_with_channel(
                "table-editor-workspace",
                DockChannel(5), // Unique channel for table editor
                window,
                cx
            )
        });

        Ok(Self {
            db,
            available_tables,
            open_tabs: Vec::new(),
            active_tab_idx: None,
            next_tab_id: 0,
            database_path: Some(path),
            focus_handle: cx.focus_handle(),
            workspace: Some(workspace),
            workspace_initialized: false,
        })
    }

    pub fn register_type_schema(&mut self, schema: TypeSchema) -> anyhow::Result<()> {
        self.db.register_type(schema)?;
        self.available_tables = self.db.list_tables()?;
        Ok(())
    }

    /// Initialize/reinitialize workspace with current tabs
    fn initialize_workspace_once(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace_initialized {
            return;
        }
        
        if let Some(ref workspace) = self.workspace {
            workspace.update(cx, |workspace, cx| {
                let dock_area = workspace.dock_area().downgrade();
                
                if self.open_tabs.is_empty() {
                    // Show welcome panel when no tabs
                    let welcome_panel = cx.new(|cx| {
                        WelcomePanelWrapper::new(cx)
                    });
                    
                    workspace.initialize(
                        ui::dock::DockItem::tabs(
                            vec![std::sync::Arc::new(welcome_panel) as std::sync::Arc<dyn ui::dock::PanelView>],
                            Some(0),
                            &dock_area,
                            window,
                            cx,
                        ),
                        None,
                        None,
                        None,
                        window,
                        cx,
                    );
                } else {
                    // Create panels for all open tabs
                    let tab_panels: Vec<std::sync::Arc<dyn ui::dock::PanelView>> = self.open_tabs
                        .iter()
                        .map(|tab| {
                            match &tab.tab_type {
                                TabType::Table { name, view } => {
                                    let panel = cx.new(|cx| {
                                        TablePanelWrapper::new(name.clone(), view.clone(), cx)
                                    });
                                    std::sync::Arc::new(panel) as std::sync::Arc<dyn ui::dock::PanelView>
                                }
                                TabType::Query { name, view } => {
                                    let panel = cx.new(|cx| {
                                        QueryPanelWrapper::new(name.clone(), view.clone(), cx)
                                    });
                                    std::sync::Arc::new(panel) as std::sync::Arc<dyn ui::dock::PanelView>
                                }
                            }
                        })
                        .collect();
                    
                    workspace.initialize(
                        ui::dock::DockItem::tabs(
                            tab_panels,
                            self.active_tab_idx,
                            &dock_area,
                            window,
                            cx,
                        ),
                        None,
                        None,
                        None,
                        window,
                        cx,
                    );
                }
            });
            
            self.workspace_initialized = true;
        }
    }

    pub fn select_table(&mut self, table_name: String, window: &mut Window, cx: &mut Context<Self>) -> anyhow::Result<()> {
        // Check if table is already open
        if let Some(idx) = self.open_tabs.iter().position(|tab| {
            matches!(&tab.tab_type, TabType::Table { name, .. } if name == &table_name)
        }) {
            self.active_tab_idx = Some(idx);
            cx.notify();
            return Ok(());
        }
        
        // Check if schema exists for this table
        if self.db.get_schema(&table_name).is_none() {
            return Err(anyhow::anyhow!(
                "No schema registered for table '{}'", table_name
            ));
        }
        
        // Create new tab
        let delegate = DataTableView::new(self.db.clone(), table_name.clone())?;
        let table_view = cx.new(|cx| Table::new(delegate, window, cx));
        
        let tab_type = TabType::Table { 
            name: table_name.clone(), 
            view: table_view 
        };
        
        let tab = EditorTab {
            id: self.next_tab_id,
            tab_type: tab_type.clone(),
        };
        
        self.next_tab_id += 1;
        self.open_tabs.push(tab);
        self.active_tab_idx = Some(self.open_tabs.len() - 1);
        
        // Force workspace reinit on next render to pick up new tab
        self.workspace_initialized = false;
        
        cx.emit(DataTableEvent::TableOpened(table_name));
        cx.notify();
        
        Ok(())
    }
    
    pub fn open_query_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query_view = cx.new(|cx| QueryEditorView::new(self.db.clone(), window, cx));
        
        let tab_type = TabType::Query {
            name: format!("Query {}", self.next_tab_id),
            view: query_view,
        };
        
        let tab = EditorTab {
            id: self.next_tab_id,
            tab_type: tab_type.clone(),
        };
        
        self.next_tab_id += 1;
        self.open_tabs.push(tab);
        self.active_tab_idx = Some(self.open_tabs.len() - 1);
        
        // Force workspace reinit on next render to pick up new tab
        self.workspace_initialized = false;
        
        cx.notify();
    }
    
    pub fn close_tab(&mut self, tab_idx: usize, cx: &mut Context<Self>) {
        if tab_idx < self.open_tabs.len() {
            let tab = self.open_tabs.remove(tab_idx);
            cx.emit(DataTableEvent::TableClosed(tab.id));
            
            // Adjust active tab
            if self.open_tabs.is_empty() {
                self.active_tab_idx = None;
            } else if let Some(active) = self.active_tab_idx {
                if active >= self.open_tabs.len() {
                    self.active_tab_idx = Some(self.open_tabs.len() - 1);
                }
            }
            cx.notify();
        }
    }

    pub fn add_new_row(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        if let Some(active_idx) = self.active_tab_idx {
            if let Some(tab) = self.open_tabs.get(active_idx) {
                if let TabType::Table { view, name, .. } = &tab.tab_type {
                    view.update(cx, |table, cx| {
                        if let Err(e) = table.delegate_mut().add_new_row() {
                            eprintln!("Failed to add row: {}", e);
                        } else {
                            cx.notify();
                        }
                    });
                }
            }
        }
        Ok(())
    }

    pub fn delete_selected_row(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        if let Some(active_idx) = self.active_tab_idx {
            if let Some(tab) = self.open_tabs.get(active_idx) {
                if let TabType::Table { view, name, .. } = &tab.tab_type {
                    view.update(cx, |table, cx| {
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
            }
        }
        Ok(())
    }

    pub fn refresh_data(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        if let Some(active_idx) = self.active_tab_idx {
            if let Some(tab) = self.open_tabs.get(active_idx) {
                if let TabType::Table { view, .. } = &tab.tab_type {
                    view.update(cx, |table, cx| {
                        if let Err(e) = table.delegate_mut().refresh_rows(0, 100) {
                            eprintln!("Failed to refresh rows: {}", e);
                        }
                        cx.notify();
                    });
                }
            }
        }
        Ok(())
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let is_table_tab = self.active_tab_idx.and_then(|idx| {
            self.open_tabs.get(idx).map(|tab| matches!(tab.tab_type, TabType::Table { .. }))
        }).unwrap_or(false);
        
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
                    .disabled(!is_table_tab)
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
                    .disabled(!is_table_tab)
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
                    .disabled(!is_table_tab)
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
                        let is_open = self.open_tabs.iter().any(|tab| {
                            matches!(&tab.tab_type, TabType::Table { name, .. } if name == table)
                        });
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
                            .when(is_open, |this| {
                                this.bg(cx.theme().accent.opacity(0.3))
                            })
                            .when(!is_open, |this| {
                                this.hover(|this| this.bg(cx.theme().muted))
                            })
                            .child(table.clone())
                    }))
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

impl Render for DataTableEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Initialize workspace on first render
        self.initialize_workspace_once(window, cx);
        
        let toolbar = self.render_toolbar(cx);
        let sidebar = self.render_sidebar(cx);
        
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
                        div()
                            .flex_1()
                            .h_full()
                            .when_some(self.workspace.clone(), |this, workspace| {
                                this.child(workspace)
                            })
                    )
            )
    }
}

pub fn create_data_table_editor(window: &mut Window, cx: &mut App) -> Entity<DataTableEditor> {
    cx.new(|cx| DataTableEditor::new(window, cx))
}

pub fn create_data_table_editor_with_db(
    path: PathBuf,
    window: &mut Window,
    cx: &mut App,
) -> anyhow::Result<Entity<DataTableEditor>> {
    Ok(cx.new(|cx| DataTableEditor::open_database(path, window, cx).unwrap()))
}
