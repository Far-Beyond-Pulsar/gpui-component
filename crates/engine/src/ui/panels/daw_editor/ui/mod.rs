/// Complete DAW UI Module
/// Production-quality interface components for the embedded DAW

mod state;
mod panel;
mod mixer;

// Individual panels - to be implemented one by one
mod timeline;
// mod transport;
// mod inspector;
mod browser;
// mod toolbar;
mod track_header;
// mod clip_editor;
// mod automation;
// mod effects;
// mod routing;

pub use state::*;
pub use panel::DawPanel;
