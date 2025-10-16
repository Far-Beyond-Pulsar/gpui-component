# Sub-Graph System Design

## Overview
This document describes the design for sub-graphs (collapsed graphs/macros) in the blueprint editor, allowing users to create reusable node groups with custom interfaces.

## Data Model

### 1. SubGraphDefinition
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGraphDefinition {
    /// Unique identifier for this sub-graph type
    pub id: String,

    /// Display name
    pub name: String,

    /// Description for documentation
    pub description: String,

    /// The internal graph structure
    pub graph: GraphDescription,

    /// Custom interface pins defined by the user
    pub interface: SubGraphInterface,

    /// Metadata
    pub metadata: SubGraphMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGraphInterface {
    /// Custom input pins (can include both exec and data pins)
    pub inputs: Vec<SubGraphPin>,

    /// Custom output pins (can include both exec and data pins)
    pub outputs: Vec<SubGraphPin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGraphPin {
    /// Pin identifier (used for connections)
    pub id: String,

    /// Display name
    pub name: String,

    /// Pin data type (Execution or Typed)
    pub data_type: DataType,

    /// Optional description
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGraphMetadata {
    pub created_at: String,
    pub modified_at: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
}
```

### 2. Special Internal Nodes

Inside a sub-graph, special interface nodes represent the entry/exit points:

```rust
// Node type: "subgraph_input"
// Automatically generated based on SubGraphInterface.inputs
// Has output pins matching the interface's input pins

// Node type: "subgraph_output"
// Automatically generated based on SubGraphInterface.outputs
// Has input pins matching the interface's output pins
```

### 3. Sub-Graph Instance Node

When a sub-graph is placed in a graph, it becomes a regular node:

```rust
NodeInstance {
    id: "node_123",
    node_type: "subgraph:my_custom_macro", // Format: "subgraph:{sub_graph_id}"
    position: Position { x: 100.0, y: 200.0 },
    properties: {
        "subgraph_id": PropertyValue::String("my_custom_macro".to_string()),
    },
    // Pins are dynamically generated from SubGraphInterface
    inputs: vec![...],
    outputs: vec![...],
}
```

## Pin Customization Flow

### In the Details Pane:
When a sub-graph definition is selected, the details pane shows:

1. **Interface Editor Section**
   - "Input Pins" list with add/remove/edit buttons
   - "Output Pins" list with add/remove/edit buttons

2. **Pin Configuration Dialog** (when adding/editing a pin):
   - Pin Name (text field)
   - Pin Type (dropdown: Execution, Data)
   - Data Type (if Data pin selected - type selector with all available types)
   - Description (optional text field)

### Pin Propagation:
1. When interface pins are modified, the internal Input/Output nodes are automatically updated
2. When a sub-graph instance is placed, its pins are generated from the interface definition
3. When interface changes, all instances in other graphs are updated

## Compiler Behavior

### Sub-Graph Expansion Algorithm:

1. **Pre-processing Phase**:
   - Load all sub-graph definitions
   - Build dependency graph (detect circular references)
   - Validate all sub-graph instances reference valid definitions

2. **Expansion Phase** (for each sub-graph instance):
   ```rust
   fn expand_subgraph(
       instance: &NodeInstance,
       definition: &SubGraphDefinition,
       parent_graph: &mut GraphDescription,
   ) -> Result<(), CompilerError> {
       // 1. Create unique node IDs for all nodes in the sub-graph
       //    Format: "{instance_id}__{internal_node_id}"

       // 2. Clone all internal nodes with new IDs

       // 3. Map interface connections:
       //    - Connections to instance's input pins → connections to internal "subgraph_input" node outputs
       //    - Connections from internal "subgraph_output" node inputs → connections from instance's output pins

       // 4. Replace instance node with expanded internal nodes

       // 5. Update all connections

       // 6. Remove the original instance node

       Ok(())
   }
   ```

3. **Recursive Expansion**:
   - Sub-graphs can contain other sub-graphs
   - Expand in topological order (innermost first)
   - Track expansion depth to prevent infinite recursion

4. **Post-expansion**:
   - Validate expanded graph (no dangling connections)
   - Continue with normal compilation

## Storage Structure

```
project/
├── blueprints/
│   ├── main_graph.json          # Main blueprint graphs
│   └── ui_graph.json
└── subgraphs/
    ├── custom_macro_1.json      # Sub-graph definitions
    ├── utility_functions.json
    └── state_machines.json
```

Each sub-graph file contains a `SubGraphDefinition` serialized as JSON.

## UI Implementation

### 1. Sub-Graph Editor Tab System
- Top bar shows breadcrumb navigation: "Main Graph > My Macro"
- Tabs for each open graph/sub-graph
- Double-click sub-graph instance to open in new tab
- Close tab to return to parent

### 2. Details Pane - Interface Editor
When sub-graph definition is selected:

```
┌─────────────────────────────────┐
│ Interface                       │
├─────────────────────────────────┤
│ Input Pins:                     │
│ ┌─────────────────────────────┐ │
│ │ [▶] Execute     (exec)      │ │
│ │ [●] Value       (f64)       │ │
│ │ [●] Target      (String)    │ │
│ └─────────────────────────────┘ │
│ [+ Add Input Pin]               │
│                                 │
│ Output Pins:                    │
│ ┌─────────────────────────────┐ │
│ │ [▶] Then        (exec)      │ │
│ │ [●] Result      (bool)      │ │
│ └─────────────────────────────┘ │
│ [+ Add Output Pin]              │
└─────────────────────────────────┘
```

### 3. Internal Interface Nodes
Inside a sub-graph, special nodes appear:

- **Input Node** (auto-generated, cannot be deleted):
  - Has output pins matching interface inputs
  - Green color to indicate entry point

- **Output Node** (auto-generated, cannot be deleted):
  - Has input pins matching interface outputs
  - Red color to indicate exit point

## Thread Safety for Spawn Thread Nodes

When a sub-graph contains a "Spawn Thread" node, variable filtering must be applied:

1. **Analysis Phase**:
   - Detect all "Spawn Thread" nodes in sub-graphs
   - Trace data flow from sub-graph inputs to thread spawn
   - Identify which input variables are used in the thread

2. **Validation**:
   - Ensure all variables used in thread are thread-safe (Clone + Send + Sync)
   - Warn/error if non-thread-safe types are used

3. **Code Generation**:
   - Only pass required variables to thread
   - Generate appropriate Arc/Mutex wrappers if needed

## Examples

### Example 1: Simple Utility Macro

**Interface**:
- Inputs: `[exec] Execute`, `[f64] A`, `[f64] B`
- Outputs: `[exec] Then`, `[f64] Sum`, `[f64] Product`

**Internal Graph**:
- Input node with outputs: Execute, A, B
- Math nodes computing sum and product
- Output node with inputs: Then, Sum, Product

**Usage**:
- Place "Calculate Sum and Product" node
- Connect input values
- Get both sum and product as outputs

### Example 2: State Machine Macro

**Interface**:
- Inputs: `[exec] Execute`, `[String] Event`
- Outputs: `[exec] On Enter State A`, `[exec] On Enter State B`, `[exec] On Exit`

**Internal Graph**:
- Complex state transition logic
- Multiple execution paths
- Internal state storage

### Example 3: Nested Sub-Graphs

Graph A contains Graph B, which contains Graph C:
1. Expand C within B first
2. Then expand B within A
3. Finally expand A in main graph

## API Extensions

### New Compiler Functions:
```rust
impl BlueprintCompiler {
    /// Load all sub-graph definitions from project
    fn load_subgraph_definitions(&mut self, project_path: &Path) -> Result<(), Error>;

    /// Expand all sub-graph instances recursively
    fn expand_subgraphs(&mut self, graph: &mut GraphDescription) -> Result<(), Error>;

    /// Validate sub-graph interface matches internal nodes
    fn validate_subgraph_interface(&self, definition: &SubGraphDefinition) -> Result<(), Error>;
}
```

### New UI Functions:
```rust
impl BlueprintEditor {
    /// Open sub-graph in new tab
    fn open_subgraph_tab(&mut self, subgraph_id: String, cx: &mut Context);

    /// Create new sub-graph from selected nodes (collapse)
    fn collapse_selection_to_subgraph(&mut self, cx: &mut Context);

    /// Update interface editor based on selected sub-graph
    fn refresh_interface_editor(&mut self, cx: &mut Context);
}
```

## Migration Path

1. **Phase 1**: Implement data structures and serialization
2. **Phase 2**: Implement compiler expansion algorithm
3. **Phase 3**: Add UI for creating/editing sub-graphs
4. **Phase 4**: Add tab system for navigation
5. **Phase 5**: Add interface editor in details pane
6. **Phase 6**: Add thread safety validation
7. **Phase 7**: Polish and testing

## Open Questions

1. Should sub-graphs support local variables?
2. How to handle sub-graph versioning?
3. Should there be a "macro library" browser?
4. How to visualize which sub-graphs are used where?
