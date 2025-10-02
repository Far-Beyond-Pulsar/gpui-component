//! # Pulsar Blueprint Macros
//!
//! Procedural macros for defining blueprint nodes in Rust.
//!
//! ## Macros
//!
//! - `#[blueprint]` - Mark a function as a blueprint node and auto-register it
//! - `exec_output!()` - Define execution output points in control flow nodes

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, Pat, ReturnType, Type, FnArg, Stmt, Expr};

/// Mark a function as a blueprint node and automatically register it.
///
/// # Attributes
///
/// - `type`: Node type - `NodeTypes::pure`, `NodeTypes::fn_`, `NodeTypes::control_flow`, or `NodeTypes::event`
/// - `color`: Optional hex color for the node in the UI (e.g., `"#ff0000"`)
/// - `category`: Optional category for grouping nodes (e.g., `"Math"`)
///
/// # Examples
///
/// ## Pure Node
/// ```ignore
/// #[blueprint(type: NodeTypes::pure, category: "Math")]
/// fn add(a: i64, b: i64) -> i64 {
///     a + b
/// }
/// ```
///
/// ## Function Node
/// ```ignore
/// #[blueprint(type: NodeTypes::fn_, category: "Debug")]
/// fn print_string(message: String) {
///     println!("[DEBUG] {}", message);
/// }
/// ```
///
/// ## Control Flow Node
/// ```ignore
/// #[blueprint(type: NodeTypes::control_flow, category: "Flow")]
/// fn branch(condition: bool) {
///     if condition {
///         exec_output!("True");
///     } else {
///         exec_output!("False");
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn blueprint(args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    let args_str = args.to_string();

    // Extract function information
    let fn_name = &input.sig.ident;
    let fn_name_str = fn_name.to_string();

    // Parse node type
    let node_type_str = if args_str.contains("NodeTypes :: pure") || args_str.contains("NodeTypes::pure") {
        "Pure"
    } else if args_str.contains("NodeTypes :: fn_") || args_str.contains("NodeTypes::fn_") {
        "Function"
    } else if args_str.contains("NodeTypes :: control_flow") || args_str.contains("NodeTypes::control_flow") {
        "ControlFlow"
    } else if args_str.contains("NodeTypes :: event") || args_str.contains("NodeTypes::event") {
        "Event"
    } else {
        "Function" // Default
    };

    // Extract category
    let category = extract_string_value(&args_str, "category");
    let category_str = category.unwrap_or_else(|| "General".to_string());

    // Extract color
    let color = extract_string_value(&args_str, "color");
    let color_opt = if let Some(c) = color {
        quote! { Some(#c) }
    } else {
        quote! { None }
    };

    // Extract parameters
    let params: Vec<_> = input.sig.inputs.iter().filter_map(|arg| {
        if let FnArg::Typed(pat_type) = arg {
            if let Pat::Ident(ident) = &*pat_type.pat {
                let param_name = ident.ident.to_string();
                let ty = &*pat_type.ty;
                let param_type = quote!(#ty).to_string();
                return Some(quote! {
                    crate::NodeParameter {
                        name: #param_name,
                        ty: #param_type,
                    }
                });
            }
        }
        None
    }).collect();

    // Extract return type
    let return_type = match &input.sig.output {
        ReturnType::Default => quote! { None },
        ReturnType::Type(_, ty) => {
            let ty_str = quote!(#ty).to_string();
            quote! { Some(#ty_str) }
        }
    };

    // Find exec_output calls
    let exec_outputs = find_exec_output_labels(&input);
    let exec_outputs_array = if exec_outputs.is_empty() {
        quote! { &[] }
    } else {
        quote! { &[#(#exec_outputs),*] }
    };

    // Determine exec inputs based on node type
    let exec_inputs = match node_type_str {
        "Pure" | "Event" => quote! { &[] },
        _ => quote! { &["exec"] },
    };

    // Build documentation from doc comments (/// or #[doc = "..."])
    let docs: Vec<String> = input.attrs.iter()
        .filter_map(|attr| {
            // Doc comments become #[doc = "..."] attributes
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Lit(expr_lit) = &nv.value {
                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                            return Some(lit_str.value().trim().to_string());
                        }
                    }
                }
            }
            None
        })
        .collect();

    let docs_array = if docs.is_empty() {
        quote! { &[] }
    } else {
        quote! { &[#(#docs),*] }
    };

    // Convert function source to string for storage
    let fn_source = quote!(#input).to_string();

    // Generate the registration const
    let registry_ident = syn::Ident::new(
        &format!("__BLUEPRINT_NODE__{}", fn_name_str.to_uppercase()),
        fn_name.span()
    );

    let node_type_ident = syn::Ident::new(node_type_str, fn_name.span());

    let expanded = quote! {
        #[allow(dead_code)]
        #input

        #[::linkme::distributed_slice(crate::BLUEPRINT_REGISTRY)]
        #[linkme(crate = ::linkme)]
        static #registry_ident: crate::NodeMetadata = crate::NodeMetadata {
            name: #fn_name_str,
            node_type: crate::NodeType::#node_type_ident,
            params: &[#(#params),*],
            return_type: #return_type,
            exec_inputs: #exec_inputs,
            exec_outputs: #exec_outputs_array,
            function_source: #fn_source,
            documentation: #docs_array,
            category: #category_str,
            color: #color_opt,
        };
    };

    TokenStream::from(expanded)
}

/// Extract a string value from an attribute string like `category: "Math"`
fn extract_string_value(attr_str: &str, key: &str) -> Option<String> {
    if let Some(key_pos) = attr_str.find(key) {
        if let Some(quote_start) = attr_str[key_pos..].find('"') {
            let quote_start = key_pos + quote_start + 1;
            if let Some(quote_end) = attr_str[quote_start..].find('"') {
                return Some(attr_str[quote_start..quote_start + quote_end].to_string());
            }
        }
    }
    None
}

/// Find all exec_output!() labels in a function
fn find_exec_output_labels(func: &ItemFn) -> Vec<String> {
    let mut labels = Vec::new();
    find_exec_in_block(&func.block, &mut labels);

    // Remove duplicates while preserving order
    let mut seen = std::collections::HashSet::new();
    labels.into_iter().filter(|l| seen.insert(l.clone())).collect()
}

fn find_exec_in_block(block: &syn::Block, labels: &mut Vec<String>) {
    for stmt in &block.stmts {
        match stmt {
            Stmt::Expr(expr, _) => find_exec_in_expr(expr, labels),
            Stmt::Macro(stmt_macro) => {
                if stmt_macro.mac.path.is_ident("exec_output") {
                    if let Ok(label) = syn::parse2::<syn::LitStr>(stmt_macro.mac.tokens.clone()) {
                        labels.push(label.value());
                    }
                }
            }
            _ => {}
        }
    }
}

fn find_exec_in_expr(expr: &Expr, labels: &mut Vec<String>) {
    match expr {
        Expr::Macro(macro_expr) => {
            if macro_expr.mac.path.is_ident("exec_output") {
                if let Ok(label) = syn::parse2::<syn::LitStr>(macro_expr.mac.tokens.clone()) {
                    labels.push(label.value());
                }
            }
        }
        Expr::Block(block_expr) => find_exec_in_block(&block_expr.block, labels),
        Expr::If(if_expr) => {
            find_exec_in_block(&if_expr.then_branch, labels);
            if let Some((_, else_branch)) = &if_expr.else_branch {
                find_exec_in_expr(else_branch, labels);
            }
        }
        Expr::Match(match_expr) => {
            for arm in &match_expr.arms {
                find_exec_in_expr(&arm.body, labels);
            }
        }
        Expr::Loop(loop_expr) => find_exec_in_block(&loop_expr.body, labels),
        Expr::ForLoop(for_expr) => find_exec_in_block(&for_expr.body, labels),
        Expr::While(while_expr) => find_exec_in_block(&while_expr.body, labels),
        Expr::Unsafe(unsafe_expr) => find_exec_in_block(&unsafe_expr.block, labels),
        _ => {}
    }
}

/// Mark an execution output point in a control flow node.
///
/// This macro is a marker that gets replaced by the compiler during code generation.
/// It should only be used inside functions marked with `#[blueprint(type: NodeTypes::control_flow)]`.
///
/// # Arguments
///
/// - `label`: String literal identifying this execution output (e.g., `"True"`, `"False"`, `"Body"`)
///
/// # Examples
///
/// ```ignore
/// #[blueprint(type: NodeTypes::control_flow)]
/// fn branch(condition: bool) {
///     if condition {
///         exec_output!("True");  // Nodes connected to "True" pin execute here
///     } else {
///         exec_output!("False"); // Nodes connected to "False" pin execute here
///     }
/// }
/// ```
#[proc_macro]
pub fn exec_output(input: TokenStream) -> TokenStream {
    let _label = parse_macro_input!(input as syn::LitStr);

    // At runtime, this expands to nothing
    // The compiler will replace it during code generation
    let expanded = quote! {
        ()
    };

    TokenStream::from(expanded)
}

// Note: bp_doc is no longer a proc-macro. It's just a marker attribute
// that the #[blueprint] macro looks for. This way it doesn't get consumed
// before blueprint can see it.
