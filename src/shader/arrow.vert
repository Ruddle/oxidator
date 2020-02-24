#version 450

layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec2 a_TexCoord;

layout(location = 2) in vec4 mata;
layout(location = 3) in vec4 matb;
layout(location = 4) in vec4 matc;
layout(location = 5) in vec4 matd;
layout(location = 6) in vec4 color;



layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec3 world_pos;
layout(location = 2) out vec4 v_color;


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
    v_TexCoord = a_TexCoord;
 
    v_color = color;

    float dist = color.w;

    vec3 pos = a_Pos.xyz;
    if (a_Pos.z>0.0){
        pos.z +=dist;
    } else if(a_Pos.z<=0.0){
        pos.z =0;
    }

    mat4 t = mat4(mata,matb,matc,matd);

    gl_Position = cor_proj_view *t* vec4(pos,1.0);
    world_pos = a_Pos.xyz;
}
