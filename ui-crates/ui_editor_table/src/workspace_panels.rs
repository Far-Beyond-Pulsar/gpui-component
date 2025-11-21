use gpui::*;
use ui::{ActiveTheme, StyledExt, dock::{Panel, PanelEvent}, v_flex, table::Table};
use std::path::PathBuf;
use crate::{
    table_view::DataTableView,
    query_editor::QueryEditorView,
    database::DatabaseManager,
};

/// Table Panel - wraps a single table view
pub struct TablePanelWrapper {
    table_name: String,
    table_view: Entity<Table<DataTableView>>,
    focus_handle: FocusHandle,
}

impl TablePanelWrapper {
    pub fn new(
        table_name: String,
        table_view: Entity<Table<DataTableView>>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            table_name,
            table_view,
            focus_handle: cx.focus_handle(),
        }
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
        "table"
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

/// Query Panel - wraps a query editor view
pub struct QueryPanelWrapper {
    query_name: String,
    query_view: Entity<QueryEditorView>,
    focus_handle: FocusHandle,
}

impl QueryPanelWrapper {
    pub fn new(
        query_name: String,
        query_view: Entity<QueryEditorView>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            query_name,
            query_view,
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
        "query"
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

/// Welcome Panel - shown when no tables/queries are open
pub struct WelcomePanelWrapper {
    focus_handle: FocusHandle,
}

impl WelcomePanelWrapper {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl EventEmitter<PanelEvent> for WelcomePanelWrapper {}

impl Render for WelcomePanelWrapper {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                div()
                    .text_xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(cx.theme().foreground)
                    .child("Welcome to Database Editor")
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Select a table from the sidebar or create a new query")
            )
    }
}

impl Focusable for WelcomePanelWrapper {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for WelcomePanelWrapper {
    fn panel_name(&self) -> &'static str {
        "welcome"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        "Welcome".into_any_element()
    }

    fn dump(&self, _cx: &App) -> ui::dock::PanelState {
        ui::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}
