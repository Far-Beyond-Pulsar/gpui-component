# Pulsar Engine - Complete Architecture Documentation

## Executive Summary

Pulsar Engine is a comprehensive, modern game engine written in Rust featuring:
- Visual scripting system (Blueprints) with node-based programming
- Full 3D level editor with transform gizmos and scene management
- Integrated code editor with Rust Analyzer LSP support
- Digital Audio Workstation (DAW) for game audio
- Real-time rendering with Bevy integration
- Multi-window support with Winit + GPUI composition
- Hot reload capabilities
- Cross-platform support (Windows, macOS, Linux)

**Total codebase**: 133 Rust files, ~45,000+ lines of code

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Pulsar Engine                        │
│  ┌────────────┐  ┌────────────┐  ┌──────────────────────┐   │
│  │   Window   │  │     UI     │  │      Compiler        │   │
│  │ Management │  │  System    │  │ (Blueprint → Rust)   │   │
│  │   (Winit)  │  │  (GPUI)    │  │                      │   │
│  └──────┬─────┘  └──────┬─────┘  └──────────┬───────────┘   │
│         │                │                   │              │
│         └────────────────┴───────────────────┘              │
│                          │                                  │
│         ┌────────────────┴───────────────────┐              │
│         │       Engine State & Assets        │              │
│         └────────────────┬───────────────────┘              │
│                          │                                  │
│         ┌────────────────┴───────────────────┐              │
│         │         Backend Systems            │              │
│         │  (ECS, Physics, Audio, Networking) │              │
│         └────────────────────────────────────┘              │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
Pulsar-Native/
├── crates/
│   ├── engine/              Main engine crate (frontend, UI, editors)
│   ├── engine_backend/      Backend systems (ECS, physics, networking)
│   ├── ui/                  Reusable UI components library
│   ├── macros/              UI component proc macros
│   ├── pulsar_macros/       Engine-specific proc macros
│   └── pulsar_std/          Standard library for game scripts
├── assets/                  Engine assets (icons, fonts, etc.)
├── themes/                  UI theme definitions
├── libraries/               External libraries
├── docs/                    Documentation
└── tools/                   Development tools
```

## Module Organization (crates/engine/src/)

### Core Systems
```
src/
├── main.rs                  Application entry point (1907 lines → needs refactoring)
├── engine_state.rs          Global engine state management
├── assets.rs                Asset loading and embedding
├── recent_projects.rs       Recent projects tracking
├── themes.rs                Theme system
├── window_manager.rs        Window lifecycle management
├── renderer.rs              Rendering abstractions
└── stdlib_builder.rs        Standard library builder
```

### Backend Integration
```
src/backend/
├── mod.rs                   Backend module exports
├── client/                  Game client networking
│   └── mod.rs
└── server/                  Game server integration
    └── mod.rs
