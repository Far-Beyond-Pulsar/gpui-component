/// Problems Drawer - Displays diagnostics, errors, and warnings from rust-analyzer
use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants as _, ButtonVariant},
    h_flex, v_flex, ActiveTheme as _, IconName, StyledExt, Sizable as _,
    list::{List, ListDelegate},
    IndexPath, Selectable,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Clone, Debug)]
pub struct NavigateToDiagnostic {
    pub file_path: PathBuf,
    pub line: usize,
    pub column: usize,
}

impl EventEmitter<NavigateToDiagnostic> for ProblemsDrawer {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticSeverity {
    pub fn icon(&self) -> IconName {
        match self {
            Self::Error => IconName::Close,
            Self::Warning => IconName::TriangleAlert,
            Self::Information => IconName::Info,
            Self::Hint => IconName::Info,
        }
    }

    pub fn color(&self, cx: &App) -> Hsla {
        match self {
            Self::Error => Hsla { h: 0.0, s: 0.8, l: 0.5, a: 1.0 }, // Red
            Self::Warning => Hsla { h: 40.0, s: 0.9, l: 0.5, a: 1.0 }, // Orange
            Self::Information => cx.theme().accent, // Blue
            Self::Hint => cx.theme().muted_foreground, // Gray
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::Warning => "Warning",
            Self::Information => "Info",
            Self::Hint => "Hint",
        }
    }
}

#[derive(Clone, IntoElement)]
struct DiagnosticItem {
    diagnostic: Diagnostic,
    selected: bool,
    on_click: Option<Arc<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl DiagnosticItem {
    fn new(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostic,
            selected: false,
            on_click: None,
        }
    }

    fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Arc::new(handler));
        self
    }
}

impl Selectable for DiagnosticItem {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for DiagnosticItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let diagnostic = self.diagnostic.clone();
        let on_click = self.on_click.clone();
        
        div()
            .w_full()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .when(self.selected, |this| this.bg(cx.theme().selection))
            .hover(|this| this.bg(cx.theme().secondary))
            .cursor_pointer()
            .on_mouse_down(gpui::MouseButton::Left, move |_event, window, cx| {
                if let Some(handler) = &on_click {
                    handler(window, cx);
                }
            })
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .child(
                                        gpui_component::Icon::new(diagnostic.severity.icon())
                                            .size_4()
                                            .text_color(diagnostic.severity.color(cx))
                                    )
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(diagnostic.severity.color(cx))
                                    .child(diagnostic.severity.label())
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .line_clamp(1)
                                    .child(format!(
                                        "{}:{}:{}",
                                        diagnostic.file_path,
                                        diagnostic.line,
                                        diagnostic.column
                                    ))
                            )
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(diagnostic.message.clone())
                    )
                    .when_some(diagnostic.source.as_ref(), |this, source| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("Source: {}", source))
                        )
                    })
            )
    }
}

pub struct ProblemsDrawer {
    focus_handle: FocusHandle,
    diagnostics: Arc<Mutex<Vec<Diagnostic>>>,
    filtered_severity: Option<DiagnosticSeverity>,
    list: Entity<List<ProblemsListDelegate>>,
    selected_index: Option<IndexPath>,
}

struct ProblemsListDelegate {
    drawer: Entity<ProblemsDrawer>,
    diagnostics: Vec<Diagnostic>,
}

impl ListDelegate for ProblemsListDelegate {
    type Item = DiagnosticItem;

    fn items_count(&self, _section: usize, _cx: &App) -> usize {
        self.diagnostics.len()
    }

    fn perform_search(
        &mut self,
        _query: &str,
        _window: &mut Window,
        _cx: &mut Context<List<Self>>,
    ) -> gpui::Task<()> {
        gpui::Task::ready(())
    }

    fn render_item(
        &self,
        ix: IndexPath,
        _window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let diagnostic = self.diagnostics.get(ix.row)?.clone();
        let drawer = self.drawer.clone();
        
        Some(
            DiagnosticItem::new(diagnostic.clone())
                .on_click(move |_window, cx| {
                    let file_path = PathBuf::from(&diagnostic.file_path);
                    drawer.update(cx, |_, cx| {
                        cx.emit(NavigateToDiagnostic {
                            file_path,
                            line: diagnostic.line,
                            column: diagnostic.column,
                        });
                    });
                })
        )
    }

    fn render_empty(&self, _window: &mut Window, cx: &mut Context<List<Self>>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .p_8()
            .child(
                v_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        gpui_component::Icon::new(IconName::Check)
                            .size_8()
                            .text_color(cx.theme().success)
                    )
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(cx.theme().foreground)
                            .child("No problems detected")
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Your code is looking good!")
                    )
            )
    }

    fn confirm(&mut self, _secondary: bool, _window: &mut Window, cx: &mut Context<List<Self>>) {
        // Handle double-clicking or pressing Enter on a diagnostic
        if let Some(index) = self.drawer.read(cx).selected_index {
            if let Some(diagnostic) = self.diagnostics.get(index.row) {
                let file_path = PathBuf::from(&diagnostic.file_path);
                self.drawer.update(cx, |drawer, cx| {
                    cx.emit(NavigateToDiagnostic {
                        file_path,
                        line: diagnostic.line,
                        column: diagnostic.column,
                    });
                });
            }
        }
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _window: &mut Window,
        cx: &mut Context<List<Self>>,
    ) {
        self.drawer.update(cx, |drawer, _| {
            drawer.selected_index = ix;
        });
    }
}

