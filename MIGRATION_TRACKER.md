# Complete Restructure - Migration Tracker

## Status: In Progress

This document tracks the complete restructure of the Pulsar Engine into a modular, clean architecture.

## Architecture Created ✅

### New Crates
- ✅ `engine_state` - Global state management
- ✅ `engine_window` - OS window management  
- ✅ `ui_common` - Common UI utilities
- ✅ `ui_core` - Core UI app (PulsarApp, PulsarRoot)
- ✅ `ui_entry` - Entry/project selection screen
- ✅ `ui_editor` - Main editor window
- ✅ `ui_settings` - Settings window
- ✅ `ui_multiplayer` - Multiplayer window
- ✅ `ui_terminal` - Terminal component
- ✅ `ui_problems` - Problems window
- ✅ `ui_file_manager` - File manager window

### Workspace Configuration
- ✅ Updated `Cargo.toml` with all new crates
- ✅ Added workspace dependencies
- ✅ All crates compile successfully

## Phase 1: Foundation ✅

### engine_state Crate
- ✅ Created `lib.rs` with EngineState
- ✅ Created `metadata.rs` (key-value storage)
- ✅ Created `renderers.rs` (GPU renderer registry)
- ✅ Created `channels.rs` (WindowRequest channel)
- ✅ Compiles successfully

### engine_window Crate  
- ✅ Copied window module from engine
- ✅ Created lib.rs
- ⏳ Update imports to use engine_state
- ⏳ Remove dependencies on engine internals
- ⏳ Clean up and test

### UI Crates (Placeholders)
- ✅ Created all Cargo.toml files
- ✅ Created placeholder lib.rs files
- ✅ Created placeholder struct exports
- ✅ All compile successfully

## Phase 2: Move UI Code (TODO)

### ui_common
**Source**: `engine/src/ui/helpers/`, `engine/src/ui/common/`
**Target**: `ui-crates/ui_common/src/`

Files to move:
- [ ] `helpers/*` → `helpers/`
- [ ] Common utilities

### ui_core  
**Source**: `engine/src/ui/core/`, `engine/src/ui/common/`
**Target**: `ui-crates/ui_core/src/`

Files to move:
- [ ] `core/app.rs` → `app.rs` (PulsarApp, PulsarRoot)
- [ ] `core/shared.rs` → `shared.rs`
- [ ] `core/file_utils.rs` → `file_utils.rs`
- [ ] `common/command_palette/` → `command_palette.rs`
- [ ] `common/menu/` → `menu.rs`

### ui_entry
**Source**: `engine/src/ui/windows/entry_*`
**Target**: `ui-crates/ui_entry/src/`

Files to move:
- [ ] `entry_window.rs` → `window.rs`
- [ ] `entry_screen/mod.rs` → `screen/mod.rs`
- [ ] `entry_screen/views/` → `views/`
- [ ] `entry_screen/project_selector/` → `project_selector/`
- [ ] `entry_screen/git_operations.rs` → `git_operations.rs`
- [ ] `entry_screen/integration_launcher.rs` → `integration_launcher.rs`
- [ ] `entry_screen/recent_projects.rs` → `recent_projects.rs`
- [ ] `entry_screen/types.rs` → `types.rs`

### ui_editor
**Source**: `engine/src/ui/windows/editor/`
**Target**: `ui-crates/ui_editor/src/`

Files to move:
- [ ] `editor/mod.rs` → `window.rs`
- [ ] `editor/tabs/` → `tabs/` (entire directory)
  - [ ] `script_editor/` 
  - [ ] `level_editor/`
  - [ ] `blueprint_editor/`
  - [ ] `daw_editor/`
  - [ ] `specialized_editors/`
- [ ] `editor/drawers/` → `drawers/`
  - [ ] `file_manager.rs`
  - [ ] `terminal.rs`
  - [ ] `problems.rs`

### ui_settings
**Source**: `engine/src/ui/windows/settings/`
**Target**: `ui-crates/ui_settings/src/`

Files to move:
- [ ] `settings/mod.rs` → `window.rs`
- [ ] `settings/utils.rs` → `utils.rs`

### ui_multiplayer
**Source**: `engine/src/ui/windows/multiplayer_window/`
**Target**: `ui-crates/ui_multiplayer/src/`