```

### Compiler System (Blueprint Visual Scripting)
```
src/compiler/               (~4,100 lines total)
├── mod.rs                   Compiler orchestration (125 lines)
├── ast_utils.rs             AST manipulation utilities (315 lines)
├── code_generator.rs        Rust code generation (551 lines)
├── data_resolver.rs         Data type resolution (467 lines)
├── execution_routing.rs     Execution flow routing (58 lines)
├── node_metadata.rs         Node metadata handling (72 lines)
├── node_parser.rs           Node parsing logic (328 lines)
├── subgraph_expander.rs     Subgraph expansion (330 lines)
├── type_extractor.rs        Type extraction (76 lines)
├── validate_blueprint.rs    Blueprint validation (108 lines)
├── tests.rs                 Compiler tests (302 lines)
└── test_default_ui_graph.rs Test graphs (157 lines)
```

### Graph System
```
src/graph/                  (~1,531 lines total)
├── mod.rs                   Graph data structures (1200 lines → needs refactoring)
└── type_system.rs           Type system for nodes (331 lines)
```

### Settings
```
src/settings/
├── mod.rs                   Settings module exports
└── engine_settings.rs       Engine configuration (128 lines)
```

### User Interface System
```
src/ui/                     (~31,000+ lines total)
├── mod.rs                   UI module orchestration (23 lines)
├── app.rs                   Main application state (1874 lines → needs refactoring)
├── entry_window.rs          Entry point window (40 lines)
├── loading_window.rs        Loading screen (500 lines)
├── settings_window.rs       Settings window (48 lines)
├── settings_screen.rs       Settings panels (848 lines)
├── project_selector.rs      Project selection (167 lines)
├── command_palette.rs       Command palette (512 lines)
├── menu.rs                  Menu bar (980 lines → needs refactoring)
├── shared.rs                Shared UI utilities (262 lines)
│
├── file_manager_drawer.rs   File browser drawer (1817 lines → needs refactoring)
├── file_manager_window.rs   File manager window (70 lines)
├── file_utils.rs            File utilities (221 lines)
│
├── problems_drawer.rs       Problems panel drawer (470 lines)
├── problems_window.rs       Problems window (46 lines)
│
├── terminal_drawer.rs       Terminal drawer (58 lines)
├── terminal_window.rs       Terminal window (43 lines)
│
├── rust_analyzer_manager.rs Rust Analyzer LSP (1249 lines → needs refactoring)
├── lsp_completion_provider.rs LSP completion (416 lines)
│
└── gpu_renderer.rs          GPU renderer integration (284 lines)
```

### Entry Screen System
```
src/ui/entry_screen/        (~3,000+ lines total)
├── mod.rs                   Entry screen orchestration (637 lines)
├── types.rs                 Data types (82 lines)
├── git_operations.rs        Git integration (147 lines)
├── integration_launcher.rs  Integration launcher (575 lines)
│
└── views/
    ├── mod.rs               View exports (20 lines)
    ├── sidebar.rs           Sidebar navigation (103 lines)
    ├── recent_projects.rs   Recent projects view (335 lines)
    ├── new_project.rs       New project wizard (166 lines)
    ├── clone_git.rs         Git clone view (178 lines)
    ├── templates.rs         Project templates (152 lines)
    ├── upstream_prompt.rs   Upstream prompts (156 lines)
    │
    └── project_settings/    Project settings system
        ├── mod.rs           Settings orchestration (170 lines)
        ├── types.rs         Settings types (497 lines)
        ├── general.rs       General settings (82 lines)
        ├── metadata.rs      Project metadata (369 lines)
        ├── integrations.rs  Integration settings (400 lines)
        ├── performance.rs   Performance settings (386 lines)
        ├── git_info.rs      Git information (110 lines)
        ├── git_ci.rs        CI/CD settings (114 lines)
        ├── disk_info.rs     Disk usage (78 lines)
        └── helpers.rs       Helper functions (83 lines)
