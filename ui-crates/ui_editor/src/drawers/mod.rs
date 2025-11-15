//! Drawer Components
//!
//! Drawers are collapsible sidebar panels that can be shown/hidden in the main editor.
//! They provide auxiliary functionality like file browsing, problem lists, and terminal access.

pub mod file_manager_drawer;
pub mod problems_drawer;
pub mod terminal_drawer;

pub use file_manager_drawer::FileManagerDrawer;
pub use problems_drawer::ProblemsDrawer;
pub use terminal_drawer::TerminalDrawer;
