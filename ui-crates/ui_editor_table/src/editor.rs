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

    /// Add pending tabs to the first available TabPanel
    fn add_pending_tabs_to_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open_tabs.is_empty() || !self.workspace_initialized {
            return;
        }
        
        // Find the first TabPanel in the workspace and add new tabs to it
        if let Some(workspace) = self.workspace.clone() {
            let num_existing_panels = self.open_tabs.len() - 1; // All but the last one
            
            // Get the last tab that was just added
            if let Some(last_tab) = self.open_tabs.last() {
                let panel: std::sync::Arc<dyn ui::dock::PanelView> = match &last_tab.tab_type {
                    TabType::Table { name, view } => {
                        let panel = cx.new(|cx| {
                            TablePanelWrapper::new(name.clone(), view.clone(), cx)
                        });
                        std::sync::Arc::new(panel)
                    }
                    TabType::Query { name, view } => {
                        let panel = cx.new(|cx| {
                            QueryPanelWrapper::new(name.clone(), view.clone(), cx)
                        });
                        std::sync::Arc::new(panel)
                    }
                };
                
                // Defer adding the panel to avoid reentrant updates
                window.defer(cx, move |window, cx| {
                    _ = workspace.update(cx, |workspace, cx| {
                        let dock_area = workspace.dock_area();
                        
                        // Get the first TabPanel from the center items
                        if let Some(tab_panel) = dock_area.read(cx).items().left_top_tab_panel(cx) {
                            _ = tab_panel.update(cx, |tab_panel, cx| {
                                tab_panel.add_panel(panel, window, cx);
                            });
                        }
                    });
                });
            }
        }
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
        
        // Add the new tab to the workspace efficiently
        self.add_pending_tabs_to_workspace(window, cx);
        
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
        
        // Add the new tab to the workspace efficiently
        self.add_pending_tabs_to_workspace(window, cx);
        
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
                        let delegate = table.delegate_mut();
                        let page_size = delegate.state.page_size;
                        if let Err(e) = delegate.refresh_rows(0, page_size) {
                            eprintln!("Failed to refresh rows: {}", e);
                        }
                        cx.notify();
                    });
                }
            }
        }
        Ok(())
    }

    pub fn duplicate_selected_row(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        if let Some(active_idx) = self.active_tab_idx {
            if let Some(tab) = self.open_tabs.get(active_idx) {
                if let TabType::Table { view, .. } = &tab.tab_type {
                    view.update(cx, |table, cx| {
                        let delegate = table.delegate_mut();
                        if let Some(selected_row) = delegate.state.selected_row {
                            if let Err(e) = delegate.duplicate_row(selected_row) {
                                eprintln!("Failed to duplicate row: {}", e);
                            } else {
                                println!("✓ Row duplicated successfully");
                            }
                        }
                        cx.notify();
                    });
                }
            }
        }
        Ok(())
    }

    pub fn copy_row_as_sql(&mut self, cx: &mut Context<Self>) {
        if let Some(active_idx) = self.active_tab_idx {
            if let Some(tab) = self.open_tabs.get(active_idx) {
                if let TabType::Table { view, .. } = &tab.tab_type {
                    view.update(cx, |table, cx| {
                        let delegate = table.delegate();
                        if let Some(selected_row) = delegate.state.selected_row {
                            if let Some(sql) = delegate.copy_row_as_insert(selected_row) {
                                println!("✓ Copied SQL: {}", sql);
                                // TODO: Copy to clipboard when available
                            }
                        }
                        cx.notify();
                    });
                }
            }
        }
    }

    pub fn get_table_stats(&self, cx: &App) -> String {
        if let Some(active_idx) = self.active_tab_idx {
            if let Some(tab) = self.open_tabs.get(active_idx) {
                if let TabType::Table { view, .. } = &tab.tab_type {
                    return view.read(cx).delegate().get_table_stats();
                }
            }
        }
        "No table selected".to_string()
    }

    pub fn next_page(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        if let Some(active_idx) = self.active_tab_idx {
            if let Some(tab) = self.open_tabs.get(active_idx) {
                if let TabType::Table { view, .. } = &tab.tab_type {
                    view.update(cx, |table, cx| {
                        if let Err(e) = table.delegate_mut().next_page() {
                            eprintln!("Failed to go to next page: {}", e);
                        }
                        cx.notify();
                    });
                }
            }
        }
        Ok(())
    }

    pub fn previous_page(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        if let Some(active_idx) = self.active_tab_idx {
            if let Some(tab) = self.open_tabs.get(active_idx) {
                if let TabType::Table { view, .. } = &tab.tab_type {
                    view.update(cx, |table, cx| {
                        if let Err(e) = table.delegate_mut().previous_page() {
                            eprintln!("Failed to go to previous page: {}", e);
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

        v_flex()
            .w_full()
            .gap_0()
            .child(
                // Main toolbar
                h_flex()
                    .w_full()
                    .gap_2()
                    .p_2()
                    .bg(cx.theme().muted.opacity(0.3))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("add-row")
                            .icon(IconName::Plus)
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
                        Button::new("duplicate-row")
                            .icon(IconName::Copy)
                            .label("Duplicate")
                            .tooltip("Duplicate selected row")
                            .small()
                            .outline()
                            .disabled(!is_table_tab)
                            .on_click(cx.listener(|editor, _, _, cx| {
                                if let Err(e) = editor.duplicate_selected_row(cx) {
                                    eprintln!("Failed to duplicate row: {}", e);
                                }
                                cx.notify();
                            }))
                    )
                    .child(
                        Button::new("delete-row")
                            .icon(IconName::Close)
                            .label("Delete")
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
                        Button::new("copy-as-insert")
                            .icon(IconName::Code)
                            .label("Copy as SQL")
                            .tooltip("Copy selected row as INSERT statement")
                            .small()
                            .outline()
                            .disabled(!is_table_tab)
                            .on_click(cx.listener(|editor, _, _, cx| {
                                editor.copy_row_as_sql(cx);
                                cx.notify();
                            }))
                    )
                    .child(Divider::vertical().h_6())
                    .child(
                        Button::new("refresh")
                            .icon(IconName::Refresh)
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
                            .icon(IconName::Code)
                            .label("New Query")
                            .small()
                            .outline()
                            .on_click(cx.listener(|editor, _, window, cx| {
                                editor.open_query_tab(window, cx);
                            }))
                    )
                    .child(
                        div().flex_1() // Spacer
                    )
                    .child(
                        Label::new("💡 Click cell to edit • Enter to save • Esc to cancel")
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                    )
            )
            .when(is_table_tab, |this| {
                this.child(
                    // Status/info bar
                    h_flex()
                        .w_full()
                        .gap_2()
                        .px_2()
                        .py_1()
                        .bg(cx.theme().muted.opacity(0.2))
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            Label::new("📊")
                                .text_sm()
                        )
                        .child(
                            Label::new(self.get_table_stats(cx))
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                        )
                        .child(
                            div().flex_1() // Spacer
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .child(
                                    Button::new("page-prev")
                                        .icon(IconName::ChevronLeft)
                                        .xsmall()
                                        .ghost()
                                        .on_click(cx.listener(|editor, _, _, cx| {
                                            if let Err(e) = editor.previous_page(cx) {
                                                eprintln!("Failed to go to previous page: {}", e);
                                            }
                                            cx.notify();
                                        }))
                                )
                                .child(
                                    Label::new("Page")
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                )
                                .child(
                                    Button::new("page-next")
                                        .icon(IconName::ChevronRight)
                                        .xsmall()
                                        .ghost()
                                        .on_click(cx.listener(|editor, _, _, cx| {
                                            if let Err(e) = editor.next_page(cx) {
                                                eprintln!("Failed to go to next page: {}", e);
                                            }
                                            cx.notify();
                                        }))
                                )
                        )
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