```

### Editor Panels
```
src/ui/panels/              (~18,000+ lines total)
├── mod.rs                   Panel exports (10 lines)
│
├── blueprint_editor2/       Visual scripting editor (~6,300 lines)
│   ├── mod.rs               Editor orchestration (643 lines)
│   ├── panel.rs             Main panel UI (3748 lines → NEEDS MAJOR REFACTORING)
│   ├── node_graph.rs        Node graph rendering (1946 lines → needs refactoring)
│   ├── properties.rs        Node properties (640 lines)
│   ├── variables.rs         Variable management (431 lines)
│   ├── toolbar.rs           Editor toolbar (349 lines)
│   ├── file_drawer.rs       File operations (328 lines)
│   ├── node_creation_menu.rs Node creation (452 lines)
│   ├── node_library.rs      Node library (146 lines)
│   ├── hoverable_tooltip.rs Tooltips (147 lines)
│   ├── minimap.rs           Canvas minimap (207 lines)
│   └── macros.rs            Blueprint macros (262 lines)
│
├── level_editor/            3D level editor (~3,200 lines)
│   ├── mod.rs               Editor orchestration (22 lines)
│   ├── gizmos.rs            Transform gizmos (623 lines)
│   ├── scene_database.rs    Scene data (569 lines)
│   │
│   └── ui/                  Level editor UI
│       ├── mod.rs           UI exports (20 lines)
│       ├── panel.rs         Main panel (649 lines)
│       ├── viewport.rs      3D viewport (2050 lines → NEEDS REFACTORING)
│       ├── hierarchy.rs     Scene hierarchy (219 lines)
│       ├── properties.rs    Entity properties (298 lines)
│       ├── asset_browser.rs Asset browser (295 lines)
│       ├── scene_browser.rs Scene browser (169 lines)
│       ├── toolbar.rs       Editor toolbar (210 lines)
│       ├── state.rs         Editor state (296 lines)
│       └── actions.rs       Editor actions (87 lines)
│
├── script_editor/           Code editor with LSP (~3,200 lines)
│   ├── mod.rs               Editor orchestration (229 lines)
│   ├── text_editor.rs       Text editing (1205 lines → needs refactoring)
│   ├── file_explorer.rs     File tree (1001 lines → needs refactoring)
│   ├── terminal.rs          Integrated terminal (589 lines)
│   └── autocomplete_integration.rs LSP autocomplete (143 lines)
│
├── daw_editor/              Digital Audio Workstation (~6,800 lines)
│   ├── mod.rs               DAW orchestration (187 lines)
│   ├── audio_types.rs       Audio data structures (486 lines)
│   ├── audio_graph.rs       Audio routing graph (370 lines)
│   ├── audio_service.rs     Audio processing (333 lines)
│   ├── asset_manager.rs     Audio assets (413 lines)
│   ├── project.rs           DAW projects (299 lines)
│   ├── real_time_audio.rs   Real-time audio (312 lines)
│   ├── gpu_dsp.rs           GPU-accelerated DSP (400 lines)
│   ├── ecs_integration.rs   ECS integration (184 lines)
│   │
│   └── ui/                  DAW UI components
│       ├── mod.rs           UI exports (20 lines)
│       ├── panel.rs         Main DAW panel (762 lines)
│       ├── state.rs         DAW state (595 lines)
│       ├── timeline.rs      Timeline view (860 lines)
│       ├── mixer.rs         Audio mixer (830 lines)
│       ├── track_header.rs  Track headers (289 lines)
│       ├── transport.rs     Transport controls (351 lines)
│       ├── browser.rs       Audio file browser (573 lines)
│       ├── inspector.rs     Property inspector (144 lines)
│       ├── toolbar.rs       DAW toolbar (249 lines)
│       ├── clip_editor.rs   Audio clip editor (21 lines → stub)
│       ├── effects.rs       Effects rack (10 lines → stub)
│       ├── automation.rs    Automation editor (10 lines → stub)
│       └── routing.rs       Audio routing UI (10 lines → stub)
│
└── [Future editors - currently stubs at 82 lines each]
    ├── animation_editor.rs  Animation editor (stub)
    ├── behavior_editor.rs   Behavior tree editor (stub)
    ├── diagram_editor.rs    Diagram editor (stub)
    ├── foliage_editor.rs    Foliage painter (stub)
    ├── material_editor.rs   Material editor (86 lines)
    ├── navmesh_editor.rs    Navigation mesh editor (stub)
    ├── particle_editor.rs   Particle system editor (stub)
    ├── physics_editor.rs    Physics editor (stub)
    ├── prefab_editor.rs     Prefab editor (stub)
    ├── skeleton_editor.rs   Skeleton editor (stub)
    ├── sound_editor.rs      Sound editor (stub)
    ├── terrain_editor.rs    Terrain editor (stub)
    └── ui_editor.rs         UI editor (stub)
```

### Terminal Emulator
```
src/ui/terminal/            (~4,200 lines total)
├── mod.rs                   Terminal module (10 lines)
├── terminal_core.rs         Core terminal logic (961 lines)
├── terminal_element.rs      Terminal UI element (681 lines)
├── terminal_element_zed.rs  Zed-style terminal (2012 lines → NEEDS REFACTORING)
├── rendering.rs             Terminal rendering (374 lines)
│
└── mappings/                Input mappings
    ├── mod.rs               Mapping exports (4 lines)
    ├── keys.rs              Keyboard mappings (449 lines)
    └── mouse.rs             Mouse mappings (1 line → stub)
