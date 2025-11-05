#!/bin/bash
# Script to compile GLSL shaders to SPIR-V
# Requires glslangValidator or glslc to be installed

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Try to find a GLSL compiler
if command -v glslangValidator &> /dev/null; then
    COMPILER="glslangValidator"
elif command -v glslc &> /dev/null; then
    COMPILER="glslc"
else
    echo "Error: No GLSL compiler found. Please install glslang or shaderc."
    echo "On NixOS: nix-shell -p vulkan-tools shaderc"
    echo "On Ubuntu: sudo apt install glslang-tools"
    exit 1
fi

echo "Using compiler: $COMPILER"

# Compile vertex shader
echo "Compiling fullscreen.vert..."
if [ "$COMPILER" = "glslangValidator" ]; then
    "$COMPILER" -V "$SCRIPT_DIR/fullscreen.vert" -o "$SCRIPT_DIR/fullscreen.vert.spv"
else
    "$COMPILER" "$SCRIPT_DIR/fullscreen.vert" -o "$SCRIPT_DIR/fullscreen.vert.spv"
fi

# Compile fragment shader
echo "Compiling texture.frag..."
if [ "$COMPILER" = "glslangValidator" ]; then
    "$COMPILER" -V "$SCRIPT_DIR/texture.frag" -o "$SCRIPT_DIR/texture.frag.spv"
else
    "$COMPILER" "$SCRIPT_DIR/texture.frag" -o "$SCRIPT_DIR/texture.frag.spv"
fi

echo "Shader compilation complete!"
