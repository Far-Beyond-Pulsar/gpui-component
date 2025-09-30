# Proper Graph Compiler Design - Unreal Blueprint Model

## Understanding the Execution Model

### Template Types

After analyzing the templates, there are 3 distinct types:

#### 1. Pure Expressions (Data Flow Only)
```rust
// add.tron
@[in_a_number]@ + @[in_b_number]@
```
- No function wrapper
- Just Rust expressions
- Used for data calculations
- Never part of execution flow

#### 2. Simple Executable Nodes (No Exec Placeholders)
```rust
// print_string.tron
fn @[pulsar_node_fn_id]@() {
    println!("[DEBUG] {}", @[in_message_string]@);
}

// thread_park.tron
fn @[pulsar_node_fn_id]@() {
    std::thread::park();
}
```
- Have function wrapper
- Have statements/side effects
- **NO exec placeholders** (`@[pulsar_exec_*]@`)
- Can be called as functions OR inlined

#### 3. Control Flow Nodes (Have Exec Placeholders)
```rust
// branch.tron
fn @[pulsar_node_fn_id]@() {
    if @[in_condition_bool]@ {
        @[pulsar_exec_a]@
    } else {
        @[pulsar_exec_b]@
    }
}

// thread_spawn.tron
fn @[pulsar_node_fn_id]@() -> std::thread::JoinHandle<()> {
    let handle = std::thread::spawn(|| {
        @[pulsar_exec_body]@
    });
    @[pulsar_exec_continue]@
    handle
}
```
- Have function wrapper
- Have exec placeholders
- **MUST be inlined**, never called as functions
- Exec placeholders filled with nodes connected to those output pins

## How Unreal Blueprints Work (Our Model)

1. **Entry Points**: Begin Play, On Tick, etc. become top-level functions (main, on_tick)
2. **Execution Flow**: White exec pins show the flow of control
3. **Inline Expansion**: Nodes with exec placeholders are expanded inline
4. **Execution Output Routing**: Each exec output pin routes to different code

### Example 1: Linear Chain
```
BeginPlay -> print_string("A") -> print_string("B")
```

Generates:
```rust
fn main() {
    print_string("A");
    print_string("B");
}
```

### Example 2: Branch
```
BeginPlay -> branch(condition)
             ├─true─> print_string("true")
             └─false─> print_string("false")
```

Generates:
```rust
fn main() {
    // Inline branch template
    if condition {
        print_string("true");
    } else {
        print_string("false");
    }
}
```

### Example 3: Thread Spawn (The Critical Case)
```
BeginPlay -> thread_spawn -> print_string("after spawn")
             └─body─> thread_park
```

Generates:
```rust
fn main() {
    // Inline thread_spawn template
    let handle = std::thread::spawn(|| {
        // Fill pulsar_exec_body with nodes connected to "body" pin
        thread_park();
    });
    // Fill pulsar_exec_continue with nodes connected to "continue" pin
    print_string("after spawn");

    handle
}
```

## Compiler Architecture

### Phase 1: Template Analysis
Analyze each template to determine its type:

```rust
enum TemplateType {
    /// Pure expression like "a + b"
    PureExpression,

    /// Function with no exec placeholders - can be called
    SimpleFunction,

    /// Has exec placeholders - must be inlined
    ControlFlow {
        exec_placeholders: Vec<String>, // e.g., ["pulsar_exec_body", "pulsar_exec_continue"]
    },
}

fn analyze_template(template_content: &str) -> TemplateType {
    if !template_content.contains("fn ") {
        return TemplateType::PureExpression;
    }

    let exec_placeholders: Vec<String> = // find all @[pulsar_exec_*]@

    if exec_placeholders.is_empty() {
        TemplateType::SimpleFunction
    } else {
        TemplateType::ControlFlow { exec_placeholders }
    }
}
```

### Phase 2: Build Execution Routing Table
```rust
struct ExecutionRouting {
    // (source_node_id, output_pin_name) -> Vec<target_node_ids>
    routes: HashMap<(String, String), Vec<String>>,
}

impl ExecutionRouting {
    fn get_nodes_for_pin(&self, node_id: &str, pin_name: &str) -> &[String] {
        self.routes.get(&(node_id.to_string(), pin_name.to_string()))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}
```

### Phase 3: Generate Entry Points
For each entry point (BeginPlay, OnTick, etc.):

```rust
fn compile_entry_point(
    entry_node: &NodeInstance,
    graph: &GraphDescription,
    routing: &ExecutionRouting,
) -> Result<String, String> {
    let function_name = match entry_node.node_type.as_str() {
        "begin_play" => "main",
        "on_tick" => "on_tick",
        other => other,
    };

    let mut body = String::new();

    // Find first exec output and follow it
    let first_exec_out = get_first_exec_output(&entry_node);
    let connected_nodes = routing.get_nodes_for_pin(&entry_node.id, &first_exec_out);

    for node_id in connected_nodes {
        let node = &graph.nodes[node_id];
        compile_node_inline(node, graph, routing, &mut body, 1)?;
    }

    Ok(format!("fn {}() {{\n{}}}", function_name, body))
}
```