```

### Editors Module
```
src/ui/editors/
└── mod.rs                   Editor module exports (66 lines)
```

## Technology Stack

### Core Framework
- **Rust** - Systems programming language
- **GPUI** - UI framework (from Zed editor)
- **Winit** - Cross-platform window management
- **Tokio** - Async runtime (8 worker threads)

### Rendering
- **Direct3D 11** (Windows) - Low-level rendering and composition
- **WGPU** - Cross-platform GPU compute and rendering
- **Bevy** - 3D rendering engine (integrated via GpuRenderer)
- Zero-copy texture sharing for efficient composition

### Language Tools & Compilation
- **Syn** - Rust parsing for Blueprint compiler
- **Quote** - Rust code generation
- **Prettyplease** - Code formatting
- **Ropey** - Text rope for efficient text editing
- **LSP-Types** - Language Server Protocol types
- **Rust Analyzer** - Rust language server

### Audio
- **CPAL** - Cross-platform audio I/O
- **Symphonia** - Audio decoding (MP3, WAV, FLAC, Ogg Vorbis)
- **Hound** - WAV encoding/decoding
- **Lewton** - Vorbis decoder
- **Claxon** - FLAC decoder
- GPU-accelerated DSP via WGPU compute shaders

### Terminal
- **Alacritty Terminal** - Professional terminal emulator core
- **Portable PTY** - Pseudo-terminal support
- **Which** - Executable finder

### Utilities
- **Serde** - Serialization/deserialization
- **Anyhow** - Error handling
- **Tracing** - Structured logging
- **DashMap** - Concurrent HashMap
- **Parking Lot** - Fast synchronization primitives
- **Regex** - Regular expressions
- **Chrono** - Date and time
- **Directories** - Standard directories
- **Git2** - Git integration
- **RFD** - File dialogs
- **Open** - Open files/URLs
- **UUID** - Unique identifiers
- **Device Query** - Raw input polling (for viewport controls)
- **Reqwest** - HTTP client (for GitHub API)
- **Sysinfo** - System information

## Key Features & Systems

### 1. Blueprint Visual Scripting System
**Purpose**: Node-based visual programming that compiles to native Rust code

**Components**:
- Node graph editor with pan/zoom (panel.rs, node_graph.rs)
- Node library with search (node_library.rs)
- Property inspector (properties.rs)
- Variable management (variables.rs)
- Function/macro system (macros.rs)
- File operations (file_drawer.rs)
- Canvas minimap (minimap.rs)
- Node creation menu (node_creation_menu.rs)
- Hoverable tooltips (hoverable_tooltip.rs)

**Compiler Pipeline**:
1. Parse node graph (node_parser.rs)
2. Validate connections (validate_blueprint.rs)
3. Expand subgraphs (subgraph_expander.rs)
4. Resolve data types (data_resolver.rs, type_extractor.rs)
5. Generate Rust AST (ast_utils.rs)
6. Route execution flow (execution_routing.rs)
7. Generate final code (code_generator.rs)
8. Hot reload compiled code

**Features**:
- Type-safe node connections
- Execution flow control
- Data flow visualization
- Breakpoint debugging (planned)
- Performance profiling (planned)
- Visual diffs (planned)

### 2. Level Editor (3D Scene Editor)
**Purpose**: Build and edit 3D game levels with real-time preview

**Components**:
- 3D viewport with camera controls (viewport.rs)
- Transform gizmos (translate, rotate, scale) (gizmos.rs)
- Scene hierarchy tree (hierarchy.rs)
- Entity property inspector (properties.rs)
- Asset browser (asset_browser.rs)
- Scene browser (scene_browser.rs)
- Editor toolbar (toolbar.rs)
- Scene database (scene_database.rs)
- Editor actions (actions.rs)
- State management (state.rs)

**Features**:
- Real-time 3D rendering via Bevy
- Multi-selection support
- Undo/redo system
- Prefab instantiation
- Scene serialization
- Asset import
- Lighting preview
- Physics visualization
- Navigation mesh display
- Camera bookmarks

**Integration**:
- Bevy ECS for entity management
- Rapier3D for physics
- Custom gizmo rendering
- GPU-accelerated picking

### 3. Script Editor
**Purpose**: Full-featured code editor with Rust Analyzer integration

**Components**:
- Text editor with syntax highlighting (text_editor.rs)
- File explorer tree (file_explorer.rs)
- Integrated terminal (terminal.rs)
- LSP autocomplete (autocomplete_integration.rs)
- Rust Analyzer manager (rust_analyzer_manager.rs)

**Features**:
- Syntax highlighting via Tree-sitter
- Code completion (Rust Analyzer)
- Go to definition
- Find references
- Inline diagnostics
- Code formatting
- Refactoring support
- Integrated terminal
- Git integration
- Multi-cursor editing

**LSP Integration**:
- Full Rust Analyzer support
- Semantic tokens
- Hover information
- Signature help
- Document symbols
- Workspace symbols

### 4. Digital Audio Workstation (DAW)
**Purpose**: Create and edit game audio with professional tools

**Components**:
- Multi-track timeline (timeline.rs)
- Audio mixer (mixer.rs)
- Track headers (track_header.rs)
- Transport controls (transport.rs)
- Audio file browser (browser.rs)
- Property inspector (inspector.rs)
- Audio graph routing (audio_graph.rs)
- Effects rack (effects.rs - stub)
- Automation editor (automation.rs - stub)
- Clip editor (clip_editor.rs - stub)

**Audio Engine**:
- Real-time audio processing (real_time_audio.rs)
- Audio asset management (asset_manager.rs)
- Audio service layer (audio_service.rs)
- GPU-accelerated DSP (gpu_dsp.rs)
- ECS integration (ecs_integration.rs)
- Project management (project.rs)

**Features**:
- Non-destructive editing
- GPU-accelerated effects (convolution, FFT EQ)
- Spatial audio (3D positioning)
- MIDI support (planned)
- VST plugin support (planned)
- Audio routing graph
- Automation lanes
- Time stretching
- Pitch shifting
- Multi-channel support

**Supported Formats**:
- WAV (encode/decode)
- MP3 (decode)
- FLAC (decode)
- Ogg Vorbis (decode)

### 5. Terminal Emulator
**Purpose**: Professional terminal emulator for development tasks

**Components**:
- Terminal core (terminal_core.rs)
- Terminal UI elements (terminal_element.rs, terminal_element_zed.rs)
- Rendering engine (rendering.rs)
- Input mappings (keys.rs, mouse.rs)

**Features**:
- PTY support via portable-pty
- Alacritty terminal emulation
- ANSI escape sequence support
- Scrollback buffer
- Text selection
- Hyperlink detection
- Multiple terminal tabs
- Split panes (planned)

### 6. Window Management System
**Purpose**: Multi-window support with zero-copy GPU composition

**Architecture**:
- Winit for OS window management
- GPUI for UI rendering
- Direct3D 11 for texture composition (Windows)
- Per-window independent state

**Composition Pipeline** (Windows):
1. Bevy renders 3D content to D3D12 shared texture (bottom layer)
2. GPUI renders UI to D3D11 shared texture (top layer)
3. D3D11 composites both to swap chain back buffer
4. No CPU-GPU data transfers (zero-copy)

**Features**:
- Multiple independent windows
- Per-window GPUI application
- Window type routing (Settings, ProjectEditor, ProjectSplash)
- Window creation via channel system
- Modal dialog support

### 7. Entry Screen System
**Purpose**: Project management and initial setup

**Components**:
- Sidebar navigation (sidebar.rs)
- Recent projects list (recent_projects.rs)
- New project wizard (new_project.rs)
- Git clone interface (clone_git.rs)
- Project templates (templates.rs)
- Upstream prompts (upstream_prompt.rs)
- Git operations (git_operations.rs)
- Integration launcher (integration_launcher.rs)

**Project Settings**:
- General settings (general.rs)
- Metadata editing (metadata.rs)
- Integration configuration (integrations.rs)
- Performance tuning (performance.rs)
- Git information (git_info.rs)
- CI/CD setup (git_ci.rs)
- Disk usage analysis (disk_info.rs)

### 8. Settings System
**Purpose**: Engine and project configuration

**Features**:
- Theme selection
- Font configuration
- Performance settings
- Keybinding customization
- Plugin management
- Integration settings
- Editor preferences
- Project metadata

**Persistence**:
- TOML-based configuration files
- Per-project settings
- Global engine settings
- Settings versioning
- Migration support

### 9. Problem/Diagnostic System
**Purpose**: Display compiler errors, warnings, and diagnostics

**Components**:
- Problems drawer (problems_drawer.rs)
- Problems window (problems_window.rs)
- LSP diagnostic integration

**Features**:
- Error aggregation
- Warning filtering
- Quick fixes
- Jump to location
- Severity levels
- File grouping

### 10. File Management
**Purpose**: File browser and operations

**Components**:
- File manager drawer (file_manager_drawer.rs)
- File manager window (file_manager_window.rs)
- File utilities (file_utils.rs)

**Features**:
- Tree view
- File operations (create, delete, rename, move)
- Context menus
- File previews
- Search/filter
- Recent files

### 11. Command Palette
**Purpose**: Quick access to all engine commands

**Component**: command_palette.rs (512 lines)

**Features**:
- Fuzzy search
- Command categories
- Keyboard shortcuts display
- Recent commands
- Context-aware commands

### 12. Menu System
**Purpose**: Application menu bar

**Component**: menu.rs (980 lines)

**Menus**:
- File (New, Open, Save, Recent, Exit)
- Edit (Undo, Redo, Cut, Copy, Paste, Settings)
- View (Panels, Layouts, Zoom)
- Tools (Commands, Extensions)
- Help (Documentation, About)

## Data Flow

### Blueprint Compilation Flow
```
User creates nodes in Blueprint Editor
        ↓
