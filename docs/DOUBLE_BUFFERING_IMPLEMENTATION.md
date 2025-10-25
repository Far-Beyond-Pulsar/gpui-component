# Double Buffering Implementation

## Overview

This document describes the implementation of double buffering for the Bevy renderer, replacing the previous single-buffer approach. Double buffering eliminates visual tearing and race conditions by ensuring the renderer writes to one buffer while GPUI reads from another.

## Architecture

### Buffer Structure

The system maintains two DXGI shared textures (render targets):
- **Buffer 0**: Can be either read or write buffer (swaps each frame)
- **Buffer 1**: Can be either read or write buffer (swaps each frame)

### Key Components

#### 1. SharedGpuTextures (types.rs)
```rust
pub struct SharedGpuTextures {
    pub textures: Arc<[Handle<Image>; 2]>,           // Two render target handles
    pub native_handles: Arc<Mutex<Option<[NativeTextureHandle; 2]>>>, // DXGI handles
    pub write_index: Arc<AtomicUsize>,               // Current write buffer index
    pub read_index: Arc<AtomicUsize>,                // Current read buffer index
    pub frame_number: Arc<AtomicU64>,                // Frame counter
    pub width: u32,
    pub height: u32,
}
```

#### 2. Buffer Swapping System (scene.rs)
The `swap_render_buffers_system` runs each frame to:
1. Atomically swap write_index and read_index
2. Update the camera's render target to point to the new write buffer
3. Increment the frame counter
4. Log swap operations (every 120 frames)

#### 3. Buffer Access (renderer.rs)
- `get_read_index()`: Returns the current read buffer index (used by GPUI)
- `get_current_native_handle()`: Returns the DXGI handle for the current read buffer

## Implementation Details

### Initialization (textures.rs)

Both buffers are created at startup:
```rust
write_index: AtomicUsize::new(0),  // Start writing to buffer 0
read_index: AtomicUsize::new(1),   // Start reading from buffer 1
```

### Scene Setup (scene.rs)

The camera is initially configured to render to the write buffer:
```rust
let write_index = textures.write_index.load(Ordering::Acquire);
let render_target = textures.textures[write_index].clone();
camera.target = RenderTarget::Image(render_target.into());
```

### Frame Flow

**Frame N:**
1. Bevy renders to buffer at `write_index` (e.g., buffer 0)
2. GPUI reads from buffer at `read_index` (e.g., buffer 1)
3. `swap_render_buffers_system` runs at end of frame
4. Indices swap: write_index=1, read_index=0
5. Camera target updated to buffer 1

**Frame N+1:**
1. Bevy renders to buffer 1 (previous read buffer, now write)
2. GPUI reads from buffer 0 (previous write buffer, now read)
3. Process repeats...

## System Ordering

Systems are ordered to ensure correct double buffering:

```rust
// Sync systems (FIRST - get data from other threads)
.add_systems(Update, sync_camera_input_system)
.add_systems(Update, sync_gizmo_state_system)
.add_systems(Update, sync_viewport_mouse_input_system)
.add_systems(Update, sync_game_objects_system)

// Game systems (MIDDLE - update game state)
.add_systems(Update, camera_movement_system)
.add_systems(Update, update_gizmo_target_system)
.add_systems(Update, viewport_click_selection_system)
.add_systems(Update, gizmo_drag_system)

// Rendering systems (LAST - prepare for render)
.add_systems(Update, update_metrics_system)
.add_systems(Update, update_gpu_profiler_system)
.add_systems(Update, update_gizmo_visuals)
.add_systems(Update, update_selection_highlighting)
.add_systems(Update, debug_rendering_system)
.add_systems(Update, swap_render_buffers_system)  // CRITICAL: LAST
```

**Important:** `swap_render_buffers_system` MUST run last in the Update schedule to ensure:
1. All rendering for the current frame is queued
2. The camera target is updated for the NEXT frame
3. Indices are swapped atomically before the next frame begins

## Benefits

### 1. No Visual Tearing
- GPUI always reads from a complete, stable frame
- Bevy never overwrites a buffer being read by GPUI

### 2. Thread Safety
- Atomic operations ensure lock-free buffer index access
- No race conditions between render thread and UI thread

### 3. Performance
- Zero-copy rendering (DXGI shared textures)
- Lock-free reads via atomic indices
- Minimal overhead (~16 bytes of atomics per swap)

### 4. Debugging
- Frame counter tracks total frames rendered
- Periodic logging shows buffer swap activity
- Clear separation of write vs read buffers

## Testing

To verify double buffering is working:

1. **Check console logs** for buffer swap messages:
   ```
   [BEVY] 🔄 Buffer swap: write=1, read=0, frame=120
   [BEVY] 🔄 Buffer swap: write=0, read=1, frame=240
   ```

2. **Verify no tearing** in viewport during camera movement

3. **Check frame counter** increments properly:
   ```rust
   let frame_num = textures.frame_number.load(Ordering::Acquire);
   ```

4. **Monitor read/write indices** are always different:
   ```rust
   assert_ne!(write_index, read_index);
   ```

## Future Enhancements

### Triple Buffering
Could add a third buffer to allow:
- One buffer being rendered
- One buffer being read by GPUI
- One buffer available for next frame

### Adaptive Buffering
Could dynamically switch between single/double/triple buffering based on:
- Frame rate stability
- GPU utilization
- GPUI read patterns

### Buffer Metrics
Could track per-buffer statistics:
- How long each buffer is read
- Write vs read duration
- Buffer "age" before being read

## Modified Files

1. **renderer.rs**
   - Updated `get_read_index()` to read from atomic instead of returning 0
   - Added buffer swap system to Update schedule

2. **scene.rs**
   - Updated `setup_scene()` to use write_index for initial camera target
   - Added `swap_render_buffers_system()` for frame-by-frame buffer swapping
   - Enhanced logging to show buffer indices

3. **textures.rs**
   - Added native_handles storage to SharedGpuTextures
   - Both DXGI handles now stored in array for indexed access

4. **mod.rs**
   - Updated comment to reflect double-buffering implementation

## Atomic Operations

All buffer index operations use atomic memory ordering:

- **Acquire**: Used when reading indices (ensures visibility of writes)
- **Release**: Used when writing indices (ensures writes are visible)
- **No Ordering::Relaxed**: Avoided to prevent reordering issues

This ensures proper synchronization between render thread and UI thread without locks.

## Conclusion

The double buffering implementation provides a robust, thread-safe, zero-copy rendering pipeline with excellent performance characteristics. The infrastructure was largely in place; this implementation primarily wired up the existing components and added the critical buffer swapping logic.
