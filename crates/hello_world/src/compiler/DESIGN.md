# Macro-Based Node Graph Compiler Design

## Overview

This compiler transforms visual node graphs from the Blueprint Editor into executable Rust code. Unlike the old template-based system that used `.tron` files, the new system works with Rust functions defined via `#[blueprint]` attribute macros in the `pulsar_std` crate.

## Philosophy: From Templates to Function Calls

**Old System (.tron templates):**
- Nodes defined as template files with placeholders
- Compiler substitutes values into placeholders
- Example: `@[in_a_number]@ + @[in_b_number]@`

**New System (Rust macros):**
- Nodes defined as actual Rust functions
- Compiler generates **function calls** to these nodes
- Example: `add(value1, value2)`

## Node Types

The `#[blueprint]` macro supports three node types, each requiring different code generation strategies:

### 1. Pure Nodes (`NodeTypes::pure`)
- **Definition**: Functions that take inputs and return outputs, no side effects
- **No exec pins**: These nodes are only part of data flow, never execution flow
- **Example**:
  ```rust
  #[blueprint(type: NodeTypes::pure)]
  fn add(a: i64, b: i64) -> i64 {
      a + b
  }
  ```
- **Code Generation**: Direct function call, result used inline
  ```rust
  let result = add(value1, value2);
  ```

### 2. Simple Function Nodes (`NodeTypes::fn`)
- **Definition**: Functions with side effects, single execution flow
- **One exec input, one exec output**: Linear execution chain
- **Example**:
  ```rust
  #[blueprint(type: NodeTypes::fn)]
  fn print_string(message: String) {
      println!("[DEBUG] {}", message);
  }
  ```
- **Code Generation**: Function call, then follow exec chain
  ```rust
  print_string(msg);
  // Next node in exec chain continues here
  ```

### 3. Control Flow Nodes (`NodeTypes::control_flow`)
- **Definition**: Functions that branch execution flow using `exec_output!()` macro
- **Multiple exec outputs**: Each `exec_output!()` connects to different nodes
- **Example**:
  ```rust
  #[blueprint(type: NodeTypes::control_flow)]
  fn branch(thing: bool) {
      if thing {
         exec_output!("True");
      } else {
         exec_output!("False");
   }
  }
  ```
- **Code Generation**: **Must be inlined**, replacing `exec_output!()` with connected code
  ```rust
  if condition {
      // Code from nodes connected to "True" pin
      print_string("It's true!");
  } else {
      // Code from nodes connected to "False" pin
      print_string("It's false!");
  }
  ```

## Execution Model: Following the Blueprint Wire

The compiler follows the **Unreal Engine Blueprint execution model**:

1. **Entry Points**: Special nodes like `begin_play` become top-level functions (`main()`)
2. **Execution Flow**: White exec pins define the flow of control
3. **Data Flow**: Colored data pins carry values between nodes
4. **Inline Expansion**: Control flow nodes are expanded inline at their usage site
5. **Execution Routing**: Each exec output pin routes to specific connected nodes

### Example: Thread Spawn

```text
BeginPlay -> thread_spawn -> print("after spawn")
             └─body─> print("in thread")
```

Compiles to:
```rust
fn main() {
    let handle = std::thread::spawn(|| {
        // Code from nodes connected to "body" pin
        println!("[DEBUG] {}", "in thread");
    });
    // Code from nodes connected to "continue" pin
    println!("[DEBUG] {}", "after spawn");
    handle
}
```

## Compiler Architecture

### Phase 1: Node Metadata Extraction
**Module**: `node_metadata.rs`

Since nodes are now defined as Rust functions in `pulsar_std`, we need to extract metadata:

**Option A: Parse `pulsar_std` source code** (Recommended)
- Use `syn` crate to parse Rust code
- Extract all functions with `#[blueprint]` attribute
- Build metadata: function name, parameters, return type, node type
- Identify `exec_output!()` calls to determine exec pins

**Option B: Macro-generated registry**
- Have the `#[blueprint]` macro generate a registry at compile time
- Look up node metadata from this registry

**Metadata Structure:**
```rust
pub struct NodeMetadata {
    pub name: String,
    pub node_type: NodeType, // pure, fn, control_flow
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
    pub exec_inputs: Vec<String>,  // Usually just ["exec"]
    pub exec_outputs: Vec<String>, // e.g., ["True", "False"] for branch
}

pub enum NodeType {
    Pure,          // No exec pins, pure function
    Function,      // One exec in, one exec out
    ControlFlow,   // One exec in, multiple exec outs
}
```

### Phase 2: Execution Routing
**Module**: `execution_router.rs`

**KEEP THIS MODULE** - It's already well-designed in the current compiler.

Maps execution connections: `(node_id, output_pin_name) -> Vec<target_node_ids>`

This tells us which nodes to compile when filling each `exec_output!()` call.

### Phase 3: Data Flow Resolution
**Module**: `data_resolver.rs` (NEW)

