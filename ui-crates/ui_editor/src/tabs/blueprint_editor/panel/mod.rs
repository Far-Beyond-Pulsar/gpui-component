//! Blueprint Editor Panel - Main Module
//!
//! This module contains the main BlueprintEditorPanel struct and coordinates
//! all functionality across specialized submodules.
//!
//! ## Module Organization
//!
//! - `core` - Core panel struct, initialization, and accessors
//! - `tabs` - Tab management (creation, switching, closing)
//! - `file_io` - Save/load blueprint files
//! - `compilation` - Compile blueprints to Rust code
//! - `graph_conversion` - Convert between graph formats
//! - `node_ops` - Node operations (create, delete, duplicate, copy/paste)
//! - `connection_ops` - Connection dragging and management
//! - `comment_ops` - Comment operations (drag, resize, edit)
//! - `selection` - Selection box and multi-selection
//! - `variables` - Variable management (create, delete, getter/setter nodes)
//! - `viewport` - Pan, zoom, and viewport controls
//! - `menu` - Node creation menu and context menus
//! - `render` - Main rendering implementation

use gpui::*;
use ui::{
    button::{Button, ButtonVariants as _},
    dock::{Panel, PanelEvent},
    input::InputState,
    resizable::{h_resizable, v_resizable, resizable_panel, ResizableState},
    tab::{Tab, TabBar},
    v_flex, h_flex, ActiveTheme as _, PixelsExt, IconName,
};
use smol::Timer;
use std::time::Duration;

use super::hoverable_tooltip::HoverableTooltip;
use super::node_creation_menu::{NodeCreationEvent, NodeCreationMenu};
use super::node_graph::NodeGraphRenderer;
use super::toolbar::ToolbarRenderer;
use super::*;
use super::BlueprintComment;
use ui::graph::{DataType as GraphDataType, GraphDescription};

// Re-export main types
pub use core::{BlueprintEditorPanel, ConnectionDrag, ResizeHandle};
pub use tabs::{GraphTab, SerializedGraphTab};

// Module declarations
mod core;
mod tabs;
mod file_io;
mod compilation;
mod graph_conversion;
mod node_ops;
mod connection_ops;
mod comment_ops;
mod selection;
mod variables;
mod viewport;
mod menu;
mod render;

// Constants
pub const NODE_MENU_WIDTH: f32 = 280.0;
pub const NODE_MENU_MAX_HEIGHT: f32 = 350.0;
