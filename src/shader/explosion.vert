#version 450

layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec2 v_center;
layout(location = 2) out float v_size;
layout(location = 3) out float v_life;
layout(location = 4) out float v_seed;
layout(location = 5) out vec3 v_world_pos;
layout(location = 6) out vec2 v_screen_pos;



layout(location = 0) in vec3 pos_world;
layout(location = 1) in float life;
layout(location = 2) in float seed;
layout(location = 3) in float size_world;

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


    vec4 hposition = vec4(pos_world, 1.0);
    v_world_pos = pos_world + vec3(0,0,0.5);
    vec4 sposition = cor_proj_view * hposition;

    v_size = size_world*7.0;
    //   float size_screen=  2600*v_size / sposition.w;
    float size_screen=  260*v_size / sposition.w;
    sposition/=sposition.w;


    vec2 center = sposition.xy;

    v_center = center;

    v_life =  life;
    v_seed =  seed*50.0;
    vec2 tc = vec2(0.0);
    switch(gl_VertexIndex) {
        case 0: tc = vec2(1.0, 0.0); break;
        case 1: tc = vec2(1.0, 1.0); break;
        case 2: tc = vec2(0.0, 0.0); break;
        case 3: tc = vec2(0.0, 1.0); break;
    }
    v_TexCoord = tc;

    vec2 min = -inv_resolution*size_screen;
    vec2 max = inv_resolution*size_screen;

    vec2 pos =   center  ;
     switch(gl_VertexIndex) {
        case 0: pos += vec2(max.x, min.y); break;
        case 1: pos += vec2(max.x, max.y); break;
        case 2: pos += vec2(min.x, min.y); break;
        case 3: pos += vec2(min.x, max.y); break;
    }

    v_screen_pos = pos.xy*0.5 +0.5;
    gl_Position = vec4(pos , 0.5, 1.0);
}
