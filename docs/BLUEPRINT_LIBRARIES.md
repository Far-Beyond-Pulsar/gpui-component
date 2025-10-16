# Blueprint Sub-Graph Libraries System

## Overview

The Blueprint Libraries system allows you to create reusable sub-graphs (collapsed graphs/macros) that can be packaged into JSON-based libraries. These libraries are automatically loaded and made available in the node creation menu.

## Architecture

### Core Components

1. **Sub-Graph Definition** (`SubGraphDefinition`)
   - Represents a single reusable sub-graph
   - Contains internal graph structure and custom interface pins
   - Can be nested within other sub-graphs

2. **Sub-Graph Library** (`SubGraphLibrary`)
   - A collection of related sub-graph definitions
   - Stored as JSON files in the libraries directory
   - Categorized for easy organization

3. **Library Manager** (`LibraryManager`)
   - Loads and caches all libraries from search paths
   - Provides quick lookup for sub-graphs by ID
   - Manages library categories

4. **Sub-Graph Expander** (`SubGraphExpander`)
   - Recursively expands sub-graph instances before compilation
   - Handles nested sub-graphs
   - Detects circular references

## Directory Structure

```
project/
├── libraries/
│   ├── std/                    # Standard library (ships with Pulsar)
│   │   ├── math.json          # Math utilities
│   │   ├── logic.json         # Logic utilities
│   │   └── flow_control.json  # Control flow macros
│   └── user/                   # User-created libraries
│       ├── custom.json
│       └── ...
└── blueprints/
    ├── main_graph.json
    └── ui_graph.json
```

## Creating a Sub-Graph Library

### Library JSON Structure

```json
{
  "id": "my_library",
  "name": "My Custom Library",
  "version": "1.0.0",
  "description": "Description of the library",
  "author": "Your Name",
  "category": "custom",
  "subgraphs": [
    {
      "id": "my_macro",
      "name": "My Macro",
      "description": "What this macro does",
      "graph": {
        "nodes": { ... },
        "connections": [ ... ],
        "metadata": { ... },
        "comments": []
      },
      "interface": {
        "inputs": [
          {
            "id": "input1",
            "name": "Input 1",
            "data_type": { "Typed": { ... } },
            "description": "Description of input"
          }
        ],
        "outputs": [
          {
            "id": "output1",
            "name": "Output 1",
            "data_type": { "Typed": { ... } },
            "description": "Description of output"
          }
        ]
      },
      "metadata": {
        "created_at": "2025-01-01T00:00:00Z",
        "modified_at": "2025-01-01T00:00:00Z",
        "author": "Your Name",
        "tags": ["tag1", "tag2"]
      }
    }
  ],
  "metadata": {
    "created_at": "2025-01-01T00:00:00Z",
    "modified_at": "2025-01-01T00:00:00Z",
    "tags": ["library-tag"],
    "icon": "icon-name"
  }
}
```

### Sub-Graph Interface

Each sub-graph has a customizable interface with:
- **Input Pins**: Can include any number of execution and data pins
- **Output Pins**: Can include any number of execution and data pins
- These pins propagate to special nodes inside the sub-graph:
  - `subgraph_input` node with matching output pins
  - `subgraph_output` node with matching input pins

### Internal Structure

Inside a sub-graph:
1. **Input Node** (`subgraph_input`):
   - Auto-generated with output pins matching interface inputs
   - Represents the entry point for data/execution
   - Green color in UI (entry point)

2. **Output Node** (`subgraph_output`):
   - Auto-generated with input pins matching interface outputs
   - Represents the exit point for data/execution
   - Red color in UI (exit point)

3. **Internal Nodes**:
   - Any regular blueprint nodes
   - Can include other sub-graphs (nesting)

## Using Sub-Graphs

### In the Blueprint Editor

1. **Adding a Sub-Graph**:
   - Open node creation menu
   - Navigate to library category
   - Select the sub-graph macro
   - Place in editor like any other node

2. **Editing Interface** (when editing a sub-graph definition):
   - Select sub-graph definition
   - Open details pane
   - Use Interface Editor to add/remove/edit pins
   - Changes propagate to internal Input/Output nodes

3. **Navigating Sub-Graphs**:
   - Double-click a sub-graph instance to open in new tab
   - Tab bar shows breadcrumb navigation
   - Close tab to return to parent

### Programmatically

```rust
use crate::graph::{LibraryManager, SubGraphLibrary, SubGraphDefinition};

// Create library manager
let mut lib_manager = LibraryManager::default();

// Load all libraries from default paths
lib_manager.load_all_libraries().unwrap();

// Get a specific sub-graph
let subgraph = lib_manager.get_subgraph("clamp").unwrap();

// Create an instance
let instance = subgraph.create_instance("my_clamp", Position { x: 100.0, y: 200.0 });

// Add to graph
graph.add_node(instance);
```

## Compilation Process

### Normal Compilation
```rust
use crate::compiler::compile_graph;

let code = compile_graph(&graph)?;
```

### With Library Support
```rust
use crate::compiler::compile_graph_with_library_manager;
use crate::graph::LibraryManager;

let mut lib_manager = LibraryManager::default();
lib_manager.load_all_libraries()?;

let code = compile_graph_with_library_manager(&graph, Some(lib_manager))?;
```

### Expansion Pipeline

1. **Pre-processing**: Load all libraries
2. **Circular Reference Detection**: Validate sub-graph dependencies
3. **Recursive Expansion** (innermost first):
   - Find all sub-graph instances
   - For each instance:
     - Clone internal nodes with prefixed IDs
     - Map external connections to internal Input/Output nodes
     - Remove instance node
   - Repeat until no sub-graphs remain
