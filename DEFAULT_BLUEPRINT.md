# Default Blueprint: Math & Control Flow Example

## Overview

The default `blueprint.json` demonstrates all key features of the Pulsar blueprint compiler:
- **Event nodes** (entry points)
- **Pure nodes** (data flow with dependency resolution)
- **Control flow nodes** (branching execution)
- **Function nodes** (side effects)

## Visual Graph

```
                     [Pure Nodes - Pre-evaluated]

    add_node              multiply_node           equals_node
    ┌────────┐           ┌────────┐              ┌────────┐
    │  add   │           │multiply│              │ equals │
    │ a: 2   │──result──→│   a: ◄─┼──result────→│  a: ◄──┤
    │ b: 3   │           │   b: 4 │              │  b: 20 │
    └────────┘           └────────┘              └────────┘
                                                       │
                                                       │ result (bool)
                                                       ▼

    [Event Node]                [Control Flow]

    main_event                  branch_node
    ┌────────┐                  ┌──────────┐
    │  main  │                  │  branch  │
    │        │                  │          │
    │ [Body]─┼──────exec──────→ │ [exec]   │
    └────────┘                  │ cond: ◄──┤ (from equals_node)
                                │          │
                                │ [True]───┼──┐
                                │ [False]──┼──┼─┐
                                └──────────┘  │ │
                                              │ │
                [Function Nodes]              │ │
                                              │ │
           print_true              print_false│ │
           ┌─────────────┐        ┌──────────┼─┘
           │print_string │        │print_str │
       ┌──│ [exec]      │    ┌───│ [exec]   │
       │  │ message: ✓  │    │   │ message: │
       └─►│             │    └──►│    ✗     │
          └─────────────┘        └──────────┘
```

## Node Details

### 1. Event Node: main_event
- **Type**: Event
- **Node Type**: `main`
- **Purpose**: Defines the entry point `pub fn main()`
- **Outputs**:
  - `Body` (execution) → connected to branch_node

### 2. Pure Nodes (Data Flow Chain)

#### add_node
- **Type**: Pure
- **Node Type**: `add`
- **Inputs**: a=2, b=3 (constants)
- **Output**: result (i64)
- **Calculation**: 2 + 3 = 5

#### multiply_node
- **Type**: Pure
- **Node Type**: `multiply`
- **Inputs**:
  - a ← add_node.result (data connection)
  - b = 4 (constant)
- **Output**: result (i64)
- **Calculation**: 5 * 4 = 20

#### equals_node
- **Type**: Pure
- **Node Type**: `equals`
- **Inputs**:
  - a ← multiply_node.result (data connection)
  - b = 20 (constant)
- **Output**: result (bool)
- **Calculation**: 20 == 20 = true

### 3. Control Flow Node: branch_node
- **Type**: ControlFlow
- **Node Type**: `branch`
- **Inputs**:
  - exec ← main_event.Body (execution connection)
  - condition ← equals_node.result (data connection)
- **Outputs**:
  - `True` (execution) → print_true
  - `False` (execution) → print_false
- **Behavior**: Inlines if/else, substitutes exec_output!() macros

### 4. Function Nodes (Side Effects)

#### print_true
- **Type**: Function
- **Node Type**: `print_string`
- **Inputs**:
  - exec ← branch_node.True (execution connection)
  - message = "✓ Calculation correct! (2+3)*4 = 20" (constant)
- **Behavior**: Prints success message

#### print_false
- **Type**: Function
- **Node Type**: `print_string`
- **Inputs**:
  - exec ← branch_node.False (execution connection)
  - message = "✗ Something went wrong with the calculation!" (constant)
- **Behavior**: Prints error message

## Generated Code

```rust
// Auto-generated code from Pulsar Blueprint
// DO NOT EDIT - Changes will be overwritten

use pulsar_std::*;

pub fn main() {
    // Pure node evaluations
    let node_add_node_result = add(2, 3);
    let node_multiply_node_result = multiply(node_add_node_result, 4);
    let node_equals_node_result = equals(node_multiply_node_result, 20);

    // Execution chain
    if node_equals_node_result {
        print_string ("✓ Calculation correct! (2+3)*4 = 20") ;
    } else {
        print_string ("✗ Something went wrong with the calculation!") ;
    }
}
```

## Compilation Process Breakdown

### Phase 1: Metadata Extraction
```
Loaded 30 node definitions from pulsar_std
- main: Event (1 exec output: Body)
- add: Pure (2 inputs, 1 output)
- multiply: Pure (2 inputs, 1 output)
- equals: Pure (2 inputs, 1 output)
- branch: ControlFlow (1 exec in, 2 exec outs: True/False)
- print_string: Function (1 exec in, 1 exec out, 1 data in)
```

### Phase 2: Data Flow Resolution
```
Pure evaluation order (topological sort):
1. add_node (no dependencies)
2. multiply_node (depends on add_node)
3. equals_node (depends on multiply_node)

Input sources:
- add_node.a = Constant(2)
- add_node.b = Constant(3)
- multiply_node.a = Connection(add_node.result)
- multiply_node.b = Constant(4)
- equals_node.a = Connection(multiply_node.result)
- equals_node.b = Constant(20)
- branch_node.condition = Connection(equals_node.result)
```

### Phase 3: Execution Routing
```
Execution routes:
- (main_event, Body) → [branch_node]
- (branch_node, True) → [print_true]
- (branch_node, False) → [print_false]
```

### Phase 4: Code Generation

1. **Event Function Signature**
   ```rust
   pub fn main() {
   ```

2. **Pure Node Pre-evaluation**
   ```rust
   let node_add_node_result = add(2, 3);
   let node_multiply_node_result = multiply(node_add_node_result, 4);
   let node_equals_node_result = equals(node_multiply_node_result, 20);
   ```

3. **Control Flow Inlining**
   - Parse `branch` function body: `if condition { exec_output!("True"); } else { exec_output!("False"); }`
   - Substitute parameters: `condition` → `node_equals_node_result`
   - Replace `exec_output!("True")` → `print_string("✓ ...")`
   - Replace `exec_output!("False")` → `print_string("✗ ...")`
   - Inline result:
   ```rust
   if node_equals_node_result {
       print_string("✓ ...");
   } else {
       print_string("✗ ...");
   }
   ```

## What This Demonstrates

### ✅ All Node Types
- **Event**: Entry point definition (main)
- **Pure**: Data flow without side effects (add, multiply, equals)
- **ControlFlow**: Branching execution (branch)
- **Function**: Side effects (print_string)

### ✅ Data Dependencies
- Pure nodes evaluated in correct order
- Values flow from add → multiply → equals
- Results stored in variables for reuse

### ✅ Control Flow
- Branch node inlines if/else structure
- exec_output!() macros replaced with actual code
- Preserves Rust control flow semantics

### ✅ Execution Flow
- Event node defines entry point
- Execution flows from Body → branch → prints
- Multiple execution paths (True/False branches)

### ✅ Type Safety
- i64 values through math operations
- bool result from comparison
- String messages to print

## Expected Output

When compiled and run:
```
✓ Calculation correct! (2+3)*4 = 20
```

## Validation

The default blueprint passes all validation checks:
- ✓ Header comment
- ✓ use statement
- ✓ pub fn main()
- ✓ Pure node evaluations
- ✓ add function call
- ✓ multiply function call
- ✓ equals function call
- ✓ branch control flow
- ✓ print_string calls
- ✓ Exactly 1 main function
- ✓ Control flow structure (if/else)

---

*This blueprint serves as both a working example and a test case for the compiler's capabilities.*