impl ProblemsDrawer {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let diagnostics = Arc::new(Mutex::new(Vec::new()));
        
        let drawer_entity = cx.entity().clone();
        let list = cx.new(|cx| {
            List::new(
                ProblemsListDelegate {
                    drawer: drawer_entity,
                    diagnostics: Vec::new(),
                },
                _window,
                cx,
            )
        });

        Self {
            focus_handle,
            diagnostics,
            filtered_severity: None,
            list,
            selected_index: None,
        }
    }

    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic, cx: &mut Context<Self>) {
        {
            let mut diagnostics = self.diagnostics.lock().unwrap();
            diagnostics.push(diagnostic);
        }
        self.update_list(cx);
    }

    pub fn clear_diagnostics(&mut self, cx: &mut Context<Self>) {
        {
            let mut diagnostics = self.diagnostics.lock().unwrap();
            diagnostics.clear();
        }
        self.update_list(cx);
    }

    pub fn set_diagnostics(&mut self, diagnostics: Vec<Diagnostic>, cx: &mut Context<Self>) {
        {
            let mut diag = self.diagnostics.lock().unwrap();
            *diag = diagnostics;
        }
        self.update_list(cx);
    }

    fn update_list(&mut self, cx: &mut Context<Self>) {
        let diagnostics = self.diagnostics.lock().unwrap().clone();
        
        let filtered = if let Some(severity) = &self.filtered_severity {
            diagnostics
                .into_iter()
                .filter(|d| &d.severity == severity)
                .collect()
        } else {
            diagnostics
        };

        self.list.update(cx, |list, cx| {
            list.delegate_mut().diagnostics = filtered;
            cx.notify();
        });
        
        cx.notify();
    }

    pub fn count_by_severity(&self, severity: DiagnosticSeverity) -> usize {
        let diagnostics = self.diagnostics.lock().unwrap();
        diagnostics.iter().filter(|d| d.severity == severity).count()
    }

    pub fn total_count(&self) -> usize {
        self.diagnostics.lock().unwrap().len()
    }

    fn set_filter(&mut self, severity: Option<DiagnosticSeverity>, cx: &mut Context<Self>) {
        self.filtered_severity = severity;
        self.update_list(cx);
    }
}

impl Focusable for ProblemsDrawer {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ProblemsDrawer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let error_count = self.count_by_severity(DiagnosticSeverity::Error);
        let warning_count = self.count_by_severity(DiagnosticSeverity::Warning);
        let info_count = self.count_by_severity(DiagnosticSeverity::Information);
        let total_count = self.total_count();

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Header with title and filter buttons
                v_flex()
                    .w_full()
                    .bg(cx.theme().secondary)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        // Title bar
                        h_flex()
                            .w_full()
                            .px_3()
                            .py_2()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(cx.theme().foreground)
                                    .child(format!("Problems ({})", total_count))
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("clear")
                                            .ghost()
                                            .xsmall()
                                            .icon(IconName::Close)
                                            .tooltip("Clear all problems")
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.clear_diagnostics(cx);
                                            }))
                                    )
                            )
                    )
                    .child(
                        // Filter buttons bar
                        h_flex()
                            .w_full()
                            .px_3()
                            .py_1()
                            .gap_1()
                            .child(
                                Button::new("filter-all")
                                    .xsmall()
                                    .when(self.filtered_severity.is_none(), |btn| {
                                        btn.with_variant(ButtonVariant::Primary)
                                    })
                                    .when(self.filtered_severity.is_some(), |btn| {
                                        btn.ghost()
                                    })
                                    .label(format!("All ({})", total_count))
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.set_filter(None, cx);
                                    }))
                            )
                            .child(
                                Button::new("filter-errors")
                                    .xsmall()
                                    .when(
                                        self.filtered_severity == Some(DiagnosticSeverity::Error),
                                        |btn| btn.with_variant(ButtonVariant::Danger)
                                    )
                                    .when(
                                        self.filtered_severity != Some(DiagnosticSeverity::Error),
                                        |btn| btn.ghost()
                                    )
                                    .icon(IconName::Close)
                                    .label(format!("{}", error_count))
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.set_filter(Some(DiagnosticSeverity::Error), cx);
                                    }))
                            )
                            .child(
                                Button::new("filter-warnings")
                                    .xsmall()
                                    .when(
                                        self.filtered_severity == Some(DiagnosticSeverity::Warning),
                                        |btn| btn.with_variant(ButtonVariant::Warning)
                                    )
                                    .when(
                                        self.filtered_severity != Some(DiagnosticSeverity::Warning),
                                        |btn| btn.ghost()
                                    )
                                    .icon(IconName::TriangleAlert)
                                    .label(format!("{}", warning_count))
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.set_filter(Some(DiagnosticSeverity::Warning), cx);
                                    }))
                            )
                            .child(
                                Button::new("filter-info")
                                    .xsmall()
                                    .when(
                                        self.filtered_severity == Some(DiagnosticSeverity::Information),
                                        |btn| btn.with_variant(ButtonVariant::Primary)
                                    )
                                    .when(
                                        self.filtered_severity != Some(DiagnosticSeverity::Information),
                                        |btn| btn.ghost()
                                    )
                                    .icon(IconName::Info)
                                    .label(format!("{}", info_count))
                                    .on_click(cx.listener(|this, _, _window, cx| {
                                        this.set_filter(Some(DiagnosticSeverity::Information), cx);
                                    }))
                            )
                    )
            )
            .child(
                // Problems list - use proper scrollable container
                div()
                    .id("problems-list-container")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(self.list.clone())
            )
    }
}
