//! Problems Drawer - Displays diagnostics, errors, and warnings from rust-analyzer

// TODO: This should be a seprate window similar to Unreal Engine. The content is similar to VSCode's Problems panel
//       It should be toggleable from the View menu and dockable in the main window when docking is enabled

use gpui::{prelude::*, *};
use ui::{
    button::{Button, ButtonVariants as _, ButtonVariant},
    h_flex, v_flex, ActiveTheme as _, IconName, Sizable as _,
    Selectable,
};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

// Local diagnostic types (TODO: unify with ui::diagnostics)
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
                    .w_full()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .w_full()
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .child(
                                        ui::Icon::new(diagnostic.severity.icon())
                                            .size_4()
                                            .text_color(diagnostic.severity.color(cx))
                                    )
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
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
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
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
                            .w_full()
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
    selected_index: Option<usize>,
}

impl ProblemsDrawer {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let diagnostics = Arc::new(Mutex::new(Vec::new()));

        Self {
            focus_handle,
            diagnostics,
            filtered_severity: None,
            selected_index: None,
        }
    }

    pub fn add_diagnostic(&mut self, diagnostic: Diagnostic, cx: &mut Context<Self>) {
        {
            let mut diagnostics = self.diagnostics.lock().unwrap();
            diagnostics.push(diagnostic);
        }
        cx.notify();
    }

    pub fn clear_diagnostics(&mut self, cx: &mut Context<Self>) {
        {
            let mut diagnostics = self.diagnostics.lock().unwrap();
            diagnostics.clear();
        }
        self.selected_index = None;
        cx.notify();
    }

    pub fn set_diagnostics(&mut self, diagnostics: Vec<Diagnostic>, cx: &mut Context<Self>) {
        {
            let mut diag = self.diagnostics.lock().unwrap();
            *diag = diagnostics;
        }
        self.selected_index = None;
        cx.notify();
    }

    fn get_filtered_diagnostics(&self) -> Vec<Diagnostic> {
        let diagnostics = self.diagnostics.lock().unwrap().clone();
        
        if let Some(severity) = &self.filtered_severity {
            diagnostics
                .into_iter()
                .filter(|d| &d.severity == severity)
                .collect()
        } else {
            diagnostics
        }
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
        self.selected_index = None;
        cx.notify();
    }

    fn select_diagnostic(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected_index = Some(index);
        cx.notify();
    }

    fn navigate_to_diagnostic(&mut self, diagnostic: &Diagnostic, cx: &mut Context<Self>) {
        let file_path = PathBuf::from(&diagnostic.file_path);
        cx.emit(NavigateToDiagnostic {
            file_path,
            line: diagnostic.line,
            column: diagnostic.column,
        });
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
        
        let filtered_diagnostics = self.get_filtered_diagnostics();
        let selected_index = self.selected_index;

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
                // Problems list - simple scrollable container
                div()
                    .id("problems-list-container")
                    .flex_1()
                    .overflow_y_scroll()
                    .when(filtered_diagnostics.is_empty(), |container| {
                        container.child(
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
                                            ui::Icon::new(IconName::Check)
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
                        )
                    })
                    .when(!filtered_diagnostics.is_empty(), |container| {
                        let drawer_entity = cx.entity().clone();
                        container.child(
                            v_flex()
                                .w_full()
                                .children(
                                    filtered_diagnostics
                                        .into_iter()
                                        .enumerate()
                                        .map(|(index, diagnostic)| {
                                            let is_selected = selected_index == Some(index);
                                            let drawer = drawer_entity.clone();
                                            let diag = diagnostic.clone();
                                            
                                            DiagnosticItem::new(diagnostic)
                                                .selected(is_selected)
                                                .on_click(move |_window, cx| {
                                                    drawer.update(cx, |drawer, cx| {
                                                        drawer.select_diagnostic(index, cx);
                                                        drawer.navigate_to_diagnostic(&diag, cx);
                                                    });
                                                })
                                        })
                                )
                        )
                    })
            )
    }
}
