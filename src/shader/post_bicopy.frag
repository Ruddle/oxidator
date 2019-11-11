#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 0) uniform texture2D t_color;
layout(set = 1, binding = 1) uniform sampler s_color;

layout(set = 0, binding = 0) uniform Locals {
    mat4 cor_proj_view;
    mat4 u_View;
    mat4 u_proj;
    mat4 u_Normal;
    vec2 mouse_pos;
    vec2 resolution;
    vec2 inv_resolution;
    vec2 start_drag;
    float pen_radius;
    float pen_strength;
    vec2 hmap_size;
};

void main() {
    vec3 color =  texture(sampler2D(t_color, s_color), v_TexCoord).rgb;
    o_Target = vec4(color,1.0);
}
