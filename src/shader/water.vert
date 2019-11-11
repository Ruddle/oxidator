#version 450

layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) flat out int v_floor_lwall_fwall_rwall;

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
    float min = 0.0003;
    float max = 1.0-min;
    vec2 tc = vec2(0.0);
    switch(gl_VertexIndex) {
        case 0: tc = vec2(max, min); break;
        case 1: tc = vec2(max, max); break;
        case 2: tc = vec2(min, min); break;
        case 3: tc = vec2(min, max); break;
    }
    v_TexCoord = tc;

    v_floor_lwall_fwall_rwall = gl_InstanceIndex;
    float water_level = 40; 
    vec3 pos = vec3(0); 
 
    switch(v_floor_lwall_fwall_rwall){
        case 0: 
            pos = vec3(tc*hmap_size,water_level);
            break;
        case 1: 
            if(tc== vec2(max,max)){
                tc= vec2(min,min);
            }else if(tc== vec2(min,min)){
                tc= vec2(max,max);
            }
            pos = vec3(min*hmap_size.x, tc* vec2( hmap_size.x ,water_level*1.001));
            break;
        case 2: 
            pos = vec3(tc.x* hmap_size.x,min*hmap_size.y, tc.y* water_level*1.001);
            break;
        case 3: 
            pos = vec3(max*hmap_size.x, tc* vec2( hmap_size.x ,water_level*1.001));
            break;
    }

    
    gl_Position = cor_proj_view *vec4(pos,1.0);
}
