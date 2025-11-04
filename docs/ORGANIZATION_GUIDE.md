# Pulsar Engine - Code Organization & Documentation Guide

## Purpose

This document provides comprehensive guidance for organizing, documenting, and maintaining the Pulsar Engine codebase.

## Documentation Standards

### Module-Level Documentation

Every Rust file should start with module-level documentation (`//!`) that includes:

1. **Title** - Clear, concise name for the module
2. **Purpose** - What this module does and why it exists
3. **Architecture** - How it fits into the overall system (with diagrams if complex)
4. **Usage** - Example code showing how to use the module
5. **Key Types** - Overview of main structs/enums
6. **Implementation Notes** - Important details about the implementation

**Template**:
```rust
//! Module Title
//!
//! ## Purpose
//! Brief description of what this module does and why it exists.
//!
//! ## Architecture
//! ```text
//! [Optional ASCII diagram]
//! ```
//!
//! ## Usage
//! ```rust,ignore
//! // Example usage
//! ```
//!
//! ## Implementation Details
//! Important notes about the implementation.

// Module code here...
```

### Type Documentation

Every public struct, enum, and type alias needs documentation:

**Template**:
```rust
/// Brief description of the type
///
/// Longer description explaining:
/// - What it represents
/// - When to use it
/// - Any invariants or constraints
///
/// # Example
/// ```rust,ignore
/// // Example usage
/// ```
pub struct MyType {
    /// Field description
    field: Type,
}
```

### Function Documentation

Every public function needs documentation:

**Template**:
```rust
/// Brief description of what the function does
///
/// Longer description if needed.
///
/// # Arguments
/// * `param1` - Description of first parameter
/// * `param2` - Description of second parameter
///
/// # Returns
/// Description of return value
///
/// # Errors
/// Description of possible errors (if Result)
///
/// # Panics
/// Description of panic conditions (if any)
///
/// # Example
/// ```rust,ignore
/// // Example usage
/// ```
pub fn my_function(param1: Type1, param2: Type2) -> ReturnType {
    // Implementation
}
```

## File Organization Principles

### 1. One Concern Per File

Each file should have a single, clear responsibility. If a file grows beyond 500 lines, consider splitting it.

**Before** (bad):
```
panel.rs (3748 lines) - Everything for blueprint editor
```

**After** (good):
```
panel.rs (400 lines) - Main panel orchestration
canvas.rs (400 lines) - Canvas rendering
node_rendering.rs (400 lines) - Node rendering
selection.rs (300 lines) - Selection management
...
```

### 2. Module Hierarchy

Use subdirectories to organize related functionality:

```
feature/
├── mod.rs          # Public API and re-exports
├── types.rs        # Data structures
├── state.rs        # State management
├── operations.rs   # Core operations
└── ui/             # UI components
    ├── mod.rs
    ├── panel.rs
    └── toolbar.rs
```

### 3. Naming Conventions

- **Files**: `snake_case.rs`
- **Modules**: `snake_case`
- **Types**: `PascalCase`
- **Functions**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Lifetimes**: `'a`, `'b`, etc. (single lowercase letters)

**Suffixes**:
- `*_window.rs` - Top-level windows
- `*_drawer.rs` - Slide-out drawers/panels
- `*_editor.rs` - Editor components
- `*_manager.rs` - Manager/orchestrator
- `*_state.rs` - State management
- `*_types.rs` - Type definitions
- `*_utils.rs` - Utility functions

### 4. Import Organization

Organize imports in groups (separated by blank lines):

```rust
// 1. Standard library
use std::collections::HashMap;
use std::sync::Arc;

// 2. External crates
use serde::{Deserialize, Serialize};
use gpui::*;

// 3. Internal crates (workspace members)
use engine_backend::subsystems::render::NativeTextureHandle;

// 4. Local crate modules
use crate::engine_state::EngineState;
use crate::ui::shared::*;

// 5. Super/parent imports
use super::types::*;
```

## Refactoring Guidelines

### When to Refactor

Refactor when a file/function:
- Exceeds 500 lines (file) or 50 lines (function)
- Has multiple responsibilities
- Is difficult to understand or test
- Has high cyclomatic complexity
- Violates DRY (Don't Repeat Yourself)

### How to Refactor Large Files

**Step 1: Identify Logical Sections**
```rust
// main.rs (1907 lines)
// Lines 1-85: Actions and utilities
// Lines 86-246: Event handling helpers
// Lines 247-330: Main function
// Lines 331-488: Application structs
// Lines 489-1740: Event handlers
// Lines 1741-1907: Helper functions
```

**Step 2: Extract to New Modules**
```
main.rs → 
  actions.rs (85 lines)
  event_utils.rs (160 lines)
  main.rs (100 lines)
  window/
    app.rs (200 lines)
    events.rs (400 lines)
    handlers.rs (800 lines)
    helpers.rs (200 lines)
```

**Step 3: Update Imports**
```rust
mod actions;
mod event_utils;
mod window;

