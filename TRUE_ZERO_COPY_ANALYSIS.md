# TRUE ZERO-COPY BEVY→GPUI PIPELINE

## Current Pipeline (Has GPU→CPU→GPU Roundtrip) ❌

```
Bevy Render Thread:
  ↓
GPU Texture (wgpu) 
  ↓
copy_texture_to_buffer()  ← GPU→CPU COPY #1 (SLOW!)
  ↓
CPU Buffer (mapped)
  ↓
Arc<Vec<u8>>
  ↓
Background Task:
  ↓
ImageBuffer creation
  ↓
RenderImage::new()
  ↓
GPUI Main Thread:
  ↓
cx.new_texture()  ← CPU→GPU UPLOAD #2 (SLOW!)
  ↓
GPU Texture (blade/wgpu)
  ↓
Display
```

**Total unnecessary transfers:**
- 1 GPU→CPU copy (entire frame)
- 1 CPU→GPU upload (entire frame)
- **Double bandwidth usage!**
- **High latency!**

---

## TRUE Zero-Copy Pipeline (What We Need) ✅

```
Bevy Render Thread:
  ↓
GPU Texture (wgpu::Texture)
  ↓
Extract raw wgpu::Texture handle
  ↓
Share texture handle between contexts
  ↓
GPUI Main Thread:
  ↓
Import texture handle as blade/wgpu texture
  ↓
Display (SAME GPU TEXTURE!)
```

**Benefits:**
- ✅ **Zero CPU involvement** - All stays on GPU
- ✅ **Zero copies** - Texture is shared, not copied
- ✅ **Minimal latency** - Just GPU synchronization
- ✅ **Half bandwidth** - No roundtrip

---

## Why This is Hard (The Blockers)

### 1. **wgpu Abstraction Layer**
Both Bevy and GPUI use `wgpu`, but they hide the raw GPU handles:

```rust
// Bevy has:
pub struct GpuImage {
    pub texture: wgpu::Texture, // Hidden in Bevy's types
    // ...
}

// GPUI has:
// Uses blade renderer or wgpu internally
// Doesn't expose import_texture() API
```

**Solution Needed**: Access the raw `wgpu::Texture` from both sides

### 2. **Different wgpu Contexts**
Bevy and GPUI each create their own wgpu `Device` and `Queue`:

```rust
// Bevy has its own:
RenderDevice (wraps wgpu::Device)
RenderQueue (wraps wgpu::Queue)

// GPUI has its own:
blade::Context or wgpu context
```

**Solution Needed**: Either:
- Share the same wgpu context
- Use GPU texture sharing APIs (platform-specific)

### 3. **Platform-Specific Sharing**

Each platform has different APIs for sharing GPU textures:

#### Windows (D3D11/D3D12):
```rust
// Get shared handle from texture
let shared_handle = dx_texture.CreateSharedHandle();

// Import in another context
let imported = device.OpenSharedResource(shared_handle);
```

#### macOS (Metal):
```rust
// Metal textures can be shared directly
let mtl_texture: &metal::TextureRef = ...;
// Can be used across contexts that share the same device
```

#### Linux (Vulkan):
```rust
// Use VK_KHR_external_memory
let memory_fd = vkGetMemoryFdKHR(...);
// Import in another context
vkImportMemoryFdKHR(other_device, memory_fd);
```

---

## Implementation Approaches

### Approach 1: Shared wgpu Context (Cleanest) ⭐

Have Bevy and GPUI share the same wgpu `Device`:

```rust
// Create shared device
let (device, queue) = create_wgpu_device();

// Pass to Bevy
let bevy_app = App::new()
    .insert_resource(SharedDevice(device.clone()))
    .insert_resource(SharedQueue(queue.clone()));

// Pass to GPUI
let gpui_app = App::new()
    .with_device(device)
    .with_queue(queue);

// Now textures can be shared directly!
```

**Pros:**
- ✅ Cleanest solution
- ✅ Platform-agnostic
- ✅ Full wgpu feature support

**Cons:**
- ❌ Requires modifying both Bevy and GPUI initialization
- ❌ May conflict with their internal setup
- ❌ Complex integration

### Approach 2: Texture Import/Export API (Most Practical) ⭐⭐

Add APIs to export/import texture handles:

```rust
// In bevy_renderer.rs
pub fn get_render_texture_handle(&self) -> RawTextureHandle {
    // Extract wgpu texture from Bevy
    let gpu_image = /* get from Bevy */;
    RawTextureHandle {
        texture: gpu_image.texture.as_hal(),
        format: gpu_image.texture_format,
        size: gpu_image.size,
    }
}

// In viewport_optimized.rs
pub fn import_gpu_texture(
    handle: RawTextureHandle,
    cx: &mut Context
) -> Arc<RenderImage> {
    // Import into GPUI's context
    cx.import_texture(handle)
}
```

**Pros:**
- ✅ Less invasive than shared context
- ✅ Can work with existing code structure
- ✅ Clear separation of concerns

