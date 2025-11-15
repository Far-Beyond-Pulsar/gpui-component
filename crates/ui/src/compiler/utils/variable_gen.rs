//! # Variable Generation
//!
//! Generates thread-safe variable declarations and accessor code for Blueprint classes.
//!
//! Blueprint classes can have member variables that need to be accessible from
//! multiple contexts (e.g., from spawned threads). This module generates the
//! appropriate wrappers to ensure thread safety.
//!
//! ## Thread Safety Strategy
//!
//! - **Copy types** (i32, f32, bool, etc.) use `Cell<T>` for interior mutability
//! - **Non-Copy types** (String, Vec, etc.) use `RefCell<T>`
//! - All types wrapped in `Arc` for shared ownership across threads
//!
//! ## Generated Code Example
//!
//! For a variable `health: i32`, generates:
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use std::cell::Cell;
//!
//! pub struct Vars {
//!     health: Arc<Cell<i32>>,
//! }
//!
//! impl Vars {
//!     pub fn new() -> Self {
//!         Self {
//!             health: Arc::new(Cell::new(100)),
//!         }
//!     }
//! }
//! ```
//!
//! ## Getter/Setter Nodes
//!
//! For each variable, getter and setter nodes are automatically available:
//! - `get_health` - Returns current value
//! - `set_health` - Sets new value

use std::collections::HashMap;

/// Variable metadata for code generation
#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub name: String,
    pub var_type: String,
}

impl VariableInfo {
    /// Create new variable info
    pub fn new(name: impl Into<String>, var_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            var_type: var_type.into(),
        }
    }

    /// Check if this type is Copy (uses Cell) or not (uses RefCell)
    pub fn is_copy_type(&self) -> bool {
        is_copy_type(&self.var_type)
    }

    /// Get the wrapper type (Cell or RefCell)
    pub fn wrapper_type(&self) -> &'static str {
        if self.is_copy_type() {
            "Cell"
        } else {
            "RefCell"
        }
    }

    /// Generate the full type declaration (Arc<Cell<T>> or Arc<RefCell<T>>)
    pub fn full_type(&self) -> String {
        format!("Arc<{}<{}>>", self.wrapper_type(), self.var_type)
    }

    /// Generate initialization code
    pub fn initialization(&self, default_value: Option<&str>) -> String {
        let wrapper = self.wrapper_type();
        let value = default_value.unwrap_or_else(|| get_default_value(&self.var_type));
        
        format!("Arc::new({}::new({}))", wrapper, value)
    }
}

/// Generate variable struct definition
pub fn generate_vars_struct(variables: &HashMap<String, String>) -> String {
    let mut code = String::from("use std::sync::Arc;\nuse std::cell::{Cell, RefCell};\n\n");
    
    code.push_str("/// Blueprint class variables\n");
    code.push_str("pub struct Vars {\n");
    
    for (name, var_type) in variables {
        let info = VariableInfo::new(name, var_type);
        code.push_str(&format!("    pub {}: {},\n", name, info.full_type()));
    }
    
    code.push_str("}\n\n");
    
    // Generate impl with constructor
    code.push_str("impl Vars {\n");
    code.push_str("    pub fn new() -> Self {\n");
    code.push_str("        Self {\n");
    
    for (name, var_type) in variables {
        let info = VariableInfo::new(name, var_type);
        code.push_str(&format!("            {}: {},\n", name, info.initialization(None)));
    }
    
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n");
    
    code
}

/// Generate getter node code
pub fn generate_getter_code(var_name: &str, var_type: &str) -> String {
    let info = VariableInfo::new(var_name, var_type);
    
    if info.is_copy_type() {
        // Copy types: just get()
        format!("self.vars.{}.get()", var_name)
    } else {
        // Non-Copy types: borrow and clone
        format!("self.vars.{}.borrow().clone()", var_name)
    }
}

/// Generate setter node code
pub fn generate_setter_code(var_name: &str, var_type: &str, value_expr: &str) -> String {
    let info = VariableInfo::new(var_name, var_type);
    
    if info.is_copy_type() {
        // Copy types: just set()
        format!("self.vars.{}.set({})", var_name, value_expr)
    } else {
        // Non-Copy types: replace with new value
        format!("*self.vars.{}.borrow_mut() = {}", var_name, value_expr)
    }
}

/// Check if a type is Copy
fn is_copy_type(type_str: &str) -> bool {
    matches!(
        type_str,
        "i32" | "i64" | "u32" | "u64" | "f32" | "f64" | "bool" | "char" 
        | "usize" | "isize" | "i8" | "i16" | "u8" | "u16"
    )
}

/// Get default value for a type
fn get_default_value(type_str: &str) -> &'static str {
    match type_str {
        // Numeric types
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => "0",
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => "0",
        "f32" | "f64" => "0.0",
        
        // Boolean
        "bool" => "false",
        
        // String types
        "String" => "String::new()",
        "&str" => "\"\"",
        "char" => "'\\0'",
        
        // Collections
        _ if type_str.starts_with("Vec<") => "Vec::new()",
        _ if type_str.starts_with("HashMap<") => "HashMap::new()",
        _ if type_str.starts_with("HashSet<") => "HashSet::new()",
        
        // Option
        _ if type_str.starts_with("Option<") => "None",
        
        // Default fallback
        _ => "Default::default()",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_type_detection() {
        assert!(is_copy_type("i32"));
        assert!(is_copy_type("f64"));
        assert!(is_copy_type("bool"));
        assert!(!is_copy_type("String"));
        assert!(!is_copy_type("Vec<i32>"));
    }

    #[test]
    fn test_variable_info() {
        let var = VariableInfo::new("health", "i32");
        assert_eq!(var.wrapper_type(), "Cell");
        assert_eq!(var.full_type(), "Arc<Cell<i32>>");
        
        let var2 = VariableInfo::new("name", "String");
        assert_eq!(var2.wrapper_type(), "RefCell");
        assert_eq!(var2.full_type(), "Arc<RefCell<String>>");
    }

    #[test]
    fn test_vars_struct_generation() {
        let mut vars = HashMap::new();
        vars.insert("health".to_string(), "i32".to_string());
        vars.insert("name".to_string(), "String".to_string());
        
        let code = generate_vars_struct(&vars);
        
        assert!(code.contains("pub struct Vars"));
        assert!(code.contains("health: Arc<Cell<i32>>"));
        assert!(code.contains("name: Arc<RefCell<String>>"));
        assert!(code.contains("impl Vars"));
        assert!(code.contains("pub fn new()"));
    }

    #[test]
    fn test_getter_generation() {
        let getter = generate_getter_code("health", "i32");
        assert_eq!(getter, "self.vars.health.get()");
        
        let getter2 = generate_getter_code("name", "String");
        assert_eq!(getter2, "self.vars.name.borrow().clone()");
    }

    #[test]
    fn test_setter_generation() {
        let setter = generate_setter_code("health", "i32", "100");
        assert_eq!(setter, "self.vars.health.set(100)");
        
        let setter2 = generate_setter_code("name", "String", "new_name");
        assert_eq!(setter2, "*self.vars.name.borrow_mut() = new_name");
    }
}