### Phase 4: Recursive Node Compilation (THE CORE)

```rust
fn compile_node_inline(
    node: &NodeInstance,
    graph: &GraphDescription,
    routing: &ExecutionRouting,
    output: &mut String,
    indent_level: usize,
) -> Result<(), String> {
    let template_type = get_template_type(&node.node_type)?;
    let indent = "    ".repeat(indent_level);

    match template_type {
        TemplateType::PureExpression => {
            // This shouldn't be in execution flow
            Err("Pure expressions can't be in exec flow")
        }

        TemplateType::SimpleFunction => {
            // Just call the function
            let args = get_args(node, graph)?;
            output.push_str(&format!("{}{}({});\n", indent, node.node_type, args));

            // Follow the single exec output (if any)
            if let Some(exec_out) = get_first_exec_output(node) {
                let connected = routing.get_nodes_for_pin(&node.id, &exec_out);
                for next_id in connected {
                    compile_node_inline(&graph.nodes[next_id], graph, routing, output, indent_level)?;
                }
            }
        }

        TemplateType::ControlFlow { exec_placeholders } => {
            // INLINE the template
            let template = get_template(&node.node_type)?;

            // Fill all placeholders
            let mut vars = HashMap::new();

            // Set inputs
            for input in get_inputs(node) {
                let var_name = format!("in_{}_{}", input.name, input.type);
                let value = get_input_value(node, &input.name, graph)?;
                vars.insert(var_name, value);
            }

            // CRITICAL: Fill exec placeholders
            for placeholder in exec_placeholders {
                // Extract pin name from placeholder: "pulsar_exec_body" -> "body"
                let pin_name = placeholder.strip_prefix("pulsar_exec_").unwrap();

                // Get nodes connected to THIS specific exec output
                let connected = routing.get_nodes_for_pin(&node.id, pin_name);

                if connected.is_empty() {
                    // No connections - empty block
                    vars.insert(placeholder, "{}".to_string());
                } else {
                    // Recursively compile connected nodes
                    let mut exec_body = String::new();
                    for next_id in connected {
                        compile_node_inline(
                            &graph.nodes[next_id],
                            graph,
                            routing,
                            &mut exec_body,
                            0, // Reset indent - will be re-indented when inserted
                        )?;
                    }
                    vars.insert(placeholder, exec_body);
                }
            }

            // Render template
            let rendered = render_template(template, vars)?;

            // Strip "fn name() { }" wrapper to get just the body
            let body = extract_function_body(rendered)?;

            // Add to output with proper indentation
            for line in body.lines() {
                output.push_str(&format!("{}{}\n", indent, line));
            }
        }
    }

    Ok(())
}
```

### Key Helper: Extract Function Body

```rust
fn extract_function_body(template_output: &str) -> Result<String, String> {
    // Template has form: "fn name() { BODY }"
    // We want just BODY

    // Find first '{' and last '}'
    let start = template_output.find('{')
        .ok_or("Template has no opening brace")?;
    let end = template_output.rfind('}')
        .ok_or("Template has no closing brace")?;

    // Extract content between braces
    let body = &template_output[start+1..end];

    // Clean up indentation
    Ok(body.trim().to_string())
}
```

## What This Fixes

### Before (Broken):
```
BeginPlay -> thread_spawn -> print_string("after")
             └─body─> thread_park
```

Generated:
```rust
fn thread_spawn() -> JoinHandle<()> {
    let handle = std::thread::spawn(|| {
        {}  // Empty!
    });
    {}  // Empty!
    handle
}

fn main() {
    thread_spawn();  // Called as function, doesn't follow exec properly
    print_string("after");
}
```

### After (Correct):
```rust
fn main() {
    // thread_spawn INLINED
    let handle = std::thread::spawn(|| {
        // pulsar_exec_body filled with nodes connected to "body" pin
        thread_park();
    });
    // pulsar_exec_continue filled with nodes connected to "continue" pin
    print_string("after");

    handle
}
```

## Implementation Strategy

1. **Delete current compiler** - too broken to fix incrementally
2. **Implement template analysis** - classify each template type
3. **Implement execution routing** - track pin connections
4. **Implement recursive inline compiler** - the core algorithm
5. **Test incrementally**:
   - Simple chain (BeginPlay -> print -> print)
   - Branch (BeginPlay -> branch -> prints)
   - Thread spawn (the critical test case)
   - Nested control flow (thread spawn with branch inside body)

## Critical Principles

1. **Never generate functions for control flow nodes** - they must always be inlined
2. **Always respect execution output routing** - each exec pin routes to specific code
3. **Recursive inline expansion** - control flow nodes expand inline, filling their placeholders recursively
4. **Simple nodes can be functions** - nodes without exec placeholders can be called
5. **Preserve Unreal execution semantics** - follow exec pins exactly like blueprints