Resolves where each input value comes from:
- Connected to an output from another node?
- A constant/property value?
- A default value?

**Responsibilities:**
1. Build data dependency graph
2. Determine evaluation order for pure nodes
3. Generate variable names for intermediate results
4. Handle type conversions if needed

**Example:**
```text
add_node(a: const 5, b: from multiply_node.result)
```
Resolves to:
```rust
let multiply_result = multiply(x, y);
let add_result = add(5, multiply_result);
```

### Phase 4: Code Generation
**Module**: `code_generator.rs`

The core compilation logic. Different strategies per node type:

#### A. Pure Node Code Generation
```rust
fn generate_pure_node(node: &NodeInstance, metadata: &NodeMetadata) -> String {
    let args = collect_argument_values(node);
    let result_var = format!("node_{}_result", node.id);
    format!("let {} = {}({});", result_var, metadata.name, args.join(", "))
}
```

#### B. Function Node Code Generation
```rust
fn generate_function_node(node: &NodeInstance, metadata: &NodeMetadata) -> String {
    let args = collect_argument_values(node);
    let mut code = format!("{}({});", metadata.name, args.join(", "));

    // Follow exec chain
    if let Some(next) = get_next_exec_node(node) {
        code.push_str(&compile_node_inline(next));
    }

    code
}
```

#### C. Control Flow Node Inlining
```rust
fn inline_control_flow_node(
    node: &NodeInstance,
    metadata: &NodeMetadata,
    routing: &ExecutionRouter,
) -> Result<String, String> {
    // 1. Parse the function body from pulsar_std source
    let source_body = get_function_source(metadata.name)?;

    // 2. Find all exec_output!() macro calls
    let exec_calls = extract_exec_output_calls(&source_body)?;

    // 3. For each exec_output!(), get connected nodes
    let mut replacements = HashMap::new();
    for (exec_name, _position) in exec_calls {
        let connected_nodes = routing.get_connected_nodes(&node.id, &exec_name);

        // Recursively compile connected nodes
        let mut exec_code = String::new();
        for target_id in connected_nodes {
            exec_code.push_str(&compile_node_inline(target_id)?);
        }

        replacements.insert(exec_name, exec_code);
    }

    // 4. Replace exec_output!() calls with actual code
    let inlined = substitute_exec_outputs(source_body, replacements)?;

    // 5. Substitute input parameters with actual values
    let args = collect_argument_values(node);
    let final_code = substitute_parameters(inlined, metadata.params, args)?;

    Ok(final_code)
}
```

**Key Challenge**: Parsing and transforming Rust code
- Need to parse function bodies from `pulsar_std`
- Find and replace `exec_output!()` macro invocations
- Maintain proper syntax and indentation

**Solution**: Use `syn` and `quote` crates:
```rust
use syn::{parse_str, visit::Visit, Expr, Macro};

// Parse function to find exec_output!() calls
struct ExecOutputVisitor {
    exec_calls: Vec<(String, Span)>,
}

impl<'ast> Visit<'ast> for ExecOutputVisitor {
    fn visit_macro(&mut self, mac: &'ast Macro) {
        if mac.path.is_ident("exec_output") {
            // Extract the label: exec_output!("True")
            let label = parse_exec_output_label(mac);
            self.exec_calls.push((label, mac.span()));
        }
    }
}
```

### Phase 5: Entry Point Generation
**Module**: `entry_points.rs`

**KEEP AND ADAPT** from current compiler.

1. Find entry point nodes (begin_play, on_tick, etc.)
2. Generate appropriate function signatures
3. Compile the execution chain starting from each entry point

```rust
pub fn generate_entry_point(
    entry_node: &NodeInstance,
    graph: &GraphDescription,
) -> Result<String, String> {
    let fn_name = match entry_node.node_type.as_str() {
        "begin_play" => "main",
        "on_tick" => "on_tick",
        _ => &entry_node.node_type,
    };

    let mut body = String::new();
    compile_execution_chain(entry_node, graph, &mut body)?;

    Ok(format!("fn {}() {{\n{}}}", fn_name, body))
}
```

## Critical Implementation Details

### 1. Function Body Extraction

**Challenge**: Get the actual source code of functions from `pulsar_std`

**Solution**:
- At compile time, use `include_str!()` to embed `pulsar_std/src/lib.rs`
- Parse with `syn` to extract all function bodies
- Cache in a `HashMap<String, syn::ItemFn>`

```rust
lazy_static! {
    static ref NODE_FUNCTIONS: HashMap<String, syn::ItemFn> = {
        let source = include_str!("../../../pulsar_std/src/lib.rs");
        parse_all_blueprint_functions(source)
    };
}
```

### 2. exec_output!() Substitution

**Challenge**: Replace `exec_output!("Label")` with actual node code

**Solution**:
- Parse function body to AST
- Use `syn::visit_mut` to find and transform macro calls
- Replace macro with a block containing the compiled node code

