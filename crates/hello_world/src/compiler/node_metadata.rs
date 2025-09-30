//! # Node Metadata Extraction
//!
//! Extracts metadata from Rust functions decorated with `#[blueprint]` in `pulsar_std`.
//!
//! This module parses the `pulsar_std` source code to extract information about
//! each node: its type (pure/fn/control_flow), parameters, return type, and
//! execution output pins (from `exec_output!()` calls).

use std::collections::HashMap;
use syn::{
    parse_str, Attribute, File, FnArg, Item, ItemFn, Pat, ReturnType, Type,
    visit::Visit, Expr, ExprMacro,
};
use quote::ToTokens;

/// Metadata about a blueprint node function
#[derive(Debug, Clone)]
pub struct NodeMetadata {
    /// Name of the function
    pub name: String,

    /// Type of node (pure, fn, control_flow)
    pub node_type: NodeType,

    /// Function parameters (inputs)
    pub params: Vec<Parameter>,

    /// Return type (if any)
    pub return_type: Option<String>,

    /// Execution input pins (usually just "exec" for non-pure nodes)
    pub exec_inputs: Vec<String>,

    /// Execution output pins (extracted from exec_output!() calls)
    pub exec_outputs: Vec<String>,

    /// The complete function source code for code generation
    pub function_source: String,

    /// Documentation from #[bp_doc] attributes
    pub documentation: Vec<String>,

    /// Category from #[blueprint] attribute
    pub category: Option<String>,

    /// Color from #[blueprint] attribute
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    /// Pure function: no exec pins, only data flow
    Pure,

    /// Simple function: one exec in, one exec out
    Function,

    /// Control flow: one exec in, multiple exec outs via exec_output!()
    ControlFlow,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub ty: String,
}

/// Visitor to find all exec_output!() macro calls in a function
struct ExecOutputVisitor {
    exec_outputs: Vec<String>,
}

