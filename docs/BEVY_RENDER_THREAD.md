# Bevy 0.17.2 Render Thread Integration - Implementation Summary

## Overview
Implemented a zero-copy Bevy renderer that runs on a dedicated render thread and streams frames to the main viewport.

## Key Features

### 1. **Dedicated Render Thread**
- Bevy runs in its own thread, completely separate from the main UI thread
- Uses `ScheduleRunnerPlugin` for headless rendering at 60 FPS
- No window creation - completely offscreen rendering

### 2. **Zero-Copy Frame Transfer**
- Uses `crossbeam_channel` for lock-free communication between render and main threads
- GPU texture → GPU buffer → CPU buffer → Viewport framebuffer
- Direct memory copy when alignment matches
- Handles WGPU row alignment automatically

### 3. **Architecture**

```
Main Thread (GPUI)                 Render Thread (Bevy)
┌─────────────────┐               ┌──────────────────┐
│  Viewport       │               │  Bevy App        │
│  Framebuffer    │◄──channel─────┤  (headless)      │
│  (RGBA8)        │               │                  │
└─────────────────┘               └──────────────────┘
        ▲                                   │
        │                                   │
        └────── Zero-Copy Transfer ─────────┘
```

### 4. **Render Pipeline**

1. **Scene Setup** (Startup)
   - Create render target texture with `COPY_SRC` usage
   - Setup camera pointing to render target
   - Create 3D scene (cube, ground plane, lights)
   - Spawn `ImageCopier` component with GPU buffer

2. **Per-Frame** (Update Loop)
   - Bevy renders scene to texture (60 FPS)
   - `ImageCopyDriver` node copies texture to buffer
   - `receive_image_from_buffer` maps buffer and sends to channel
   - Main thread receives and copies to viewport framebuffer

3. **Render Graph Integration**
   ```
   CameraDriver → ImageCopy (custom node) → RenderSystems::Render
   ```

## Implementation Details

### BevyRenderer API
```rust
pub struct BevyRenderer {
    receiver: Receiver<Vec<u8>>,  // Frame data channel
    thread_handle: JoinHandle<()>, // Render thread
    width: u32,
    height: u32,
    frame_count: u64,
    running: Arc<AtomicBool>,     // Thread control
}

impl BevyRenderer {
    // Create and start render thread
    pub async fn new(width: u32, height: u32) -> Self;
    
    // Get latest frame (non-blocking)
    pub fn render(&mut self, framebuffer: &mut Framebuffer);
    
    // Resize render target
    pub fn resize(&mut self, width: u32, height: u32);
}
```

### ImageCopyPlugin
Custom Bevy plugin that:
- Extracts `ImageCopier` components to render world
- Adds `ImageCopyDriver` node to render graph
- Handles buffer mapping and channel communication

### Key Components

**ImageCopier** - Tracks texture and GPU buffer
```rust
struct ImageCopier {
    buffer: Buffer,              // GPU→CPU staging buffer
    enabled: Arc<AtomicBool>,    // Runtime enable/disable
    src_image: Handle<Image>,    // Render target
}
```

**ImageCopyDriver** - Render graph node
```rust
impl render_graph::Node for ImageCopyDriver {
    fn run(&self, ...) -> Result<(), NodeRunError> {
        // Copy GPU texture to GPU buffer
        encoder.copy_texture_to_buffer(..);
    }
}
```

## Benefits

### Performance
- **Zero-copy**: Direct buffer transfer, no intermediate allocations
- **Parallel**: Bevy rendering doesn't block UI thread
- **Efficient**: GPU→CPU transfer only when needed

### Flexibility
- Can pause/resume rendering independently
- Easy to add multiple viewports
- Can capture frames at any time
- Resize without recreating entire app

### Compatibility
- Works without display server (headless)
- Platform-independent (uses WGPU)
- Bevy 0.17.2 APIs (latest stable)

## Usage Example

```rust
// Create renderer on separate thread
let renderer = BevyRenderer::new(1920, 1080).await;

// In render loop
loop {
    renderer.render(&mut framebuffer);
    // framebuffer now contains latest Bevy frame
}
```

## Files Modified

1. **`crates/engine_backend/Cargo.toml`**
   - Updated Bevy to 0.17.2
   - Updated WGPU to 23
   - Added crossbeam-channel dependency

2. **`crates/engine_backend/src/subsystems/render/bevy_renderer.rs`**
   - Complete rewrite based on Bevy 0.17.2 headless example
   - Implements threaded rendering with channel communication
   - Zero-copy frame transfer with alignment handling

## Technical Notes

### Buffer Alignment
WGPU requires buffer rows to be aligned to 256 bytes. The implementation handles this by:
1. Creating buffer with padded row size
2. Detecting misalignment on copy
3. Copying row-by-row when needed

### Thread Safety
- `Arc<AtomicBool>` for thread-safe control
- `crossbeam_channel` for lock-free communication
- Bevy's `Send` resources for data sharing

### Frame Latency
- 1-frame latency due to async nature
- Main thread receives frame N+1 when Bevy renders frame N
- Acceptable for real-time 3D viewport

## Future Enhancements

1. **Dynamic Scene Control**
   - Send commands to modify scene
   - Camera control from main thread
   - Object manipulation

2. **Multiple Viewports**
   - Multiple render targets
   - Different cameras/perspectives
   - Picture-in-picture

3. **Frame Synchronization**
   - VSync with main thread
   - Frame pacing
   - Adaptive quality

4. **Advanced Features**
   - Post-processing effects
   - Debug visualization
   - Performance metrics overlay

## References

- Bevy 0.17.2 headless_renderer example
- WGPU buffer mapping documentation
- Crossbeam channel documentation
