# JSON UI Canvas

A dynamic UI system for GPUI that allows you to build user interfaces using JSON with hot reload capabilities. This provides a web-dev-like experience in native applications.

## Features

- **JSON-based UI Definition**: Define your UI structure using clean JSON syntax
- **Component References**: Reference and reuse components from separate JSON files
- **Property Interpolation**: Pass arguments to components using `${variableName}` syntax
- **Hot Reload**: Automatically updates the UI when JSON files are modified
- **Nested Components**: Support for complex nested UI structures
- **Component Children**: Components can accept and render child elements

## Basic Usage

```rust
use gpui::*;
use ui::json_ui::*;

// Create a JSON canvas view
let canvas = create_json_canvas_view("path/to/your/ui.json", cx);
```

## JSON Schema

### Basic Component Structure

```json
{
  "type": "div",
  "props": {
    "padding": 20,
    "backgroundColor": "blue"
  },
  "children": [
    {
      "type": "text",
      "children": ["Hello World"]
    }
  ]
}
```

### Component References

Reference external JSON files:

```json
{
  "$ref": "header.json",
  "props": {
    "title": "Page Title",
    "subtitle": "Page Description"
  }
}
```

### Property Interpolation

Use variables in referenced components:

```json
{
  "type": "h1",
  "props": {
    "content": "${title}"
  }
}
```

## Supported Components

- **Layout**: `div`, `flex`, `row`, `column`
- **Typography**: `h1`, `h2`, `h3`, `text`
- **Form Elements**: `button`, `input`
- **Custom**: Define your own component types in the renderer

## Supported Properties

- **Layout**: `width`, `height`, `padding`, `margin`
- **Colors**: `backgroundColor`, `color`
- **Content**: `content`, `placeholder`
- **Flex**: `direction` (for flex components)

## Example Files

See the example files in this directory:

- `app.json` - Main application layout
- `header.json` - Reusable header component
- `card.json` - Reusable card component

## Hot Reload

The system automatically watches for changes to JSON files and reloads the UI in development mode. This allows for rapid iteration and a web-like development experience.

## Development vs Release

- **Development**: JSON files are parsed and rendered dynamically with hot reload
- **Release**: The system can be extended to compile JSON directly to GPUI code for optimal performance

## Adding Custom Components

Extend the renderer in `renderer.rs` to add support for your own component types:

```rust
fn render_component_internal(component: &UiComponent, props: &HashMap<String, UiValue>, cx: &mut ViewContext<JsonCanvas>) -> impl IntoElement {
    match component.component_type.as_str() {
        "my_component" => Self::render_my_component(component, props, cx),
        // ... existing components
    }
}
```