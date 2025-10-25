//! Bevy Renderer with DIRECT rendering to DXGI shared textures
//! Clean implementation - no complex buffer swapping, just render to shared GPU memory
//!
//! This module is organized into logical submodules:
//! - `types`: Core data structures (RenderMetrics, GpuProfilerData, CameraInput, etc.)
//! - `components`: Bevy ECS components (MainCamera, GameObjectId, Selected, SelectionOutline)
//! - `resources`: Bevy ECS resources (shared state between systems)
//! - `camera`: Camera input synchronization and movement systems
//! - `sync`: Game object synchronization between game and render threads
//! - `metrics`: Performance metrics and GPU profiling systems
//! - `scene`: Scene setup (objects, lights, camera spawning)
//! - `textures`: CRITICAL - DXGI shared texture creation and management
//! - `renderer`: Main BevyRenderer struct and initialization
//! - `gizmos_bevy`: Gizmo rendering for level editor (translate/rotate/scale visuals)
//! - `gizmo_interaction`: Gizmo mouse interaction and manipulation (DEPRECATED - use viewport_interaction)
//! - `viewport_interaction`: Complete viewport interaction system (object selection, gizmo manipulation)

pub mod types;
pub mod components;
pub mod resources;
pub mod camera;
pub mod sync;
pub mod metrics;
pub mod scene;
pub mod textures;
pub mod renderer;
pub mod gizmos_bevy;
pub mod gizmo_interaction;
pub mod viewport_interaction;

// Re-export public API
pub use types::{RenderMetrics, GpuProfilerData, DiagnosticMetric, CameraInput, SharedGpuTextures};
pub use renderer::BevyRenderer;
pub use textures::{RENDER_WIDTH, RENDER_HEIGHT};
pub use gizmos_bevy::{GizmoType as BevyGizmoType, GizmoAxis as BevyGizmoAxis, GizmoStateResource};
pub use viewport_interaction::{ViewportMouseInput, GizmoInteractionState};
pub use sync::{sync_gizmo_state_system, sync_viewport_mouse_input_system};
