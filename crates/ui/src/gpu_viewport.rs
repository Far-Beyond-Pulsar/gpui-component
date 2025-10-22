// Compatibility module: Re-exports bevy_viewport for backward compatibility
// This allows old code using `gpu_viewport` to work with the new `bevy_viewport` implementation

pub use crate::bevy_viewport::{
    BevyViewport,
    BevyViewportState,
    GpuTextureHandle as NativeTextureHandle,
    GpuCanvasSource,
};

// Type aliases for compatibility
pub type Viewport = BevyViewport;
pub type ViewportState = BevyViewportState;