use actions::*;
use window::WinitGpuiApp;
```

**Step 4: Test**
```bash
cargo build
cargo test
```

### Refactoring Checklist

- [ ] Identify logical sections
- [ ] Create new files with proper module docs
- [ ] Move code (keeping functionality identical)
- [ ] Update imports
- [ ] Update mod.rs files
- [ ] Add comprehensive documentation
- [ ] Test compilation
- [ ] Run existing tests
- [ ] Update ARCHITECTURE.md if structure changes

## Priority Refactoring List

### Critical (Must Do)

1. **main.rs** (1907 lines)
   - Extract to `window/` module
   - Separate event handling
   - Extract D3D11 code
   - Priority: HIGHEST

2. **ui/panels/blueprint_editor2/panel.rs** (3748 lines)
   - Split into canvas, nodes, selection, etc.
   - Priority: HIGHEST

3. **ui/panels/level_editor/ui/viewport.rs** (2050 lines)
   - Extract camera control
   - Extract rendering
   - Extract input handling
   - Priority: HIGH

4. **ui/terminal/terminal_element_zed.rs** (2012 lines)
   - Extract rendering
   - Extract input handling
   - Priority: HIGH

5. **ui/panels/blueprint_editor2/node_graph.rs** (1946 lines)
   - Extract node rendering
   - Extract connection rendering
   - Priority: HIGH

### High Priority

6. **ui/app.rs** (1874 lines)
7. **ui/file_manager_drawer.rs** (1817 lines)
8. **ui/rust_analyzer_manager.rs** (1249 lines)
9. **ui/panels/script_editor/text_editor.rs** (1205 lines)
10. **graph/mod.rs** (1200 lines)

### Medium Priority

11-20. Files between 600-1000 lines
21-30. Files between 400-600 lines

### Low Priority (But Still Important)

- Complete stub implementations (10-82 lines)
- Add missing tests
- Improve error handling
- Add examples

## Documentation Checklist

For each file, ensure:

- [ ] Module-level documentation (`//!`)
- [ ] Purpose clearly stated
- [ ] Architecture explained (with diagrams if complex)
- [ ] Usage examples provided
- [ ] All public types documented
- [ ] All public functions documented
- [ ] All function parameters explained
- [ ] Return values explained
- [ ] Error conditions documented
- [ ] Panic conditions documented
- [ ] Examples provided where helpful
- [ ] Links to related modules
- [ ] Implementation notes for complex logic

## Testing Guidelines

### Unit Tests

Place unit tests at the bottom of files:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Test code
    }
}
```

### Integration Tests

Place in `tests/` directory:

```
tests/
├── common/
│   └── mod.rs     # Test utilities
├── compiler_tests.rs
├── editor_tests.rs
└── rendering_tests.rs
```

### Test Coverage Goals

- Core systems: 80%+ coverage
- UI components: 50%+ coverage
- Utilities: 90%+ coverage

## Error Handling Guidelines

### Use Result for Recoverable Errors

```rust
pub fn load_file(path: &Path) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}
```

### Use Panic for Programming Errors

```rust
pub fn get_item(&self, index: usize) -> &Item {
    &self.items[index] // Panic if out of bounds
}
```

### Provide Context with Anyhow

```rust
use anyhow::{Context, Result};

pub fn complex_operation() -> Result<()> {
    do_something()
        .context("Failed to do something")?;
    Ok(())
}
```

## Performance Guidelines

### Avoid Allocations in Hot Paths

```rust
// Bad: Allocates on every call
fn process(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    // ...
    result
}

// Good: Reuse allocation
fn process(data: &[u8], output: &mut Vec<u8>) {
    output.clear();
    // ...
}
```

### Use References When Possible

```rust
// Bad: Clones data
fn process(data: String) {
    // ...
}

// Good: Borrows data
fn process(data: &str) {
    // ...
}
```

### Profile Before Optimizing

```bash
# Use flamegraph for profiling
cargo flamegraph --bin pulsar_engine
```

## Git Commit Guidelines

### Commit Message Format

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types**:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Formatting, missing semicolons, etc.
- `refactor`: Code change that neither fixes a bug nor adds a feature
- `perf`: Performance improvement
- `test`: Adding tests
- `chore`: Maintenance

**Example**:
```
refactor(window): Extract event handling to separate module

Split the 1907-line main.rs into focused modules:
- window/app.rs: Application handler
- window/events.rs: Event utilities
- window/handlers.rs: Event handler implementations

This improves code organization and makes the codebase more maintainable.

Closes #123
```

## Code Review Checklist

Before submitting code:

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] New code has tests
- [ ] Documentation is complete
- [ ] No commented-out code
- [ ] No debug print statements
- [ ] Error handling is appropriate
- [ ] Performance impact considered
- [ ] Breaking changes documented
- [ ] ARCHITECTURE.md updated if needed

## IDE Setup

### Recommended VS Code Extensions

- rust-analyzer: Rust language support
- Even Better TOML: TOML syntax
- crates: Cargo.toml management
- Error Lens: Inline error display
- GitLens: Git integration

### Recommended Settings

```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.features": "all",
    "editor.formatOnSave": true,
    "files.trimTrailingWhitespace": true
}
```

## Continuous Improvement

### Regular Maintenance

- **Weekly**: Review new TODOs
- **Monthly**: Check for dependency updates
- **Quarterly**: Performance profiling
- **Yearly**: Architecture review

### Technical Debt Tracking

Use comments to mark technical debt:

```rust
// TODO(username): Description of what needs to be done
// FIXME(username): Description of the problem
// HACK(username): Explanation of why this is a hack
// NOTE(username): Important implementation note
```

Track in GitHub Issues with labels:
- `tech-debt`
- `refactoring`
- `documentation`
- `performance`

## Resources

### Documentation
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Documentation Book](https://doc.rust-lang.org/rustdoc/)
- [ARCHITECTURE.md](./ARCHITECTURE.md)

### Tools
- `cargo doc` - Generate documentation
- `cargo fmt` - Format code
- `cargo clippy` - Lint code
- `cargo test` - Run tests
- `cargo flamegraph` - Profile performance

### Learning
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rust Design Patterns](https://rust-unofficial.github.io/patterns/)

## Conclusion

Good code organization and documentation are investments that pay dividends in:
- Faster development
- Easier onboarding
- Fewer bugs
- Better collaboration
- Long-term maintainability

This guide should be treated as a living document and updated as the project evolves.

---

**Document Version**: 1.0
**Last Updated**: 2025-01-03
**Maintainers**: Pulsar Engine Team
