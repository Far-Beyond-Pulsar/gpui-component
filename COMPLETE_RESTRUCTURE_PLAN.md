# Complete Engine Restructure Plan

## Current Problems

1. **Monolithic engine crate**: 55k+ lines doing everything
2. **UI mixed with logic**: Windows, tabs, editors all in one crate
3. **No proper separation**: Backend, frontend, UI, windowing all mixed
4. **Massive files**: Single files with 1000+ lines
5. **Unclear responsibilities**: Hard to find where code lives

## New Architecture

```
Pulsar-Native/
├── crates/
│   ├── engine/                    # Orchestrator only (~2k lines)
│   │   ├── main.rs               # Entry point, setup
│   │   └── orchestrator.rs       # Coordinates all systems
│   │
│   ├── engine_backend/           # Pure backend (expanded)
│   │   └── subsystems/
│   │       ├── networking/       # ✅ Already moved
│   │       ├── rendering/        # Bevy, WGPU
│   │       ├── physics/          # Rapier
│   │       ├── audio/            # Audio engine
│   │       ├── scripting/        # Script runtime
│   │       ├── assets/           # Asset management
│   │       └── lsp/              # LSP, RustAnalyzer
│   │
│   ├── engine_window/            # NEW: Window management
│   │   ├── winit_integration/    # OS windowing
│   │   ├── compositor/           # D3D11 composition
│   │   ├── events/               # Event handling
│   │   └── lifecycle/            # Window lifecycle
│   │
│   └── engine_state/             # NEW: Global state management
│       ├── metadata/              # Key-value storage
│       ├── renderers/             # GPU renderer registry
│       └── channels/              # Communication channels
│
└── ui-crates/                     # NEW: UI components as crates
    ├── ui_core/                   # Core UI app (PulsarApp, PulsarRoot)
    ├── ui_entry/                  # Entry screen/project selector
    ├── ui_editor/                 # Main editor window
    │   ├── tabs/                  # Editor tabs (script, level, etc.)
    │   └── drawers/               # Drawers (file manager, terminal, etc.)
    ├── ui_settings/               # Settings window
    ├── ui_multiplayer/            # Multiplayer window
    ├── ui_terminal/               # Terminal component
    ├── ui_problems/               # Problems window
    ├── ui_file_manager/           # File manager window
    └── ui_common/                 # Shared UI utilities
```

## Crate Breakdown

### `crates/engine` (~2000 lines)
**Purpose**: Orchestrate all systems
**Contents**:
- `main.rs`: Entry point, initialization
- `orchestrator.rs`: Coordinates subsystems
- `config.rs`: Engine configuration
**Dependencies**: All other crates
**Exports**: Entry point only

### `crates/engine_backend` (expand existing)
**Purpose**: Pure backend, UI-agnostic
**Subsystems**:
- `networking/`: WebSocket, P2P, git sync ✅
- `rendering/`: Bevy, WGPU rendering
- `physics/`: Rapier integration
- `audio/`: Audio engine
- `scripting/`: Script compilation/runtime
- `assets/`: Asset loading/management
- `lsp/`: LSP server, RustAnalyzer
**Dependencies**: tokio, bevy, rapier, etc.
**Exports**: Clean API for each subsystem

### `crates/engine_window` (NEW)
**Purpose**: OS window management
**Contents**:
- `winit_integration/`: Winit event loop
- `compositor/`: D3D11/Vulkan composition
- `events/`: Event conversion/routing
- `lifecycle/`: Window create/destroy
- `state/`: Per-window state
**Dependencies**: winit, raw-window-handle, windows
**Exports**: WindowManager, WindowHandle

### `crates/engine_state` (NEW)
**Purpose**: Global engine state
**Contents**:
- `metadata/`: Key-value storage
- `renderers/`: GPU renderer registry
- `channels/`: Communication channels
- `registry/`: Global registries
**Dependencies**: dashmap, parking_lot
**Exports**: EngineState, channels

### `ui-crates/ui_core`
**Purpose**: Core UI application
**Contents**:
- `app.rs`: PulsarApp
- `root.rs`: PulsarRoot
- `command_palette.rs`: Command palette
- `menu.rs`: Menu system
**Dependencies**: gpui, gpui_component
**Exports**: PulsarApp, PulsarRoot

### `ui-crates/ui_entry`
**Purpose**: Project selection screen
**Contents**:
- `window.rs`: EntryWindow
- `views/`: Recent projects, new project, templates
- `project_selector/`: Project picker
- `git_operations.rs`: Git clone UI
**Dependencies**: ui_core, ui_common, engine_state
**Exports**: EntryWindow