Files to move:
- [ ] `multiplayer_window/mod.rs` → `lib.rs`
- [ ] `multiplayer_window/state.rs` → `window.rs`
- [ ] `multiplayer_window/types.rs` → `types.rs`
- [ ] `multiplayer_window/connection.rs` → `connection.rs`
- [ ] `multiplayer_window/session.rs` → `session.rs`
- [ ] `multiplayer_window/chat.rs` → `chat.rs`
- [ ] `multiplayer_window/file_sync.rs` → `file_sync.rs`
- [ ] `multiplayer_window/presence.rs` → `presence.rs`
- [ ] `multiplayer_window/ui.rs` → `ui.rs`
- [ ] `multiplayer_window/traits.rs` → `traits.rs`
- [ ] `multiplayer_window/utils.rs` → `utils.rs`

### ui_terminal
**Source**: `engine/src/ui/windows/terminal/`
**Target**: `ui-crates/ui_terminal/src/`

Files to move:
- [ ] `terminal/mod.rs` → `lib.rs`
- [ ] `terminal/terminal_core.rs` → `core.rs`
- [ ] `terminal/terminal_element.rs` → `element.rs`
- [ ] `terminal/rendering.rs` → `rendering.rs`
- [ ] `terminal/mappings/` → `mappings/`
- [ ] `terminal_window.rs` → `window.rs`

### ui_problems
**Source**: `engine/src/ui/windows/problems_window.rs`
**Target**: `ui-crates/ui_problems/src/`

Files to move:
- [ ] `problems_window.rs` → `window.rs`

### ui_file_manager
**Source**: `engine/src/ui/windows/file_manager_window.rs`
**Target**: `ui-crates/ui_file_manager/src/`

Files to move:
- [ ] `file_manager_window.rs` → `window.rs`

## Phase 3: Move Backend Code (TODO)

### engine_backend Cleanup
- [ ] Move LSP from `engine/src/ui/common/services/` → `engine_backend/subsystems/lsp/`
- [ ] Move audio engine code → `engine_backend/subsystems/audio/`
- [ ] Clean up existing subsystems
- [ ] Remove any remaining UI dependencies

## Phase 4: Slim Down Engine (TODO)

### engine Crate Restructure
**Target**: Keep only orchestration (~2000 lines)

Current `engine/src/` structure:
```
├── main.rs (100 lines)
├── engine_state.rs → MOVED to engine_state crate
├── subsystems/ → MOSTLY DELETE (replaced by proper crates)
├── ui/ → MOVE to ui-crates
├── window/ → MOVED to engine_window crate
├── assets.rs → Keep (orchestration)
├── compiler/ → Keep (compiler integration)
├── graph/ → Keep (node graph)
├── render/ → Consider moving to backend
├── settings.rs → Move to ui_settings or keep as model
├── themes.rs → Move to ui_common or keep as model
```

New `engine/src/` structure:
```
├── main.rs (entry point, ~150 lines)
├── orchestrator.rs (system coordination, ~500 lines)
├── config.rs (engine configuration, ~200 lines)
├── assets.rs (asset coordination, ~300 lines)
├── compiler.rs (compiler integration, ~400 lines)
└── models/ (data models)
    ├── settings.rs
    ├── themes.rs
    └── project.rs
```

## Phase 5: engine_window Cleanup (TODO)

- [ ] Update imports to use engine_state
- [ ] Remove dependencies on engine internals
- [ ] Add proper dependency on UI crates
- [ ] Clean up event handling
- [ ] Test window lifecycle

## Phase 6: Testing & Verification (TODO)

- [ ] Ensure all crates compile
- [ ] Test window creation
- [ ] Test all UI windows
- [ ] Test backend systems
- [ ] Verify no functionality broken
- [ ] Update documentation

## Success Criteria

✅ Workspace structure created
⏳ All code moved to proper crates
⏳ Engine crate < 3000 lines
⏳ Each UI crate < 5000 lines
⏳ Clear module boundaries
⏳ All functionality preserved
⏳ Compilation successful
⏳ Tests passing

## Current Line Counts

Before:
- engine: 55,111 lines
- engine_backend: 5,462 lines

After (Target):
- engine: <3,000 lines
- engine_backend: <10,000 lines
- UI crates: <5,000 lines each
- Total: More organized, faster compilation

## Notes

This is a methodical, careful restructure. Each file must be moved with all its imports updated. The process is:

1. Copy file to new location
2. Update imports in the file
3. Update exports in new crate's lib.rs
4. Add re-export in engine for compatibility (temporary)
5. Test compilation
6. Move next file

**DO NOT RUSH**. Correctness is more important than speed.