impl<'ast> Visit<'ast> for ExecOutputVisitor {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        eprintln!("[DEBUG] Visiting block with {} statements", block.stmts.len());
        syn::visit::visit_block(self, block);
    }

    fn visit_stmt(&mut self, stmt: &'ast syn::Stmt) {
        eprintln!("[DEBUG] Visiting statement");
        syn::visit::visit_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        eprintln!("[DEBUG] Visiting expr: {}", quote::quote!(#expr).to_string().chars().take(50).collect::<String>());
        syn::visit::visit_expr(self, expr);
    }

    fn visit_expr_macro(&mut self, mac: &'ast ExprMacro) {
        eprintln!("[DEBUG] visit_expr_macro called! Path: {:?}", mac.mac.path.get_ident().map(|i| i.to_string()));

        if mac.mac.path.is_ident("exec_output") {
            eprintln!("[DEBUG] Found exec_output! Tokens: {}", mac.mac.tokens);
            // Parse the label: exec_output!("True")
            if let Ok(label) = syn::parse2::<syn::LitStr>(mac.mac.tokens.clone()) {
                eprintln!("[DEBUG] Parsed label: {}", label.value());
                self.exec_outputs.push(label.value());
            } else {
                eprintln!("[DEBUG] Failed to parse label");
            }
        }
        // Continue visiting nested expressions
        syn::visit::visit_expr_macro(self, mac);
    }
}

/// Parse all blueprint functions from pulsar_std source
pub fn extract_node_metadata() -> Result<HashMap<String, NodeMetadata>, String> {
    // For now, we'll use include_str! to embed the source at compile time
    // In the future, this could read from pulsar_std dynamically
    let source = include_str!("../../../pulsar_std/src/lib.rs");

    parse_blueprint_functions(source)
}

/// Parse blueprint functions from a Rust source string
pub fn parse_blueprint_functions(source: &str) -> Result<HashMap<String, NodeMetadata>, String> {
    let file = parse_str::<File>(source)
        .map_err(|e| format!("Failed to parse source: {}", e))?;

    let mut metadata = HashMap::new();

    for item in file.items {
        if let Item::Fn(func) = item {
            if let Some(meta) = extract_function_metadata(func)? {
                metadata.insert(meta.name.clone(), meta);
            }
        }
    }

    Ok(metadata)
}

/// Extract metadata from a single function if it has #[blueprint] attribute
fn extract_function_metadata(func: ItemFn) -> Result<Option<NodeMetadata>, String> {
    // Check if function has #[blueprint] attribute
    let blueprint_attr = func.attrs.iter().find(|attr| {
        attr.path().is_ident("blueprint")
    });

    if blueprint_attr.is_none() {
        return Ok(None);
    }

    let name = func.sig.ident.to_string();

    // Parse blueprint attribute to get node type and other properties
    let (node_type, category, color) = parse_blueprint_attribute(blueprint_attr.unwrap())?;

    // Extract documentation from #[bp_doc] attributes
    let documentation = extract_bp_doc_attributes(&func.attrs);

    // Extract parameters
    let params = extract_parameters(&func.sig.inputs)?;

    // Extract return type
    let return_type = extract_return_type(&func.sig.output);

    // Find all exec_output!() calls in function body
    let exec_outputs = find_exec_outputs(&func);

    // Determine exec inputs based on node type
    let exec_inputs = if node_type == NodeType::Pure {
        vec![]
    } else {
        vec!["exec".to_string()]
    };

    // If node has exec_output calls, it's control flow
    let final_node_type = if !exec_outputs.is_empty() {
        NodeType::ControlFlow
    } else {
        node_type
    };

    // Add implicit exec_out for simple function nodes
    let final_exec_outputs = if final_node_type == NodeType::Function && exec_outputs.is_empty() {
        vec!["exec_out".to_string()]
    } else {
        exec_outputs
    };

    // Convert function to source string
    let function_source = quote::quote!(#func).to_string();

    Ok(Some(NodeMetadata {
        name,
        node_type: final_node_type,
        params,
        return_type,
        exec_inputs,
        exec_outputs: final_exec_outputs,
        function_source,
        documentation,
        category,
        color,
    }))
}

/// Parse #[blueprint(...)] attribute to extract node type and properties
fn parse_blueprint_attribute(attr: &Attribute) -> Result<(NodeType, Option<String>, Option<String>), String> {
    let mut node_type = NodeType::Function; // Default
    let mut category = None;
    let mut color = None;

    // Parse attribute meta - in syn 2.0, we parse the tokens directly
    // #[blueprint(type: NodeTypes::pure, category: "Math", color: "#ff0000")]

    // Simple string-based parsing for now (syn 2.0 changed the API)
    let tokens_str = attr.meta.to_token_stream().to_string();

    if tokens_str.contains("NodeTypes :: pure") || tokens_str.contains("NodeTypes::pure") {
        node_type = NodeType::Pure;
    } else if tokens_str.contains("NodeTypes :: control_flow") || tokens_str.contains("NodeTypes::control_flow") {
        node_type = NodeType::ControlFlow;
    }

    // Extract category and color using simple pattern matching
    if let Some(cat_start) = tokens_str.find("category") {
        if let Some(quote_start) = tokens_str[cat_start..].find('"') {
            let quote_start = cat_start + quote_start + 1;
            if let Some(quote_end) = tokens_str[quote_start..].find('"') {
                category = Some(tokens_str[quote_start..quote_start + quote_end].to_string());
            }
        }
    }

    if let Some(col_start) = tokens_str.find("color") {
        if let Some(quote_start) = tokens_str[col_start..].find('"') {
            let quote_start = col_start + quote_start + 1;
            if let Some(quote_end) = tokens_str[quote_start..].find('"') {
                color = Some(tokens_str[quote_start..quote_start + quote_end].to_string());
            }
        }
    }

    Ok((node_type, category, color))
}

/// Extract #[bp_doc(...)] attributes as documentation
fn extract_bp_doc_attributes(attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("bp_doc"))
        .filter_map(|attr| {
            attr.parse_args::<syn::LitStr>().ok().map(|lit| lit.value())
        })
        .collect()
}

/// Extract function parameters as typed inputs
fn extract_parameters(inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>) -> Result<Vec<Parameter>, String> {
    let mut params = Vec::new();

    for arg in inputs {
        if let FnArg::Typed(pat_type) = arg {
            let name = match &*pat_type.pat {
                Pat::Ident(ident) => ident.ident.to_string(),
                _ => return Err("Unsupported parameter pattern".to_string()),
            };

            let ty = match &*pat_type.ty {
                Type::Path(type_path) => {
                    // Convert type to string
                    quote::quote!(#type_path).to_string()
                }
                _ => return Err(format!("Unsupported type for parameter {}", name)),
            };

            params.push(Parameter { name, ty });
        }
    }

    Ok(params)
}

/// Extract return type as a string
fn extract_return_type(output: &ReturnType) -> Option<String> {
    match output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => Some(quote::quote!(#ty).to_string()),
    }
}

/// Find all exec_output!() macro calls in a function
fn find_exec_outputs(func: &ItemFn) -> Vec<String> {
    let mut visitor = ExecOutputVisitor {
        exec_outputs: Vec::new(),
    };

    // DEBUG: Print the function to see what we're parsing
    eprintln!("[DEBUG] Finding exec_outputs in function: {}", func.sig.ident);
    eprintln!("[DEBUG] Function body: {}", quote::quote!(#func));

    visitor.visit_item_fn(func);

    eprintln!("[DEBUG] Found {} exec_outputs: {:?}", visitor.exec_outputs.len(), visitor.exec_outputs);

    // Remove duplicates while preserving order
    let mut seen = std::collections::HashSet::new();
    visitor
        .exec_outputs
        .into_iter()
        .filter(|name| seen.insert(name.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pure_node() {
        let source = r#"
            #[blueprint(type: NodeTypes::pure)]
            fn add(a: i64, b: i64) -> i64 {
                a + b
            }
        "#;

        let metadata = parse_blueprint_functions(source).unwrap();
        let add_meta = metadata.get("add").unwrap();

        assert_eq!(add_meta.node_type, NodeType::Pure);
        assert_eq!(add_meta.params.len(), 2);
        assert_eq!(add_meta.params[0].name, "a");
        assert!(add_meta.return_type.is_some());
        assert!(add_meta.exec_inputs.is_empty());
        assert!(add_meta.exec_outputs.is_empty());
    }

    #[test]
    fn test_parse_control_flow_node() {
        let source = r#"
            #[blueprint(type: NodeTypes::control_flow)]
            fn branch(thing: bool) {
                if thing {
                   exec_output!("True");
                } else {
                   exec_output!("False");
               }
            }
        "#;

        let metadata = parse_blueprint_functions(source).unwrap();
        let branch_meta = metadata.get("branch").unwrap();

        assert_eq!(branch_meta.node_type, NodeType::ControlFlow);
        assert_eq!(branch_meta.params.len(), 1);
        assert_eq!(branch_meta.params[0].name, "thing");
        assert_eq!(branch_meta.exec_outputs.len(), 2);
        assert!(branch_meta.exec_outputs.contains(&"True".to_string()));
        assert!(branch_meta.exec_outputs.contains(&"False".to_string()));
    }
}
