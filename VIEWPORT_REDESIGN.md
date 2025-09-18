# Viewport Component - Rewritten Architecture

The viewport component has been completely rewritten to separate rendering concerns and provide zero-copy buffer access with proper GPUI refresh mechanisms.

## Key Changes

### 1. Removed Built-in Render Engine
- No more `RenderEngine` trait in the viewport
- No more `TestRenderEngine` implementing viewport-specific interfaces
- Rendering is now the responsibility of external components

### 2. Zero-Copy Buffer System
- `ViewportBuffers` provides direct access to front/back framebuffers
- `with_front_buffer()` and `with_back_buffer()` closures avoid copying data
- `swap_buffers()` atomically swaps front and back buffers

### 3. GPUI Refresh Hook
- `RefreshHook` is a function that external render engines can call
- Triggers GPUI background task to refresh the viewport
- Thread-safe and can be called from any thread

### 4. Simplified Constructor
- `Viewport::new()` returns `(Viewport, ViewportBuffers, RefreshHook)`
- No render engine parameter required
- Any render engine can now be hooked up externally

## API Usage

### Creating a Viewport
```rust
let (viewport, buffers, refresh_hook) = Viewport::new(800, 600, FramebufferFormat::Rgba8, cx);
```

### Zero-Copy Rendering
```rust
// Render to back buffer (zero-copy)
buffers.with_back_buffer(|back_buffer| {
    // Directly modify the framebuffer data
    your_render_engine.render_to(back_buffer);
});

// Make rendered frame visible
buffers.swap_buffers();

// Trigger GPUI refresh
refresh_hook();
```

### Reading Buffer Data
```rust
// Read from front buffer (zero-copy)
let pixel_data = buffers.with_front_buffer(|front_buffer| {
    // Read specific pixels or analyze the buffer
    front_buffer.buffer[0..4] // First pixel RGBA
});
```

## Integration with External Render Engines

### Background Thread Pattern
```rust
fn spawn_render_thread(your_engine: YourRenderEngine, buffers: ViewportBuffers, refresh_hook: RefreshHook) {
    thread::spawn(move || {
        loop {
            // Render to back buffer
            buffers.with_back_buffer(|back_buffer| {
                your_engine.render(back_buffer);
            });
            
            // Swap and refresh
            buffers.swap_buffers();
            refresh_hook();
            
            // Control frame rate
            thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }
    });
}
```

### Immediate Mode Pattern
```rust
fn render_frame(your_engine: &mut YourRenderEngine, buffers: &ViewportBuffers, refresh_hook: &RefreshHook) {
    buffers.with_back_buffer(|back_buffer| {
        your_engine.render(back_buffer);
    });
    buffers.swap_buffers();
    refresh_hook();
}
```

## Benefits

1. **Separation of Concerns**: Viewport handles display, external engines handle rendering
2. **Zero-Copy Performance**: Direct buffer access without memory copying
3. **Thread Safety**: Safe to render from background threads
4. **Flexibility**: Any render engine can be integrated
5. **GPUI Integration**: Proper refresh mechanism that works with GPUI's architecture

## Buffer Formats

Supported framebuffer formats:
- `FramebufferFormat::Rgba8` - 4 bytes per pixel (default)
- `FramebufferFormat::Rgb8` - 3 bytes per pixel
- `FramebufferFormat::Bgra8` - 4 bytes per pixel (Windows native)
- `FramebufferFormat::Bgr8` - 3 bytes per pixel

The viewport automatically converts between formats when updating textures for GPUI display.

## Example Integration

See `examples/viewport_example.rs` for a complete working example that shows:
- Creating a viewport with buffer access
- Integrating the TestRenderEngine externally
- Background thread rendering
- Proper GPUI refresh handling

## Migration from Old API

### Before (Old API)
```rust
let viewport = Viewport::new(render_engine, 800, 600, cx);
viewport.request_render();
```

### After (New API)
```rust
let (viewport, buffers, refresh_hook) = Viewport::new(800, 600, FramebufferFormat::Rgba8, cx);
spawn_your_render_thread(your_engine, buffers, refresh_hook);
```

The new API provides much more control and better performance while maintaining the same visual results.
