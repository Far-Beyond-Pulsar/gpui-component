#version 450

// Fragment shader for texture sampling
// Samples from a texture and outputs the color with alpha blending support

layout(location = 0) in vec2 fragTexCoord;

layout(location = 0) out vec4 outColor;

layout(binding = 0) uniform sampler2D texSampler;

void main() {
    outColor = texture(texSampler, fragTexCoord);
}
