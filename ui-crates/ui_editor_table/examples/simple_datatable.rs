use gpui::*;
use ui_editor_table::{DatabaseManager, DataTableEditor, TypeSchema, reflection::SqlType};

#[derive(Debug, Clone)]
struct PlayerData {
    name: String,
    level: i32,
    health: f64,
    mana: f64,
    experience: i64,
    is_online: bool,
}

#[derive(Debug, Clone)]
struct ItemData {
    item_name: String,
    quantity: i32,
    value: f64,
    is_equipped: bool,
    owner_id: i64, // Foreign key to PlayerData
}

fn main() {
    App::new().run(|cx: &mut AppContext| {
        // Create a window
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::new(px(100.0), px(100.0)),
                    size: Size {
                        width: px(1200.0),
                        height: px(800.0),
                    },
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some("Data Table Editor - Game Data".into()),
                    appears_transparent: false,
                    ..Default::default()
                }),
                ..Default::default()
            },
            |cx| {
                // Create the editor
                cx.new(|cx| {
                    let mut editor = DataTableEditor::new(cx);

                    // Define PlayerData schema
                    let mut player_schema = TypeSchema::new("PlayerData");
                    player_schema.add_field("name", SqlType::Text, false);
                    player_schema.add_field("level", SqlType::Integer, false);
                    player_schema.add_field("health", SqlType::Real, false);
                    player_schema.add_field("mana", SqlType::Real, false);
                    player_schema.add_field("experience", SqlType::Integer, false);
                    player_schema.add_field("is_online", SqlType::Boolean, false);

                    // Define ItemData schema with foreign key
                    let mut item_schema = TypeSchema::new("ItemData");
                    item_schema.add_field("item_name", SqlType::Text, false);
                    item_schema.add_field("quantity", SqlType::Integer, false);
                    item_schema.add_field("value", SqlType::Real, false);
                    item_schema.add_field("is_equipped", SqlType::Boolean, false);
                    item_schema.add_field(
                        "owner_id",
                        SqlType::ForeignKey {
                            table: "player_data".to_string(),
                        },
                        false,
                    );

                    // Register schemas
                    editor.register_type_schema(player_schema).unwrap();
                    editor.register_type_schema(item_schema).unwrap();

                    // Add some sample data
                    let db = &editor.db;

                    // Insert players
                    db.insert_row(
                        "player_data",
                        vec![
                            serde_json::json!("Alice"),
                            serde_json::json!(25),
                            serde_json::json!(100.0),
                            serde_json::json!(50.0),
                            serde_json::json!(1500),
                            serde_json::json!(true),
                        ],
                    )
                    .unwrap();

                    db.insert_row(
                        "player_data",
                        vec![
                            serde_json::json!("Bob"),
                            serde_json::json!(18),
                            serde_json::json!(85.5),
                            serde_json::json!(30.0),
                            serde_json::json!(800),
                            serde_json::json!(false),
                        ],
                    )
                    .unwrap();

                    db.insert_row(
                        "player_data",
                        vec![
                            serde_json::json!("Charlie"),
                            serde_json::json!(42),
                            serde_json::json!(120.0),
                            serde_json::json!(75.0),
                            serde_json::json!(5000),
                            serde_json::json!(true),
                        ],
                    )
                    .unwrap();

                    // Insert items
                    db.insert_row(
                        "item_data",
                        vec![
                            serde_json::json!("Iron Sword"),
                            serde_json::json!(1),
                            serde_json::json!(150.0),
                            serde_json::json!(true),
                            serde_json::json!(1), // Owner: Alice
                        ],
                    )
                    .unwrap();

                    db.insert_row(
                        "item_data",
                        vec![
                            serde_json::json!("Health Potion"),
                            serde_json::json!(5),
                            serde_json::json!(25.0),
                            serde_json::json!(false),
                            serde_json::json!(2), // Owner: Bob
                        ],
                    )
                    .unwrap();

                    db.insert_row(
                        "item_data",
                        vec![
                            serde_json::json!("Magic Staff"),
                            serde_json::json!(1),
                            serde_json::json!(300.0),
                            serde_json::json!(true),
                            serde_json::json!(3), // Owner: Charlie
                        ],
                    )
                    .unwrap();

                    editor
                })
            },
        )
        .unwrap();
    });
}
