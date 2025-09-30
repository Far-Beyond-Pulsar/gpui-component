//! Quick test to debug the visitor

use syn::{parse_str, ItemFn, visit::Visit, ExprMacro};

struct ExecOutputVisitor {
    exec_outputs: Vec<String>,
    debug_count: usize,
}

impl<'ast> Visit<'ast> for ExecOutputVisitor {
    fn visit_expr_macro(&mut self, mac: &'ast ExprMacro) {
        self.debug_count += 1;
        println!("Found macro: {}", quote::quote!(#mac));

        if mac.mac.path.is_ident("exec_output") {
            println!("  -> It's an exec_output!");
            if let Ok(label) = syn::parse2::<syn::LitStr>(mac.mac.tokens.clone()) {
                println!("  -> Label: {}", label.value());
                self.exec_outputs.push(label.value());
            }
        }
        syn::visit::visit_expr_macro(self, mac);
    }
}

fn main() {
    let source = r#"
        fn branch(thing: bool) {
            if thing {
               exec_output!("True");
            } else {
               exec_output!("False");
           }
        }
    "#;

    let func: ItemFn = parse_str(source).unwrap();

    let mut visitor = ExecOutputVisitor {
        exec_outputs: Vec::new(),
        debug_count: 0,
    };

    visitor.visit_item_fn(&func);

    println!("\nTotal macros found: {}", visitor.debug_count);
    println!("Exec outputs found: {:?}", visitor.exec_outputs);
}
