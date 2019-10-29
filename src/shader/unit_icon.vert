#version 450

layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec2 v_center;
layout(location = 2) out float v_size;
layout(location = 3) out float v_team;


layout(location = 0) in vec2 center;
layout(location = 1) in float size;
layout(location = 2) in float team;

layout(set = 0, binding = 3) uniform Locals {
    vec2 mouse_pos;
    vec2 resolution;
    vec2 inv_resolution;
    vec2 start_drag;
};

void main() {
    v_center = center;
    v_size = size;
    v_team = team;

    vec2 tc = vec2(0.0);
    switch(gl_VertexIndex) {
        case 0: tc = vec2(1.0, 0.0); break;
        case 1: tc = vec2(1.0, 1.0); break;
        case 2: tc = vec2(0.0, 0.0); break;
        case 3: tc = vec2(0.0, 1.0); break;
    }
    v_TexCoord = tc;

    vec2 min = -inv_resolution*size;
    vec2 max = inv_resolution*size;

    vec2 pos =   center  ;
     switch(gl_VertexIndex) {
        case 0: pos += vec2(max.x, min.y); break;
        case 1: pos += vec2(max.x, max.y); break;
        case 2: pos += vec2(min.x, min.y); break;
        case 3: pos += vec2(min.x, max.y); break;
    }

    gl_Position = vec4(pos , 0.5, 1.0);
}
