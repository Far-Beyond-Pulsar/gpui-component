# JSON UI Examples

This directory contains example JSON files that demonstrate the JSON UI Canvas system with hot reload capabilities.

## Files

- **`basic.json`** - Simple example with basic components
- **`complex.json`** - Complex example using component references and nested structures
- **`interactive.json`** - Interactive example showcasing hot reload features
- **`header_component.json`** - Reusable header component with property interpolation
- **`card_component.json`** - Reusable card component
- **`interactive_card.json`** - Special card for demonstrating hot reload
- **`nested_example.json`** - Shows nested component references
- **`deeply_nested.json`** - Demonstrates deep component nesting

## Hot Reload Testing

1. Run the story application
2. Navigate to the "JSON UI Canvas" story
3. Load one of the example files
4. Open the JSON file in your text editor
5. Make changes and save - watch the UI update instantly!

## Try These Changes

- Change `"backgroundColor": "blue"` to `"backgroundColor": "red"`
- Edit any text content
- Modify component props like `"title"` or `"content"`
- Add new child components
- Change layout properties like `"padding"` or `"margin"`

The hot reload system watches all JSON files and their dependencies, automatically updating the UI when any file changes.