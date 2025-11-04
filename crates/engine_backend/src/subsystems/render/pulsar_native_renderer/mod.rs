#[cfg(feature = "pulsar_native_renderer")]
pub mod pulsar_native_renderer {
    pub use crate::renderer::*;
    pub use crate::backend::*;
    pub use crate::multigpu::*;
    pub use crate::raytracing_dxr::*;
    pub use crate::dlss_ngx::*;
    pub use crate::direct_storage::*;
    pub use crate::zero_copy_buffers::*;
    pub use crate::textures::*;
    pub use crate::scene::*;
    pub use crate::threadpool::*;
    pub use crate::win_host::*;
    pub use crate::d3d11_fallback::*;
    pub use crate::vulkan::*;
    pub use crate::opengl::*;

    /// Initializes Pulsar_Native renderer instead of Bevy.
    pub fn init_native_renderer() {
        unsafe {
            println!("[Pulsar_Native] Initializing DX12/Vulkan/Multigpu renderer for testing.");
            // Example window and renderer creation
            let hwnd = crate::win_host::create_window(1280, 720, "Pulsar_Native Renderer Test");
            let mut renderer = crate::renderer::Renderer::new(hwnd, 1280, 720, 4);
            while crate::win_host::pump_messages() {
                renderer.render_frame(|_, _| {});
            }
        }
    }
}

#[cfg(not(feature = "pulsar_native_renderer"))]
pub mod bevy_renderer {
    /// Initializes Bevy renderer normally.
    pub fn init_bevy_renderer() {
        println!("[Bevy] Using default Bevy renderer.");
        // Insert Bevy app setup or window creation as needed.
    }
}

/// Call this entry point to choose renderer at runtime based on feature flag.
pub fn init_renderer() {
    #[cfg(feature = "pulsar_native_renderer")]
    {
        pulsar_native_renderer::init_native_renderer();
    }
    #[cfg(not(feature = "pulsar_native_renderer"))]
    {
        bevy_renderer::init_bevy_renderer();
    }
}