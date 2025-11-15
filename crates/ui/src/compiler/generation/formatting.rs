//! # Code Formatting
//!
//! Formats generated Rust code for readability.
//!
//! The compiler can optionally format generated code using:
//! - prettyplease - For AST-based pretty printing
//! - rustfmt - For production-quality formatting (if available)
//!
//! ## Example
//!
//! Unformatted:
//! ```rust,ignore
//! pub fn main(){if true{print_string("Hi".to_string());}else{}}
//! ```
//!
//! Formatted:
//! ```rust,ignore
//! pub fn main() {
//!     if true {
//!         print_string("Hi".to_string());
//!     } else {
//!     }
//! }
//! ```

// This module's functionality is currently in ast_utils.rs (format_rust_code)
// Future refactoring can extract it here

pub use crate::compiler::utils::ast_transform::format_rust_code;
