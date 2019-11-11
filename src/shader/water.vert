#version 450

layout(location = 0) out vec2 v_TexCoord;


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
    float min = 0.0002;
    float max = 1.0-min;
    vec2 tc = vec2(0.0);
    switch(gl_VertexIndex) {
        case 0: tc = vec2(max, min); break;
        case 1: tc = vec2(max, max); break;
        case 2: tc = vec2(min, min); break;
        case 3: tc = vec2(min, max); break;
    }
    v_TexCoord = tc;
    vec2 pos =tc*hmap_size;
    gl_Position = cor_proj_view *vec4(pos,40.0,1.0);
}
