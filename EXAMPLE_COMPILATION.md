# Example Blueprint Compilation

This document demonstrates how the Pulsar Blueprint compiler transforms a visual node graph into executable Rust code.

## Example: Hello World Program

### Visual Node Graph

```
┌─────────────────┐
│   main (Event)  │
│                 │
│  [Body] ────────┼──┐
└─────────────────┘  │
                     │
                     │ (execution flow)
                     │
                     ▼
              ┌──────────────────┐
              │  print_string    │
              │    (Function)    │
  [exec] ─────┤                  │
              │  message: String │
              │    = "Hello      │
              │       World"     │
              │                  │
              │  [exec_out] ──►  │
              └──────────────────┘
```

### Node Details

#### 1. Main Event Node
- **Type**: Event (defines entry point)
- **Function**: `main`
- **Outputs**:
  - `Body` (execution) - where the program execution begins

#### 2. Print String Node
- **Type**: Function (side-effectful operation)
- **Function**: `print_string`
- **Inputs**:
  - `exec` (execution) - connected from main's Body
  - `message` (String) - constant value "Hello World"
- **Outputs**:
  - `exec_out` (execution) - continues execution chain

### Connections

| From Node | From Pin | To Node       | To Pin  | Type      |
|-----------|----------|---------------|---------|-----------|
| main_1    | Body     | print_1       | exec    | Execution |
| (none)    | -        | print_1       | message | Data (constant: "Hello World") |

### Generated Rust Code

```rust
// Auto-generated code from Pulsar Blueprint
// DO NOT EDIT - Changes will be overwritten

use pulsar_std::*;

pub fn main() {
    // Pure node evaluations

    // Execution chain
    print_string("Hello World");
}
```

### Compilation Process

1. **Metadata Extraction**: Parse `pulsar_std` to find node definitions
   - `main` is identified as an event node (NodeType::Event)
   - `print_string` is identified as a function node (NodeType::Function)

2. **Data Flow Resolution**: Build dependency graph
   - No pure nodes in this graph
   - `print_string.message` uses constant value from properties

3. **Execution Routing**: Map execution connections
   - Route: `(main_1, Body) -> [print_1]`

4. **Code Generation**:
   - Event node `main` defines the outer `pub fn main()` signature
   - Function node `print_string` generates a function call
   - Constant message value is inlined as string literal

## More Complex Example: Branch Control Flow

### Visual Node Graph

```
┌─────────────────┐
│   main (Event)  │
│                 │
│  [Body] ────────┼──┐
└─────────────────┘  │
                     │
                     ▼
              ┌──────────────────┐
              │     branch       │
              │  (Control Flow)  │
  [exec] ─────┤                  │
              │ condition: bool  │
              │    = true        │
              │                  │
              │  [True]  ────────┼──┐
              │  [False] ────────┼──┼──┐
              └──────────────────┘  │  │
                                    │  │
                  ┌─────────────────┘  │
                  │                    │
                  ▼                    ▼
         ┌──────────────┐     ┌──────────────┐
         │print_string  │     │print_string  │
         │   "Yes!"     │     │   "No!"      │
         └──────────────┘     └──────────────┘
```

### Expected Generated Code

```rust
// Auto-generated code from Pulsar Blueprint
// DO NOT EDIT - Changes will be overwritten

use pulsar_std::*;

pub fn main() {
    // Pure node evaluations

    // Execution chain
    // Inline branch control flow
    if true {
        print_string("Yes!");
    } else {
        print_string("No!");
    }
}
```

### How Control Flow Inlining Works

1. **Detection**: `branch` node has `exec_output!()` calls, classified as ControlFlow
2. **Extraction**: Found exec outputs: ["True", "False"]
3. **Routing**: Build routing table:
   - `(branch_1, True) -> [print_yes]`
   - `(branch_1, False) -> [print_no]`
4. **AST Transformation**:
   - Parse `branch` function body into AST
   - Replace `exec_output!("True")` with `print_string("Yes!");`
   - Replace `exec_output!("False")` with `print_string("No!");`
5. **Inlining**: Inject transformed function body directly into output

## Node Type Behavior Summary

| Node Type     | Exec Pins | Compilation Strategy | Example |
|---------------|-----------|---------------------|---------|
| **Pure**      | None      | Evaluated before exec chain, stored in variables | `add(a, b)` |
| **Function**  | In + Out  | Generate function call in exec chain | `print_string(msg)` |
| **ControlFlow** | In + Multiple Outs | Inline function body with exec_output!() substitution | `branch(condition)` |
| **Event**     | Only Outs | Define outer function signature | `main()`, `begin_play()` |

## Key Insights

### Why Events Define Functions
Events like `main` and `begin_play` don't execute *in* the graph - they define the *API* of the compiled code. The execution chain connected to their "Body" output becomes the function body.

### Pure Node Pre-evaluation
Pure nodes (math, logic, conversions) have no side effects, so they're evaluated first in topological order and stored in variables. This allows them to be referenced multiple times without re-evaluation.

### Control Flow Inlining
Control flow nodes like `branch`, `for_loop`, and `switch` use `exec_output!()` macros to mark branching points. During compilation, the compiler replaces these macros with the actual code from connected nodes, preserving the control flow structure.

### Function Nodes in Execution Chain
Function nodes with side effects (I/O, mutations) are called sequentially in the execution chain, following the exec pin connections from the event's Body output.
