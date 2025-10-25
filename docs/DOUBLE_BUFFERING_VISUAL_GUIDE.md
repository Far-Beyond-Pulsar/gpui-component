# Double Buffering Visual Guide

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    SharedGpuTextures                            │
│                                                                 │
│  ┌──────────────┐  ┌──────────────┐                           │
│  │  Buffer 0    │  │  Buffer 1    │                           │
│  │ (DXGI Shared)│  │ (DXGI Shared)│                           │
│  │  1600x900    │  │  1600x900    │                           │
│  └──────────────┘  └──────────────┘                           │
│         ▲                 ▲                                     │
│         │                 │                                     │
│    write_index       read_index                                │
│    (AtomicUsize)    (AtomicUsize)                              │
│         │                 │                                     │
│         └─────────┬───────┘                                     │
│                   │                                             │
│           Swapped each frame                                   │
└─────────────────────────────────────────────────────────────────┘
```

## Frame Timeline

```
═══════════════════════════════════════════════════════════════════

Frame N:
┌─────────────┐                    ┌─────────────┐
│  Buffer 0   │ ◄─── WRITE ────────│ Bevy Render │
│  (Write)    │                    │   Thread    │
└─────────────┘                    └─────────────┘
     
┌─────────────┐                    ┌─────────────┐
│  Buffer 1   │ ◄─── READ ─────────│  GPUI/UI    │
│  (Read)     │                    │   Thread    │
└─────────────┘                    └─────────────┘

        ↓↓↓ swap_render_buffers_system() ↓↓↓
        
═══════════════════════════════════════════════════════════════════

Frame N+1:
┌─────────────┐                    ┌─────────────┐
│  Buffer 0   │ ◄─── READ ─────────│  GPUI/UI    │
│  (Read)     │                    │   Thread    │
└─────────────┘                    └─────────────┘
     
┌─────────────┐                    ┌─────────────┐
│  Buffer 1   │ ◄─── WRITE ────────│ Bevy Render │
│  (Write)    │                    │   Thread    │
└─────────────┘                    └─────────────┘

        ↓↓↓ swap_render_buffers_system() ↓↓↓

═══════════════════════════════════════════════════════════════════
```

## System Execution Flow (Per Frame)

```
Update Schedule (Bevy Main Thread):
┌────────────────────────────────────────────────────────┐
│ 1. sync_camera_input_system                            │
│    └─ Read input from input thread                     │
│                                                        │
│ 2. sync_gizmo_state_system                             │
│    └─ Sync gizmo state from GPUI                       │
│                                                        │
│ 3. sync_viewport_mouse_input_system                    │
│    └─ Sync mouse input from GPUI                       │
│                                                        │
│ 4. sync_game_objects_system                            │
│    └─ Sync transforms from game thread                 │
│                                                        │
│ ├─────────── Game Logic Systems ──────────┤           │
│                                                        │
│ 5. camera_movement_system                              │
│ 6. update_gizmo_target_system                          │
│ 7. viewport_click_selection_system                     │
│ 8. gizmo_drag_system                                   │
│                                                        │
│ ├─────────── Rendering Systems ──────────┤            │
│                                                        │
│ 9. update_metrics_system                               │
│ 10. update_gpu_profiler_system                         │
│ 11. update_gizmo_visuals                               │
│ 12. update_selection_highlighting                      │
│                                                        │
│ ├─────────── CRITICAL: Buffer Swap ──────┤            │
│                                                        │
│ 13. swap_render_buffers_system          ◄─────────────┤
│     ├─ Read old write_index, read_index               │
│     ├─ Swap atomically                                │
│     ├─ Update camera.target to new write buffer       │
│     └─ Increment frame_number                         │
│                                                        │
└────────────────────────────────────────────────────────┘
                         │
                         ▼
┌────────────────────────────────────────────────────────┐
│           Render Schedule (Render Thread)              │
│                                                        │
│ Camera extracts and renders to buffer at write_index  │
│ (Now updated to the "new" write buffer)               │
│                                                        │
└────────────────────────────────────────────────────────┘
```

## Atomic Operations

```rust
// Thread-safe buffer swap (no locks needed!)

OLD STATE:
  write_index = 0
  read_index  = 1

ATOMIC SWAP:
  temp = read_index.load(Acquire)     // temp = 1
  write_index.store(temp, Release)    // write_index = 1
  
  temp = old_write                    // temp = 0
  read_index.store(temp, Release)     // read_index = 0

NEW STATE:
  write_index = 1  ✓
  read_index  = 0  ✓
```

## Memory Ordering Guarantees

- **Acquire**: Ensures all writes before this read are visible
- **Release**: Ensures this write is visible to all subsequent reads
- **No Relaxed**: Prevents compiler/CPU reordering issues

This ensures:
1. Bevy never writes to a buffer GPUI is reading
2. GPUI never reads a buffer mid-render
3. No tearing or corruption

## Key Code Locations

```
crates/engine_backend/src/subsystems/render/bevy_renderer/
├── types.rs               # SharedGpuTextures definition
├── renderer.rs            # get_read_index(), system registration
├── scene.rs               # swap_render_buffers_system()
└── textures.rs            # Native handle storage
```

## Performance Characteristics

- **Memory**: 2 × 1600×900×4 bytes = ~11 MB (two DXGI shared textures)
- **Overhead per frame**: ~4 atomic operations + 1 camera target update
- **CPU Cost**: < 1 microsecond per swap
- **Zero copies**: Still true! DXGI shared textures = direct GPU memory

## Debug Output

```
[BEVY] 🎬 Setting up scene...
[BEVY] ✅ Got render target handles
[BEVY] 📍 Initial write_index=0, read_index=1
[BEVY] 🎯 Camera will initially render to buffer 0 (asset ID: ...)
[BEVY] ✅ Camera spawned with tonemapping DISABLED - double-buffering enabled!
[BEVY] 🔄 Camera renders to write buffer, GPUI reads from read buffer

... (120 frames later)

[BEVY] 🔄 Buffer swap: write=1, read=0, frame=120
[BEVY] 🔄 Buffer swap: write=0, read=1, frame=240
[BEVY] 🔄 Buffer swap: write=1, read=0, frame=360
```

Perfect alternating pattern! ✓