Node graph serialized to JSON/YAML
        ↓
Compiler parses node graph (node_parser.rs)
        ↓
Validation (validate_blueprint.rs)
        ↓
Type resolution (data_resolver.rs, type_extractor.rs)
        ↓
Subgraph expansion (subgraph_expander.rs)
        ↓
AST generation (ast_utils.rs)
        ↓
Code generation (code_generator.rs)
        ↓
Rust source code output
        ↓
rustc compiles to native code
        ↓
Dynamic loading (planned) or restart with new code
```

### Event Flow
```
OS Window Event (Winit)
        ↓
WinitGpuiApp::window_event() (main.rs)
        ↓
Event conversion (Winit → GPUI format)
        ↓
Motion smoothing (for mouse events)
        ↓
Click detection (for double-clicks)
        ↓
GPUI inject_input_event()
        ↓
GPUI event routing to views
        ↓
View handlers update state
        ↓
Render request
        ↓
GPUI draws to shared texture
        ↓
D3D11 composition (Windows)
        ↓
Present to screen
```

### Asset Loading Flow
```
Asset request (by path)
        ↓
Assets::get() (rust-embed)
        ↓
Embedded data OR filesystem load
        ↓
Format-specific decoding
        ↓
Cache in EngineState
        ↓
Return to requester
```

## State Management

### Global State (EngineState)
- Window management
- GPU renderers (per-window)
- Metadata dictionary
- Window creation channel
- Thread-safe (Arc + Mutex)

### Per-Window State (WindowState - main.rs)
- Winit window handle
- GPUI application instance
- GPUI window handle
- D3D11 rendering state (Windows)
- Event tracking (cursor, modifiers, buttons)
- Motion smoother
- Click state
- Bevy renderer reference

### Editor State
Each editor maintains its own state:
- Blueprint editor: Node graph, selection, clipboard
- Level editor: Scene, selection, camera
- Script editor: Open files, cursor positions
- DAW editor: Tracks, clips, routing

## Rendering Architecture

### Windows Platform (Direct3D 11)
```
Layer 0 (Background): Green clear color
        ↓
