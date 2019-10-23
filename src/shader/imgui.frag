#version 450

layout(set = 1, binding = 0) uniform texture2D u_Texture;
layout(set = 1, binding = 1) uniform sampler u_Sampler;

layout(location = 0) in vec2 v_UV;
layout(location = 1) in vec4 v_Color;

layout(location = 0) out vec4 o_Target;

void main() {
  o_Target = v_Color * texture(sampler2D(u_Texture, u_Sampler), v_UV);
}