### `ui-crates/ui_editor`
**Purpose**: Main editor window
**Contents**:
- `window.rs`: Main editor window
- `tabs/`: All editor tab types
  - `script/`: Script editor
  - `level/`: Level editor
  - `blueprint/`: Blueprint editor
  - `daw/`: DAW editor
  - `specialized/`: Specialized editors
- `drawers/`: Side panels
  - `file_manager/`: File browser
  - `terminal/`: Terminal drawer
  - `problems/`: Problems panel
**Dependencies**: ui_core, ui_common, engine_backend
**Exports**: EditorWindow, all tabs/drawers

### `ui-crates/ui_settings`
**Purpose**: Settings window
**Contents**:
- `window.rs`: Settings window
- `sections/`: Settings sections
**Dependencies**: ui_core, ui_common
**Exports**: SettingsWindow

### `ui-crates/ui_multiplayer`
**Purpose**: Multiplayer collaboration
**Contents**:
- `window.rs`: Multiplayer window
- `session.rs`: Session management
- `chat.rs`: Chat UI
- `file_sync.rs`: File sync UI
- `presence.rs`: User presence
**Dependencies**: ui_core, ui_common, engine_backend
**Exports**: MultiplayerWindow

### `ui-crates/ui_terminal`
**Purpose**: Terminal emulator
**Contents**:
- `terminal.rs`: Terminal component
- `pty.rs`: PTY management
- `rendering.rs`: Terminal rendering
**Dependencies**: ui_common, alacritty_terminal
**Exports**: Terminal

### `ui-crates/ui_problems`
**Purpose**: Problems/errors display
**Contents**:
- `window.rs`: Problems window
- `errors.rs`: Error display
**Dependencies**: ui_core, ui_common
**Exports**: ProblemsWindow

### `ui-crates/ui_file_manager`
**Purpose**: File browser
**Contents**:
- `window.rs`: File manager window
- `tree.rs`: File tree
**Dependencies**: ui_core, ui_common
**Exports**: FileManagerWindow

### `ui-crates/ui_common`
**Purpose**: Shared UI utilities
**Contents**:
- `helpers/`: UI helper functions
- `styles/`: Common styles
- `utils/`: Utility functions
**Dependencies**: gpui, gpui_component
**Exports**: Shared utilities

## Migration Strategy

### Phase 1: Create New Crate Structure
1. Create `ui-crates/` directory
2. Create each UI crate with Cargo.toml
3. Create `engine_window` crate
4. Create `engine_state` crate

### Phase 2: Move Backend Code
1. Move LSP to `engine_backend/subsystems/lsp/`
2. Move audio to `engine_backend/subsystems/audio/`
3. Clean up `engine_backend` structure

### Phase 3: Split UI Code
1. Move `ui/core/` → `ui_core`
2. Move `ui/windows/entry_*` → `ui_entry`
3. Move `ui/windows/editor/` → `ui_editor`
4. Move `ui/windows/settings/` → `ui_settings`
5. Move `ui/windows/multiplayer_*` → `ui_multiplayer`
6. Move `ui/windows/terminal/` → `ui_terminal`
7. Move remaining windows to their crates

### Phase 4: Extract Window Management
1. Move `window/` → `engine_window`
2. Clean up event handling
3. Simplify compositor

### Phase 5: Extract State Management
1. Move `engine_state.rs` → `engine_state` crate
2. Expand with proper subsystems
3. Add communication channels

### Phase 6: Slim Down Engine
1. Keep only orchestration in `engine`
2. Wire up all subsystems
3. Clean entry point

### Phase 7: Test & Verify
1. Ensure compilation
2. Test all functionality
3. Verify no breakage

## File Size Targets

- `engine` main.rs: <200 lines
- `engine` orchestrator.rs: <500 lines
- Individual UI crates: <3000 lines each
- Backend subsystems: <2000 lines each
- Window management: <2000 lines total

## Benefits

✅ **Modular**: Each UI component is independent
✅ **Testable**: Can test each crate separately
✅ **Maintainable**: Clear responsibilities
✅ **Scalable**: Easy to add new UI components
✅ **Parallel**: Multiple devs can work independently
✅ **Fast compilation**: Change one crate, not all
✅ **Clear APIs**: Explicit dependencies

## Execution Plan

I will now execute this plan step by step, creating each crate and moving code methodically. This will take time but will be done correctly.