4. **Normal Compilation**: Compile expanded graph

## Example: Standard Math Library

The standard math library (`libraries/std/math.json`) includes:

### Clamp Macro
- **Inputs**: `Value`, `Min`, `Max` (all `f64`)
- **Output**: `Result` (`f64`)
- **Function**: Clamps value between min and max
- **Implementation**: Internally uses `math.max` and `math.min` nodes

### Linear Interpolate (Lerp) Macro
- **Inputs**: `A`, `B`, `T` (all `f64`)
- **Output**: `Result` (`f64`)
- **Function**: Linear interpolation: `A + (B - A) * T`

## Creating Your Own Library

### Step 1: Create Sub-Graph in Editor
1. Create a new blueprint
2. Build your logic with regular nodes
3. Add `subgraph_input` and `subgraph_output` nodes
4. Define interface pins in details pane
5. Test the logic

### Step 2: Export to Library
1. Use "Export as Macro" from editor
2. Specify library category and metadata
3. Save to `libraries/user/your_library.json`

### Step 3: Use in Other Graphs
1. Restart editor or reload libraries
2. Macro appears in node menu under your category
3. Place and use like any other node

## API Reference

### SubGraphDefinition
```rust
impl SubGraphDefinition {
    pub fn new(id: &str, name: &str) -> Self;
    pub fn add_input_pin(&mut self, id: &str, name: &str, data_type: DataType);
    pub fn add_output_pin(&mut self, id: &str, name: &str, data_type: DataType);
    pub fn sync_interface_nodes(&mut self);
    pub fn create_instance(&self, instance_id: &str, position: Position) -> NodeInstance;
}
```

### SubGraphLibrary
```rust
impl SubGraphLibrary {
    pub fn new(id: &str, name: &str, category: &str) -> Self;
    pub fn add_subgraph(&mut self, subgraph: SubGraphDefinition);
    pub fn get_subgraph(&self, id: &str) -> Option<&SubGraphDefinition>;
    pub fn save_to_file(&self, path: &Path) -> Result<(), Box<dyn Error>>;
    pub fn load_from_file(path: &Path) -> Result<Self, Box<dyn Error>>;
}
```

### LibraryManager
```rust
impl LibraryManager {
    pub fn new() -> Self;
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>);
    pub fn load_all_libraries(&mut self) -> Result<(), Box<dyn Error>>;
    pub fn register_library(&mut self, library: SubGraphLibrary);
    pub fn get_subgraph(&self, id: &str) -> Option<&SubGraphDefinition>;
    pub fn get_all_subgraphs(&self) -> Vec<&SubGraphDefinition>;
    pub fn get_subgraphs_by_category(&self, category: &str) -> Vec<&SubGraphDefinition>;
}
```

### SubGraphExpander
```rust
impl SubGraphExpander {
    pub fn new(library_manager: LibraryManager) -> Self;
    pub fn expand_all(&self, graph: &mut GraphDescription) -> Result<(), String>;
    pub fn validate_no_circular_refs(&self, definition: &SubGraphDefinition) -> Result<(), String>;
}
```

## Advanced Features

### Nested Sub-Graphs
Sub-graphs can contain other sub-graphs. The expander handles this recursively:
```
Graph A
  └─ Contains Sub-Graph B
      └─ Contains Sub-Graph C
```
Expansion order: C → B → A

### Thread Safety
When a sub-graph contains a "Spawn Thread" node:
1. Compiler analyzes data flow from inputs to thread
2. Validates all captured variables are `Clone + Send + Sync`
3. Generates appropriate synchronization wrappers

### Version Management
Libraries support semantic versioning:
- Breaking changes: Major version bump
- New features: Minor version bump
- Bug fixes: Patch version bump

Future: Multiple versions can coexist with explicit version selection in graph.

## Best Practices

1. **Keep Sub-Graphs Focused**: Each macro should do one thing well
2. **Document Interfaces**: Use descriptions for all pins
3. **Test Independently**: Test sub-graphs before using in larger graphs
4. **Avoid Deep Nesting**: Limit nesting depth to 3-4 levels
5. **Use Meaningful Names**: Clear names for sub-graphs and pins
6. **Categorize Properly**: Use appropriate categories for organization

## Troubleshooting

### Library Not Loading
- Check JSON syntax with a validator
- Ensure file is in correct directory (`libraries/std/` or `libraries/user/`)
- Check console for error messages
- Verify all referenced node types exist

### Circular Reference Error
- Check sub-graph dependencies
- Ensure no sub-graph references itself (directly or indirectly)
- Use `validate_no_circular_refs` to debug

### Compilation Errors After Expansion
- Verify internal nodes are valid
- Check pin connections match types
- Ensure Input/Output nodes match interface
- Test sub-graph independently

### Performance Issues
- Limit number of nodes per sub-graph
- Avoid excessive nesting
- Use profiling to identify bottlenecks
- Consider optimizing frequently-used macros

## Future Enhancements

- [ ] Visual sub-graph editor with interface designer
- [ ] Library browser with search and filtering
- [ ] Auto-collapse selected nodes into sub-graph
- [ ] Sub-graph versioning and dependency management
- [ ] Import/export libraries for sharing
- [ ] Hot-reload libraries without restart
- [ ] Sub-graph templates and wizards
- [ ] Performance metrics for macros

## See Also

- [SUBGRAPH_DESIGN.md](SUBGRAPH_DESIGN.md) - Detailed design documentation
- Blueprint Editor Documentation
- Compiler Documentation
- Node System Documentation
