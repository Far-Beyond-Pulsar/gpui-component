use gpui::{prelude::*, *};
use ui::{
    h_flex, v_flex, button::{Button, ButtonVariants}, label::Label,
    input::TextInput,
    ActiveTheme, Sizable, Size, StyleSized, StyledExt, Disableable,
};
use crate::database::{DatabaseManager, CellValue};

pub struct QueryEditor {
    db: DatabaseManager,
    query_buffer: SharedString,
    results: Option<QueryResult>,
    error: Option<String>,
    is_executing: bool,
}

#[derive(Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<CellValue>>,
    pub row_count: usize,
    pub execution_time_ms: u64,
}

impl QueryEditor {
    pub fn new(db: DatabaseManager) -> Self {
        Self {
            db,
            query_buffer: "SELECT * FROM ".into(),
            results: None,
            error: None,
            is_executing: false,
        }
    }

    pub fn set_query(&mut self, query: SharedString) {
        self.query_buffer = query;
    }

    pub fn execute_query(&mut self) -> anyhow::Result<()> {
        self.is_executing = true;
        self.error = None;

        let start = std::time::Instant::now();

        match self.db.execute_query(&self.query_buffer.to_string()) {
            Ok(rows) => {
                let execution_time_ms = start.elapsed().as_millis() as u64;
                let row_count = rows.len();

                let columns = if !rows.is_empty() && !rows[0].is_empty() {
                    (0..rows[0].len())
                        .map(|i| format!("Column {}", i + 1))
                        .collect()
                } else {
                    Vec::new()
                };

                self.results = Some(QueryResult {
                    columns,
                    rows,
                    row_count,
                    execution_time_ms,
                });
            }
            Err(e) => {
                self.error = Some(format!("Query error: {}", e));
                self.results = None;
            }
        }

        self.is_executing = false;
        Ok(())
    }

    pub fn clear_results(&mut self) {
        self.results = None;
        self.error = None;
    }

    pub fn render_query_input(&mut self, cx: &mut Context<QueryEditor>) -> impl IntoElement {
        let query = self.query_buffer.clone();
        v_flex()
            .gap_2()
            .child(
                Label::new("SQL Query")
                    .text_sm()
                    .font_semibold()
            )
            .child(
                div()
                    .w_full()
                    .h_32()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_md()
                    .p_2()
                    .bg(cx.theme().background)
                    .child(
                        div()
                            .text_sm()
                            .child(query.to_string())
                    )
            )
    }

    pub fn render_controls(&self, cx: &Context<QueryEditor>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(
                Button::new("execute")
                    .label(if self.is_executing { "Executing..." } else { "Execute (F5)" })
                    .disabled(self.is_executing)
                    .primary()
                    .on_click(cx.listener(|editor, _, _, cx| {
                        if let Err(e) = editor.execute_query() {
                            eprintln!("Failed to execute query: {}", e);
                        }
                        cx.notify();
                    }))
            )
            .child(
                Button::new("clear")
                    .label("Clear Results")
                    .outline()
                    .on_click(cx.listener(|editor, _, _, cx| {
                        editor.clear_results();
                        cx.notify();
                    }))
            )
            .when(self.results.is_some(), |this| {
                let result = self.results.as_ref().unwrap();
                this.child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!(
                            "{} rows in {} ms",
                            result.row_count, result.execution_time_ms
                        ))
                )
            })
    }

    pub fn render_results(&self, cx: &Context<QueryEditor>) -> impl IntoElement {
        v_flex()
            .w_full()
            .flex_1()
            .gap_2()
            .when_some(self.error.as_ref(), |this, error| {
                this.child(
                    div()
                        .w_full()
                        .p_4()
                        .bg(cx.theme().red.opacity(0.1))
                        .border_1()
                        .border_color(cx.theme().red)
                        .rounded_md()
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().red)
                                .child(error.clone())
                        )
                )
            })
            .when_some(self.results.as_ref(), |this, result| {
                this.child(
                    v_flex()
                        .w_full()
                        .flex_1()
                        .gap_1()
                        .child(
                            div()
                                .w_full()
                                .bg(cx.theme().muted)
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_t_md()
                                .child(
                                    h_flex()
                                        .children(result.columns.iter().map(|col| {
                                            div()
                                                .flex_1()
                                                .px_3()
                                                .py_2()
                                                .text_sm()
                                                .font_semibold()
                                                .border_r_1()
                                                .border_color(cx.theme().border)
                                                .child(col.clone())
                                        }))
                                )
                        )
                        .child(
                            v_flex()
                                .w_full()
                                .flex_1()
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_b_md()
                                .children(result.rows.iter().enumerate().map(|(row_idx, row)| {
                                    h_flex()
                                        .w_full()
                                        .when(row_idx % 2 == 0, |this| {
                                            this.bg(cx.theme().background)
                                        })
                                        .when(row_idx % 2 == 1, |this| {
                                            this.bg(cx.theme().muted.opacity(0.3))
                                        })
                                        .children(row.iter().map(|cell| {
                                            div()
                                                .flex_1()
                                                .px_3()
                                                .py_2()
                                                .text_sm()
                                                .border_r_1()
                                                .border_color(cx.theme().border)
                                                .child(cell.display.clone())
                                        }))
                                }))
                        )
                )
            })
    }
}

pub struct QueryEditorView {
    editor: Entity<QueryEditor>,
}

impl QueryEditorView {
    pub fn new(db: DatabaseManager, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let editor = cx.new(|_| QueryEditor::new(db));
        Self { editor }
    }
}

impl Render for QueryEditorView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let query_input = self.editor.update(cx, |editor, cx| {
            editor.render_query_input(cx)
        });
        let controls = self.editor.update(cx, |editor, cx| {
            editor.render_controls(cx)
        });
        let results = self.editor.update(cx, |editor, cx| {
            editor.render_results(cx)
        });
        
        v_flex()
            .size_full()
            .gap_4()
            .p_4()
            .child(query_input)
            .child(controls)
            .child(results)
    }
}
