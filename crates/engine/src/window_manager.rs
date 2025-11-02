/// Window Manager for Winit + GPUI integration
/// 
/// This module provides helper functions to create Winit windows with embedded GPUI.
/// The actual rendering and event handling is still done in main.rs for now.

use gpui::*;
use winit::window::{Window as WinitWindow, WindowAttributes};
use winit::event_loop::ActiveEventLoop;
use std::sync::Arc;

/// Create a new Winit window with GPUI embedded
/// Returns the winit window and the GPUI window handle
pub fn create_gpui_window<V: 'static + Render>(
    event_loop: &ActiveEventLoop,
    gpui_app: &mut Application,
    title: &str,
    width: u32,
    height: u32,
    build_root_view: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> anyhow::Result<(Arc<WinitWindow>, WindowHandle<V>)> {
    use raw_window_handle::HasWindowHandle;
    
    // Create winit window
    let window_attrs = WindowAttributes::default()
        .with_title(title)
        .with_inner_size(winit::dpi::LogicalSize::new(width, height));
        
    let winit_window = Arc::new(event_loop.create_window(window_attrs)?);
    
    println!("✅ Creating window '{}': {}x{}", title, width, height);
    
    // Get window handle and create external handle for GPUI
    let scale_factor = winit_window.scale_factor() as f32;
    let size = winit_window.inner_size();
    let logical_width = size.width as f32 / scale_factor;
    let logical_height = size.height as f32 / scale_factor;
    
    let bounds = Bounds {
        origin: point(px(0.0), px(0.0)),
        size: gpui::size(px(logical_width), px(logical_height)),
    };
    
    println!("🎯 GPUI window bounds: physical {}x{}, scale {}, logical {}x{}",
        size.width, size.height, scale_factor, logical_width, logical_height);
    
    let gpui_raw_handle = winit_window
        .window_handle()
        .expect("Failed to get window handle")
        .as_raw();
    
    let external_handle = ExternalWindowHandle {
        raw_handle: gpui_raw_handle,
        bounds,
        scale_factor,
        surface_handle: None,
    };
    
    // Open GPUI window using external window API
    let gpui_window = gpui_app.open_window_external(external_handle, build_root_view)?;
    
    println!("✅ GPUI window '{}' opened!", title);
    
    Ok((winit_window, gpui_window))
}

// Helper functions for event conversion
pub fn convert_mouse_button(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Back => MouseButton::Navigate(NavigationDirection::Back),
        winit::event::MouseButton::Forward => MouseButton::Navigate(NavigationDirection::Forward),
        winit::event::MouseButton::Other(_) => MouseButton::Left,
    }
}

pub fn convert_modifiers(winit_mods: &winit::keyboard::ModifiersState) -> Modifiers {
    Modifiers {
        control: winit_mods.control_key(),
        alt: winit_mods.alt_key(),
        shift: winit_mods.shift_key(),
        platform: winit_mods.super_key(),
        function: false,
    }
}

// Helper to convert KeyCode to string (static so it can be used without &self borrow)
pub fn keycode_to_string(code: winit::keyboard::KeyCode) -> Option<String> {
    use winit::keyboard::KeyCode::*;
    Some(match code {
        // Letters
        KeyA => "a", KeyB => "b", KeyC => "c", KeyD => "d", KeyE => "e",
        KeyF => "f", KeyG => "g", KeyH => "h", KeyI => "i", KeyJ => "j",
        KeyK => "k", KeyL => "l", KeyM => "m", KeyN => "n", KeyO => "o",
        KeyP => "p", KeyQ => "q", KeyR => "r", KeyS => "s", KeyT => "t",
        KeyU => "u", KeyV => "v", KeyW => "w", KeyX => "x", KeyY => "y", KeyZ => "z",
        
        // Numbers
        Digit0 => "0", Digit1 => "1", Digit2 => "2", Digit3 => "3", Digit4 => "4",
        Digit5 => "5", Digit6 => "6", Digit7 => "7", Digit8 => "8", Digit9 => "9",
        
        // Special keys
        Space => "space", Enter => "enter", Tab => "tab", Backspace => "backspace",
        Escape => "escape", Delete => "delete", Insert => "insert",
        Home => "home", End => "end", PageUp => "pageup", PageDown => "pagedown",
        
        // Arrow keys
        ArrowUp => "up", ArrowDown => "down", ArrowLeft => "left", ArrowRight => "right",
        
        // Function keys
        F1 => "f1", F2 => "f2", F3 => "f3", F4 => "f4",
        F5 => "f5", F6 => "f6", F7 => "f7", F8 => "f8",
        F9 => "f9", F10 => "f10", F11 => "f11", F12 => "f12",
        
        // Punctuation and symbols
        Minus => "-", Equal => "=", BracketLeft => "[", BracketRight => "]",
        Backslash => "\\", Semicolon => ";", Quote => "'",
        Comma => ",", Period => ".", Slash => "/", Backquote => "`",
        
        _ => return None,
    }.to_string())
}
