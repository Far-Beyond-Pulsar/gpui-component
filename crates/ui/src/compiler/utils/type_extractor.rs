//! # Type Extractor
//!
//! Extracts all unique types used by Blueprint nodes to provide
//! a complete type list for variable declarations.

use std::collections::HashSet;
use crate::compiler::get_node_metadata;

/// Extract all unique types from registered blueprint nodes
///
/// Returns a sorted list of all types that appear as:
/// - Node parameters (inputs)
/// - Node return types (outputs)
pub fn extract_all_blueprint_types() -> Vec<String> {
    let metadata = get_node_metadata();
    let mut types = HashSet::new();

    for node in metadata.values() {
        // Collect parameter types
        for param in node.params {
            types.insert(param.ty.to_string());
        }

        // Collect return type
        if let Some(return_ty) = node.return_type {
            types.insert(return_ty.to_string());
        }
    }

    // Convert to sorted vector
    let mut type_list: Vec<String> = types.into_iter().collect();
    type_list.sort();
    type_list
}

/// Get user-friendly display name for a type
pub fn get_type_display_name(ty: &str) -> String {
    match ty {
        "()" => "Unit".to_string(),
        "bool" => "Boolean".to_string(),
        "i32" => "Integer (32-bit)".to_string(),
        "i64" => "Integer (64-bit)".to_string(),
        "f32" => "Float (32-bit)".to_string(),
        "f64" => "Float (64-bit)".to_string(),
        "&str" => "String".to_string(),
        "String" => "String (owned)".to_string(),
        _ if ty.starts_with('(') && ty.ends_with(')') => {
            format!("Tuple: {}", ty)
        }
        _ => ty.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_extraction() {
        let types = extract_all_blueprint_types();

        // Should have at least common types
        assert!(!types.is_empty(), "Should extract some types");

        // Common types we expect
        assert!(types.contains(&"i32".to_string()) || types.contains(&"f32".to_string()),
                "Should have numeric types");
    }

    #[test]
    fn test_type_display_names() {
        assert_eq!(get_type_display_name("bool"), "Boolean");
        assert_eq!(get_type_display_name("f32"), "Float (32-bit)");
        assert_eq!(get_type_display_name("&str"), "String");
    }
}
