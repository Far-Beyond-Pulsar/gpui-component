use gpui::{prelude::*, *};
use ui::{
    h_flex, v_flex, button::Button, table::{Column, ColumnSort, Table, TableDelegate, TableEvent},
    ActiveTheme, Sizable, Size, StyleSized, StyledExt, Selectable,
};
use std::ops::Range;
use crate::{
    database::{DatabaseManager, RowData, CellValue},
    reflection::TypeSchema,
    cell_editors::{CellEditor, CellEditorView},
};

pub struct DataTableState {
    pub editing_cell: Option<(usize, usize)>, // (row_idx, col_idx)
    pub selected_row: Option<usize>,
}

pub struct DataTableView {
    db: DatabaseManager,
    table_name: String,
    schema: TypeSchema,
    rows: Vec<RowData>,
    columns: Vec<Column>,
    size: Size,
    total_rows: usize,
    visible_range: Range<usize>,
    pub state: DataTableState,
}

impl DataTableView {
    pub fn new(db: DatabaseManager, table_name: String) -> anyhow::Result<Self> {
        let schema = db
            .get_schema(&table_name)
            .ok_or_else(|| anyhow::anyhow!("Schema not found for table: {}", table_name))?;

        let total_rows = db.get_row_count(&table_name)?;
        let rows = db.fetch_rows(&table_name, 0, 100)?;

        let mut columns = vec![
            Column::new("id", "ID")
                .width(60.)
                .resizable(false)
                .sortable(),
        ];

        for field in &schema.fields {
            columns.push(
                Column::new(&field.name, &field.name)
                    .width(150.)
                    .sortable()
            );
        }

        Ok(Self {
            db,
            table_name,
            schema,
            rows,
            columns,
            size: Size::default(),
            total_rows,
            visible_range: 0..0,
            state: DataTableState {
                editing_cell: None,
                selected_row: None,
            },
        })
    }

    pub fn refresh_rows(&mut self, offset: usize, limit: usize) -> anyhow::Result<()> {
        self.rows = self.db.fetch_rows(&self.table_name, offset, limit)?;
        self.total_rows = self.db.get_row_count(&self.table_name)?;
        Ok(())
    }

    pub fn add_new_row(&mut self) -> anyhow::Result<()> {
        let default_values: Vec<serde_json::Value> = self
            .schema
            .fields
            .iter()
            .map(|field| {
                if field.nullable {
                    serde_json::Value::Null
                } else {
                    match field.sql_type {
                        crate::reflection::SqlType::Integer => serde_json::Value::Number(0.into()),
                        crate::reflection::SqlType::Real => {
                            serde_json::Number::from_f64(0.0)
                                .map(serde_json::Value::Number)
                                .unwrap_or(serde_json::Value::Null)
                        }
                        crate::reflection::SqlType::Boolean => serde_json::Value::Bool(false),
                        _ => serde_json::Value::String(String::new()),
                    }
                }
            })
            .collect();

        self.db.insert_row(&self.table_name, default_values)?;
        self.refresh_rows(0, 100)?;
        Ok(())
    }

    pub fn delete_row(&mut self, row_idx: usize) -> anyhow::Result<()> {
        if let Some(row) = self.rows.get(row_idx) {
            self.db.delete_row(&self.table_name, row.id)?;
            self.refresh_rows(0, 100)?;
        }
        Ok(())
    }

    pub fn update_cell(
        &mut self,
        row_idx: usize,
        col_idx: usize,
        value: serde_json::Value,
    ) -> anyhow::Result<()> {
        if let Some(row) = self.rows.get(row_idx) {
            if col_idx > 0 && col_idx <= self.schema.fields.len() {
                let field = &self.schema.fields[col_idx - 1];
                self.db.update_cell(&self.table_name, row.id, &field.name, value)?;
                self.refresh_rows(0, 100)?;
            }
        }
        Ok(())
    }
}

impl TableDelegate for DataTableView {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_th(
        &self,
        col_ix: usize,
        _: &mut Window,
        _cx: &mut Context<Table<Self>>,
    ) -> impl IntoElement {
        let col = &self.columns[col_ix];

        div()
            .child(col.name.clone())
            .text_sm()
            .font_semibold()
            .px_2()
            .py_1()
    }

    fn render_tr(
        &self,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<Table<Self>>,
    ) -> Stateful<Div> {
        let is_selected = self.state.selected_row == Some(row_ix);
        div()
            .id(row_ix)
            .cursor_pointer()
            .on_click(cx.listener(move |table, _, _, cx| {
                table.delegate_mut().state.selected_row = Some(row_ix);
                cx.notify();
            }))
            .when(is_selected, |this| {
                this.bg(cx.theme().accent.opacity(0.1))
            })
    }

    fn render_td(
        &self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<Table<Self>>,
    ) -> impl IntoElement {
        if let Some(row) = self.rows.get(row_ix) {
            if col_ix == 0 {
                return div()
                    .px_2()
                    .py_1()
                    .text_sm()
                    .child(row.id.to_string())
                    .into_any_element();
            }

            let cell_idx = col_ix - 1;
            if let Some(cell) = row.cells.get(cell_idx) {
                let is_editing = self.state.editing_cell == Some((row_ix, col_ix));

                if is_editing {
                    let field = &self.schema.fields[cell_idx];
                    let editor = CellEditor::new_from_sql_type(&field.sql_type, Some(cell.value.clone()));
                    return CellEditorView::new(editor).into_any_element();
                }

                return div()
                    .id(("cell", row_ix * 1000 + col_ix))
                    .px_2()
                    .py_1()
                    .text_sm()
                    .cursor_pointer()
                    .on_click(cx.listener(move |table, _, _, cx| {
                        table.delegate_mut().state.editing_cell = Some((row_ix, col_ix));
                        cx.notify();
                    }))
                    .hover(|this| this.bg(cx.theme().muted.opacity(0.5)))
                    .child(cell.display.clone())
                    .into_any_element();
            }
        }

        div()
            .px_2()
            .py_1()
            .text_sm()
            .child("—")
            .into_any_element()
    }

    fn visible_rows_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _cx: &mut Context<Table<Self>>,
    ) {
        self.visible_range = visible_range.clone();

        if visible_range.end > self.rows.len() && self.rows.len() < self.total_rows {
            if let Err(e) = self.refresh_rows(0, visible_range.end + 50) {
                eprintln!("Failed to load more rows: {}", e);
            }
        }
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<Table<Self>>,
    ) {
        println!("Sort column {} {:?}", col_ix, sort);
    }

    fn loading(&self, _: &App) -> bool {
        false
    }

    fn is_eof(&self, _: &App) -> bool {
        self.rows.len() >= self.total_rows
    }
}