Layer 1 (3D Content): Bevy renders to D3D12 shared texture
        ↓ (opened as D3D11 texture)
        ↓
Composite to swap chain back buffer (opaque blend)
        ↓
Layer 2 (UI): GPUI renders to D3D11 shared texture
        ↓
Composite to swap chain back buffer (alpha blend)
        ↓
Present
```

**Key Technologies**:
- Shared textures for zero-copy
- Runtime HLSL shader compilation
- Fullscreen quad rendering
- Alpha blending for UI
- Separate texture caching to prevent flickering

### Cross-Platform (WGPU)
- GPUI uses WGPU for cross-platform rendering
- Bevy can use WGPU backend
- Compositor planned for non-Windows platforms

## Performance Considerations

### Compilation
- Parallel crate compilation
- Optimized dependencies (resvg, rustybuzz, taffy, ttf-parser)
- Split debug info
- 16 codegen units in dev

### Runtime
- Event-driven rendering (ControlFlow::Wait)
- Motion smoothing reduces event spam
- Texture caching prevents re-allocation
- Zero-copy GPU composition
- Async backend (Tokio 8 threads)

### Memory
- Arc/Mutex for shared state
- DashMap for concurrent access
- Persistent texture buffers
- SRV caching to prevent leaks

## Testing Strategy

### Unit Tests
- Compiler tests (compiler/tests.rs)
- Graph tests (compiler/test_default_ui_graph.rs)
- Type system tests (graph/type_system.rs)

### Integration Tests
- Window creation
- Event routing
- Rendering pipeline

### Manual Testing
- Editor workflows
- Multi-window scenarios
- Hot reload
- Performance profiling

## Build System

### Workspace Configuration
```toml
[workspace]
members = [
    "crates/macros",
    "crates/engine",
    "crates/engine_backend",
    "crates/ui",
    "crates/pulsar_macros",
    "crates/pulsar_std",
]
```

### Dependencies
- GPUI from custom Zed fork (local path)
- Horizon game server integration
- Windows crate for D3D11
- Many audio, terminal, and utility crates

### Profiles
- **dev**: Fast compile, limited optimizations
- **release**: Full optimizations with debug symbols

## Error Handling

### Strategy
- Anyhow for error propagation
- Result types throughout
- Panic on critical failures (window creation, GPU init)
- Graceful degradation where possible
- User-friendly error messages

### Logging
- Tracing for structured logging
- Log levels: trace, debug, info, warn, error
- File and console output
- Performance tracing (flamegraph support)

## Future Roadmap

### Short Term
- Refactor large files (main.rs, panel.rs, viewport.rs, etc.)
- Complete stub editors
- Improve documentation
- Add more tests

### Medium Term
- Animation editor
- Particle system editor
- Material editor
- Physics editor
- Behavior tree editor

### Long Term
- Visual debugger
- Profiler UI
- Multiplayer tools
- Platform-specific optimizations
- Plugin system
- Marketplace integration

## Code Quality Metrics

### File Size Analysis
- Files > 1500 lines: 7 (needs refactoring)
- Files > 1000 lines: 14 (consider refactoring)
- Files > 500 lines: 31 (good candidates)
- Average file size: ~350 lines
- Total: ~45,000+ lines across 133 files

### Module Organization
- Well-structured: compiler/, graph/, settings/
- Needs improvement: ui/ (too flat), main.rs (monolithic)
- Missing: Proper window/ module, better ui/ hierarchy

### Documentation
- Architecture docs: This file + others
- Module docs: Sparse (needs improvement)
- Function docs: Minimal (needs improvement)
- Inline comments: Present but inconsistent

### Technical Debt
1. **main.rs** - 1907 lines, needs extraction to modules
2. **ui/panels/blueprint_editor2/panel.rs** - 3748 lines, needs major refactoring
3. **ui/panels/level_editor/ui/viewport.rs** - 2050 lines, needs refactoring
4. **ui/terminal/terminal_element_zed.rs** - 2012 lines, needs refactoring
5. **ui/panels/blueprint_editor2/node_graph.rs** - 1946 lines, needs refactoring
6. **ui/app.rs** - 1874 lines, needs refactoring
7. **ui/file_manager_drawer.rs** - 1817 lines, needs refactoring
8. Stub files - Many 10-82 line placeholder files
9. Inconsistent error handling
10. Missing comprehensive tests

## Glossary

**Blueprint**: Visual scripting graph that compiles to Rust code
**Node**: Single operation in a Blueprint (function call, variable access, etc.)
**Pin**: Connection point on a node (input or output)
**Exec Pin**: Execution flow connection
**Data Pin**: Data flow connection
**Subgraph**: Reusable Blueprint that can be instanced
**Gizmo**: 3D manipulation handle (translate/rotate/scale)
**ECS**: Entity Component System (Bevy's architecture)
**LSP**: Language Server Protocol
**PTY**: Pseudo-terminal
**DAW**: Digital Audio Workstation
**SRV**: Shader Resource View (D3D11)
**RTV**: Render Target View (D3D11)

## Conclusion

Pulsar Engine is a ambitious, comprehensive game engine project with strong fundamentals but needing organizational improvements. The core systems (Blueprint compiler, editors, rendering) are functional and well-designed, but the codebase would benefit from:

1. Breaking up large files into logical modules
2. Comprehensive documentation
3. Consistent code style
4. More tests
5. Better error handling
6. Completing stub implementations

This documentation serves as the foundation for ongoing refactoring and organizational efforts.

---

**Document Version**: 1.0
**Last Updated**: 2025-01-03
**Maintainers**: Pulsar Engine Team