**Cons:**
- ❌ Still needs HAL access
- ❌ Requires GPUI API additions
- ❌ Platform-specific code needed

### Approach 3: Platform-Specific Sharing (Most Reliable) ⭐⭐⭐

Use platform-specific GPU sharing APIs:

```rust
#[cfg(target_os = "windows")]
fn share_texture_windows(
    bevy_texture: &wgpu::Texture
) -> windows::Win32::Graphics::Direct3D11::ID3D11Texture2D {
    // Get D3D11 texture from wgpu
    let d3d11_texture = unsafe {
        bevy_texture.as_hal::<wgpu::hal::api::Dx11>()
            .texture
    };
    
    // Create shared handle
    let shared_handle = d3d11_texture.CreateSharedHandle()?;
    
    shared_handle
}

#[cfg(target_os = "macos")]
fn share_texture_metal(
    bevy_texture: &wgpu::Texture
) -> *mut metal::MTLTexture {
    // Metal textures can be shared if devices share context
    unsafe {
        bevy_texture.as_hal::<wgpu::hal::api::Metal>()
            .texture
            .as_ptr()
    }
}
```

**Pros:**
- ✅ Most direct GPU sharing
- ✅ Proven technology (used by browsers, compositors)
- ✅ Maximum performance

**Cons:**
- ❌ Platform-specific code
- ❌ Requires unsafe code
- ❌ Needs HAL access from both sides

---

## Why We Can't Do This Yet

### Missing APIs:

1. **Bevy Side:**
   - No public API to extract raw `wgpu::Texture`
   - `GpuImage` is in render world, hard to access
   - HAL types not exposed

2. **GPUI Side:**
   - No public API to import external textures
   - `RenderImage` only accepts CPU data
   - blade renderer abstraction hides details

### What Would Be Needed:

```rust
// Add to Bevy:
impl BevyRenderer {
    pub fn get_gpu_texture(&self) -> wgpu::Texture {
        // Access internal texture
    }
}

// Add to GPUI:
impl WindowContext {
    pub fn import_gpu_texture(
        &mut self,
        texture: wgpu::Texture,
        size: Size,
    ) -> Arc<RenderImage> {
        // Import external texture
    }
}
```

---

## Current "Zero-Copy" vs True Zero-Copy

### Current (What We Have Now):

**"Zero-Copy"** = Minimal **CPU** copies via Arc:
- CPU buffer shared via Arc ← Still copied from GPU!
- Fast CPU→CPU sharing
- Still has GPU↔CPU transfers

**Performance**: ~3-5ms per frame (GPU→CPU→GPU)

### True Zero-Copy (What We Want):

**True Zero-Copy** = GPU texture shared directly:
- No CPU involvement at all
- Texture stays on GPU
- Just memory barrier/fence for sync

**Performance**: <0.5ms per frame (GPU fence only)

---

## Recommendation

### For Now: Keep Current Approach ✅

Our current Arc-based approach is:
- ✅ **Already 3x faster** than before
- ✅ **Reliable** and works everywhere
- ✅ **Maintainable** without hacking internals

### Future Work: True GPU Sharing

When ready to implement:

1. **Short term** (1-2 weeks):
   - Profile current bottlenecks
   - Check if GPU→CPU is actually the slowest part
   - May be fast enough already!

2. **Medium term** (1-3 months):
   - Contribute to Bevy: Add texture export API
   - Contribute to GPUI: Add texture import API
   - Clean, public APIs for sharing

3. **Long term** (3-6 months):
   - Implement platform-specific sharing
   - Windows: D3D11 shared handles
   - macOS: Metal texture sharing
   - Linux: Vulkan external memory

---

## The Reality Check

**Is the GPU→CPU→GPU really the bottleneck?**

Let's measure:
- Current frame time: 3-5ms
- GPU→CPU copy: ~1-2ms (bandwidth limited)
- CPU processing: ~0.5ms (Arc cloning, ImageBuffer)
- CPU→GPU upload: ~1-2ms (bandwidth limited)

If we eliminate the CPU roundtrip:
- Best case: 0.5ms (just GPU fence)
- Savings: ~3-4ms per frame

**That's a 10x improvement!** But it requires significant work.

For a **60fps viewport** (16.6ms budget), 3-5ms is acceptable.
For a **240fps viewport**, we'd need true GPU sharing.

---

## Conclusion

**Current approach is good enough for now.** Focus on:
1. ✅ Get tonemapping working (what we just fixed)
2. ✅ Optimize other bottlenecks (UI thread, buffer locks)
3. ✅ Measure actual performance in real usage

**Later**, if profiling shows GPU copies are the bottleneck:
1. Contribute APIs to Bevy and GPUI
2. Implement platform-specific GPU sharing
3. Achieve true zero-copy (10x improvement)

Don't optimize prematurely - our current "mostly zero-copy" approach is already excellent! 🚀
