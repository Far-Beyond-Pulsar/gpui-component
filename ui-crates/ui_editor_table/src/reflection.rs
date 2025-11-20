use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SqlType {
    Integer,
    Real,
    Text,
    Blob,
    Boolean,
    DateTime,
    ForeignKey { table: String },
}

impl SqlType {
    pub fn to_sql_string(&self) -> String {
        match self {
            SqlType::Integer => "INTEGER".to_string(),
            SqlType::Real => "REAL".to_string(),
            SqlType::Text => "TEXT".to_string(),
            SqlType::Blob => "BLOB".to_string(),
            SqlType::Boolean => "INTEGER".to_string(), // SQLite doesn't have native boolean
            SqlType::DateTime => "TEXT".to_string(),    // Store as ISO 8601 string
            SqlType::ForeignKey { table } => format!("INTEGER REFERENCES {}(id)", table),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    pub name: String,
    pub sql_type: SqlType,
    pub nullable: bool,
    pub is_foreign_key: bool,
    pub foreign_table: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeSchema {
    pub type_name: String,
    pub table_name: String,
    pub fields: Vec<FieldSchema>,
    pub has_sub_structs: bool,
}

impl TypeSchema {
    pub fn new(type_name: impl Into<String>) -> Self {
        let type_name = type_name.into();
        let table_name = to_snake_case(&type_name);

        Self {
            type_name,
            table_name,
            fields: vec![],
            has_sub_structs: false,
        }
    }

    pub fn add_field(&mut self, name: impl Into<String>, sql_type: SqlType, nullable: bool) {
        let name = name.into();
        let (is_foreign_key, foreign_table) = match &sql_type {
            SqlType::ForeignKey { table } => (true, Some(table.clone())),
            _ => (false, None),
        };

        self.fields.push(FieldSchema {
            name,
            sql_type,
            nullable,
            is_foreign_key,
            foreign_table,
        });

        if is_foreign_key {
            self.has_sub_structs = true;
        }
    }

    pub fn to_create_table_sql(&self) -> String {
        let mut sql = format!("CREATE TABLE IF NOT EXISTS {} (\n", self.table_name);
        sql.push_str("    id INTEGER PRIMARY KEY AUTOINCREMENT,\n");

        for field in &self.fields {
            let null_constraint = if field.nullable { "" } else { " NOT NULL" };
            sql.push_str(&format!(
                "    {} {}{},\n",
                field.name,
                field.sql_type.to_sql_string(),
                null_constraint
            ));
        }

        sql.push_str(");");
        sql
    }
}

pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_upper = false;

    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !prev_is_upper {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
            prev_is_upper = true;
        } else {
            result.push(c);
            prev_is_upper = false;
        }
    }

    result
}

pub fn rust_type_to_sql_type(rust_type: &str) -> SqlType {
    match rust_type {
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
            SqlType::Integer
        }
        "f32" | "f64" => SqlType::Real,
        "String" | "str" | "&str" => SqlType::Text,
        "bool" => SqlType::Boolean,
        "Vec<u8>" | "[u8]" | "&[u8]" => SqlType::Blob,
        "DateTime" | "chrono::DateTime" => SqlType::DateTime,
        _ => {
            if rust_type.starts_with("Option<") {
                let inner = &rust_type[7..rust_type.len() - 1];
                return rust_type_to_sql_type(inner);
            }
            SqlType::Text
        }
    }
}

pub trait TableType {
    fn schema() -> TypeSchema;
}

#[macro_export]
macro_rules! table_type {
    ($type_name:ident { $($field:ident: $field_type:ty),* $(,)? }) => {
        impl $crate::reflection::TableType for $type_name {
            fn schema() -> $crate::reflection::TypeSchema {
                let mut schema = $crate::reflection::TypeSchema::new(stringify!($type_name));

                $(
                    let rust_type = stringify!($field_type);
                    let is_nullable = rust_type.starts_with("Option<");
                    let sql_type = $crate::reflection::rust_type_to_sql_type(rust_type);
                    schema.add_field(stringify!($field), sql_type, is_nullable);
                )*

                schema
            }
        }
    };
}
