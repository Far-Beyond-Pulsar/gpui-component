# Data Table Editor

A fully functional SQLite database editor for the Pulsar Native game engine, equivalent to Unreal Engine's Data Tables system. Inspired by DBeaver's editing and querying workflow.

## Features

- **Type-to-Schema Mapping**: Automatically create SQLite tables from Rust types
- **Virtual Scrolling Table**: High-performance table view with virtual scrolling for large datasets
- **Inline Editing**: Click on any cell to edit its value directly in the table view
- **Row Selection**: Click on rows to select them for deletion or other operations
- **Interactive Sidebar**: Click on table names to switch between different tables
- **Toolbar Actions**: Add rows, delete selected rows, and refresh data with interactive buttons
- **Tab Navigation**: Switch between Data view and Query Editor with clickable tabs
- **Foreign Key Support**: Sub-structs are represented as foreign keys with dropdown editors
- **Query Editor**: DBeaver-style SQL query interface with execute and clear buttons
- **CRUD Operations**: Full Create, Read, Update, Delete support
- **Type Safety**: Schema validation and type checking

## Usage

### Creating a Data Table Editor

```rust
use gpui::*;
use ui_editor_table::{DatabaseManager, DataTableEditor, TypeSchema, reflection::SqlType};

// Create an editor
let editor = cx.new_view(|cx| DataTableEditor::new(cx));

// Or open an existing database
let editor = cx.new_view(|cx| {
    DataTableEditor::open_database("path/to/database.db".into(), cx).unwrap()
});
```

### Defining Type Schemas

```rust
// Define a schema for a Rust type
let mut player_schema = TypeSchema::new("PlayerData");
player_schema.add_field("name", SqlType::Text, false);
player_schema.add_field("level", SqlType::Integer, false);
player_schema.add_field("health", SqlType::Real, false);
player_schema.add_field("is_online", SqlType::Boolean, false);

// Register the schema
editor.register_type_schema(player_schema).unwrap();
```

### Foreign Keys (Sub-Structs)

```rust
// Define a schema with a foreign key
let mut item_schema = TypeSchema::new("ItemData");
item_schema.add_field("item_name", SqlType::Text, false);
item_schema.add_field("quantity", SqlType::Integer, false);
item_schema.add_field(
    "owner_id",
    SqlType::ForeignKey {
        table: "player_data".to_string(),
    },
    false,
);

editor.register_type_schema(item_schema).unwrap();
```

When editing the `owner_id` field, a dropdown will appear showing all available players. Behind the scenes, this is stored as an ID pointing to the related table.

### Basic Operations

#### Selecting Tables
Click on any table name in the left sidebar to view and edit its data. The selected table will be highlighted.

#### Adding Rows
Click the "Add Row" button in the toolbar to insert a new row with default values. The new row will appear at the bottom of the table.

#### Editing Cells
Click on any cell in the table to enter edit mode. Type your changes and the value will be updated in the database.

#### Selecting and Deleting Rows
1. Click on any row to select it (the row will be highlighted)
2. Click the "Delete Row" button in the toolbar to remove the selected row
3. The selection is cleared after deletion

#### Refreshing Data
Click the "Refresh" button in the toolbar to reload data from the database. Useful after external changes or to see updates.

#### Programmatic Operations
```rust
// Select a table to view programmatically
editor.select_table("player_data".to_string(), window, cx).unwrap();

// Add a new row programmatically
editor.add_new_row(cx).unwrap();

// Delete selected row programmatically
editor.delete_selected_row(cx).unwrap();

// Refresh data programmatically
editor.refresh_data(cx).unwrap();
```

### Direct Database Operations

```rust
let db = DatabaseManager::new("game_data.db")?;

// Insert a row
db.insert_row(
    "player_data",
    vec![
        serde_json::json!("Alice"),
        serde_json::json!(25),
        serde_json::json!(100.0),
        serde_json::json!(true),
    ],
)?;

// Update a cell
db.update_cell(
    "player_data",
    1, // row_id
    "level",
    serde_json::json!(26),
)?;

// Delete a row
db.delete_row("player_data", 1)?;

// Execute a query
let results = db.execute_query("SELECT * FROM player_data WHERE level > 20")?;
```

## UI Interaction Guide

### Sidebar Navigation
- **Table List**: All registered tables appear in the left sidebar
- **Selection**: Click any table name to load and display its contents
- **Visual Feedback**: Selected table is highlighted with accent color
- **Hover Effects**: Tables show hover state for better discoverability

### Toolbar Actions
- **Add Row**: Creates a new row with default values based on field types
- **Delete Row**: Removes the currently selected row (disabled if no row selected)
- **Refresh**: Reloads all data from the database
- **Current Table**: Displays name of currently selected table

### Table Interaction
- **Row Selection**: Click any row to select it (highlighted with accent background)
- **Cell Editing**: Click any cell (except ID column) to enter edit mode
- **Visual Feedback**: Cells show hover effect when mouse is over them
- **ID Column**: Read-only, displays database row IDs

### Tab Navigation
- **Data Tab**: Main table view with inline editing (default)
- **Query Editor Tab**: SQL query interface for advanced operations
- **Active Tab**: Highlighted with primary button style

### Query Editor
- **SQL Input**: Enter SQL queries in the text area
- **Execute Button**: Runs the query and displays results (shortcut: F5)
- **Clear Results**: Removes query results from view
- **Results Display**: Shows column names and data in a formatted table
- **Error Display**: Shows SQL errors in a red-highlighted box
- **Execution Stats**: Displays row count and execution time

## Architecture

The editor consists of several key components:

1. **Type Reflection System** (`reflection.rs`): Maps Rust types to SQLite schemas
2. **Database Manager** (`database.rs`): Handles all SQLite operations
3. **Table View** (`table_view.rs`): Virtual scrolling table with inline editing and row selection
4. **Cell Editors** (`cell_editors.rs`): Type-specific editors for each cell type
5. **Query Editor** (`query_editor.rs`): SQL query interface with interactive execution
6. **Main Editor** (`editor.rs`): Brings everything together in a DBeaver-style interface

## Supported SQL Types

- `Integer` - For `i8`, `i16`, `i32`, `i64`, `isize`, `u8`, `u16`, `u32`, `u64`, `usize`
- `Real` - For `f32`, `f64`
- `Text` - For `String`, `str`, `&str`
- `Boolean` - For `bool`
- `Blob` - For `Vec<u8>`, `[u8]`, `&[u8]`
- `DateTime` - For `chrono::DateTime`
- `ForeignKey` - For sub-structs (stored as INTEGER with foreign key constraint)

## Example

See `examples/simple_datatable.rs` for a complete working example demonstrating:
- Creating schemas
- Adding foreign key relationships
- Inserting data
- Viewing and editing tables

Run the example with:
```bash
cargo run --example simple_datatable -p ui_editor_table
```

## Integration with Unreal-Style Workflow

Like Unreal Engine's Data Tables:

1. Define your Rust structs
2. Register them as schemas
3. The editor keeps the table synced with the type's fields
4. Sub-structs become foreign key relationships
5. Edit data in a spreadsheet-like interface
6. Query data with SQL

The key difference from traditional database editors: the schema is driven by your Rust types, ensuring type safety and automatic synchronization.