```rust
impl VisitMut for ExecOutputReplacer {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Macro(mac) = expr {
            if mac.mac.path.is_ident("exec_output") {
                let label = parse_label(&mac.mac);
                let replacement_code = self.replacements.get(&label);
                *expr = parse_str(replacement_code).unwrap();
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }
}
```

### 3. Type Safety and Validation

- Validate connections: Check that output types match input types
- Use `TypeInfo` system already in place in `graph/type_system.rs`
- Report errors with node IDs and pin names for easy debugging

### 4. Error Handling

All compilation errors should include:
- Node ID and type
- Input/output pin names
- Expected vs actual types
- Graph location (for editor highlighting)

```rust
pub enum CompilerError {
    TypeMismatch {
        node_id: String,
        expected: TypeInfo,
        found: TypeInfo,
    },
    MissingConnection {
        node_id: String,
        pin_name: String,
    },
    CyclicDependency {
        cycle: Vec<String>,
    },
    NodeNotFound {
        node_id: String,
    },
}
```

## Module Organization

```
compiler/
├── mod.rs                 # Main entry point, orchestrates compilation
├── DESIGN.md              # This file
├── node_metadata.rs       # Extract metadata from pulsar_std
├── execution_router.rs    # Maps exec connections (KEEP from current)
├── data_resolver.rs       # Resolves data dependencies and evaluation order
├── code_generator.rs      # Main code generation logic
├── entry_points.rs        # Generate entry point functions
├── ast_utils.rs           # Helpers for parsing/transforming Rust AST
├── type_checker.rs        # Validate types and connections
└── errors.rs              # Error types and reporting
```

## Migration Strategy

1. **Keep working compiler**: Don't delete the old one immediately
2. **Parallel implementation**: Build new compiler alongside old one
3. **Feature flag**: Use feature flags to switch between old and new
4. **Incremental testing**: Test each node type independently
5. **Gradual transition**: Migrate nodes one category at a time

## Testing Strategy

### Unit Tests
- Parse individual node functions from `pulsar_std`
- Test exec_output!() extraction
- Test code generation for each node type
- Test data resolution

### Integration Tests
Test graphs of increasing complexity:

1. **Simple Chain**: `begin_play -> print_string`
2. **Pure Expression**: `add(5, 3) -> print_string`
3. **Branch**: `begin_play -> branch(true) -> [print_true, print_false]`
4. **Thread Spawn**: The critical test case from DESIGN.md
5. **Nested Control Flow**: Branch inside thread spawn body
6. **Complex Graph**: Multiple control flow nodes, data dependencies

### Test Graph Format
```rust
#[test]
fn test_simple_chain() {
    let graph = create_test_graph(|g| {
        let begin = g.add_node("begin_play");
        let print = g.add_node("print_string");
        print.set_input("message", "Hello!");
        g.connect_exec(begin, print);
    });

    let code = compile(graph).unwrap();
    assert!(code.contains("fn main()"));
    assert!(code.contains("println!(\"[DEBUG] {}\""));
}
```

## Future Enhancements

1. **Optimization Pass**: Eliminate dead code, constant folding
2. **Async/Await Support**: For async nodes
3. **Hot Reload**: Recompile and reload without restart
4. **Debug Info**: Generate debug symbols for breakpoints in generated code
5. **Visual Debugging**: Map generated code back to graph positions
6. **Incremental Compilation**: Only recompile changed subgraphs

## Success Criteria

The new compiler is successful when:

1. ✅ All node types (pure, fn, control_flow) generate correct code
2. ✅ Execution flow exactly matches Blueprint semantics
3. ✅ Data dependencies are resolved correctly
4. ✅ Control flow nodes are properly inlined
5. ✅ Thread spawn test case works perfectly
6. ✅ Generated code compiles without errors
7. ✅ Generated code executes with correct behavior
8. ✅ Compilation errors are clear and actionable
9. ✅ Performance is acceptable for large graphs
10. ✅ Easy to add new nodes by just adding functions to `pulsar_std`

## Example: Complete Compilation

### Input Graph
```text
BeginPlay
  └─exec─> Branch(condition: true)
            ├─True─> Print("Branch taken!")
            └─False─> Print("Branch not taken!")
```

### Generated Code
```rust
fn main() {
    // BeginPlay node - entry point

    // Branch node - inlined control flow
    if true {
        // Nodes connected to "True" pin
        println!("[DEBUG] {}", "Branch taken!");
    } else {
        // Nodes connected to "False" pin
        println!("[DEBUG] {}", "Branch not taken!");
    }
}
```

## Key Principles

1. **Trust the Type System**: Use Rust's type system to validate graphs
2. **Follow the Wires**: Exec pins dictate control flow, data pins dictate data flow
3. **Inline Control Flow**: Never call control flow nodes as functions
4. **Generate Readable Code**: Humans should be able to read and debug output
5. **Fail Loudly**: Better to error with details than generate wrong code
6. **Preserve Semantics**: Generated code must behave exactly as the graph implies
