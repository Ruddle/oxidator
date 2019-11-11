#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) in vec2 v_min;
layout(location = 2) in vec2 v_max;
layout(location = 3) in float v_life;
layout(location = 4) in float v_alpha;
layout(location = 0) out vec4 o_Target;

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
    vec3 color = vec3(pow(1.0- v_life,0.3),pow(v_life,1.0),0.0);
    if (v_TexCoord.x > v_life){
        color= vec3(0);
    }
    o_Target = vec4(color,v_alpha);
}
