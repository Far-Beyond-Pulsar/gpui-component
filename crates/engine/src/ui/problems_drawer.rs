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

    pub fn color(&self) -> Hsla {
        match self {
            Self::Error => Hsla { h: 0.0, s: 1.0, l: 0.5, a: 1.0 },
            Self::Warning => Hsla { h: 60.0, s: 1.0, l: 0.5, a: 1.0 },
            Self::Information => Hsla { h: 200.0, s: 1.0, l: 0.5, a: 1.0 },
            Self::Hint => Hsla { h: 200.0, s: 0.5, l: 0.5, a: 1.0 },
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
}

impl DiagnosticItem {
    fn new(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostic,
            selected: false,
        }
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
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let diagnostic = self.diagnostic;
        
        div()
            .w_full()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .when(self.selected, |this| this.bg(cx.theme().selection))
            .hover(|this| this.bg(cx.theme().secondary))
            .cursor_pointer()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .w(px(12.))
                                    .h(px(12.))
                                    .rounded_full()
                                    .bg(diagnostic.severity.color())
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(diagnostic.severity.color())
                                    .child(diagnostic.severity.label())
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!(
                                        "{} [{}:{}]",
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
        _cx: &mut Context<List<Self>>,
    ) -> Option<Self::Item> {
        let diagnostic = self.diagnostics.get(ix.row)?.clone();
        Some(DiagnosticItem::new(diagnostic))
    }

    fn render_empty(&self, _window: &mut Window, cx: &mut Context<List<Self>>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("No problems detected")
            )
    }

    fn confirm(&mut self, _secondary: bool, _window: &mut Window, cx: &mut Context<List<Self>>) {
        // Handle clicking on a diagnostic to navigate to it
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
        _cx: &mut Context<List<Self>>,
    ) {
        self.drawer.update(_cx, |drawer, _| {
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

        self.list.update(cx, |list, _cx| {
            list.delegate_mut().diagnostics = filtered;
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

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Header with filter buttons
                h_flex()
                    .w_full()
                    .px_3()
                    .py_2()
                    .gap_2()
                    .bg(cx.theme().sidebar)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(cx.theme().foreground)
                            .child("Problems")
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("filter-all")
                            .ghost()
                            .small()
                            .when(self.filtered_severity.is_none(), |btn| {
                                btn.with_variant(ButtonVariant::Primary)
                            })
                            .label(format!("All ({})", self.total_count()))
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.set_filter(None, cx);
                            }))
                    )
                    .child(
                        Button::new("filter-errors")
                            .ghost()
                            .small()
                            .when(
                                self.filtered_severity == Some(DiagnosticSeverity::Error),
                                |btn| btn.with_variant(ButtonVariant::Danger)
                            )
                            .icon(IconName::Close)
                            .label(format!("Errors ({})", error_count))
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.set_filter(Some(DiagnosticSeverity::Error), cx);
                            }))
                    )
                    .child(
                        Button::new("filter-warnings")
                            .ghost()
                            .small()
                            .when(
                                self.filtered_severity == Some(DiagnosticSeverity::Warning),
                                |btn| btn.with_variant(ButtonVariant::Warning)
                            )
                            .icon(IconName::TriangleAlert)
                            .label(format!("Warnings ({})", warning_count))
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.set_filter(Some(DiagnosticSeverity::Warning), cx);
                            }))
                    )
                    .child(
                        Button::new("clear")
                            .ghost()
                            .small()
                            .icon(IconName::Close)
                            .tooltip("Clear all problems")
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.clear_diagnostics(cx);
                            }))
                    )
            )
            .child(
                // Problems list
                div()
                    .flex_1()
                    .scrollable(Axis::Vertical)
                    .child(self.list.clone())
            )
    }
}

