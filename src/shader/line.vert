#version 450

layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec2 v_min;
layout(location = 2) out vec2 v_max;
layout(location = 3) out float v_life;
layout(location = 4) out float v_alpha;
layout(location = 5) out float v_l;
layout(location = 6) out float v_w;

layout(location = 0) in vec2 min;
layout(location = 1) in vec2 max;
layout(location = 2) in float life;
layout(location = 3) in float alpha;

layout(set = 0, binding = 3) uniform Locals {
    vec2 mouse_pos;
    vec2 resolution;
    vec2 inv_resolution;
    vec2 start_drag;
};

void main() {
    v_min = min;
    v_max = max;
    v_life = life;
    v_alpha = alpha;
    v_l = length(max*resolution-min*resolution);
    v_w = 8;

    vec2 tc = vec2(0.0);
    switch(gl_VertexIndex) {
        case 0: tc = vec2(1.0, 0.0); break;
        case 1: tc = vec2(1.0, 1.0); break;
        case 2: tc = vec2(0.0, 0.0); break;
        case 3: tc = vec2(0.0, 1.0); break;
    }
    v_TexCoord = tc;

    vec2 u = normalize(max-min);
    vec2 ortho = vec2(u.y,-u.x)*inv_resolution*v_w/2.0;


    vec2 a = min -ortho;
    vec2 b = min +ortho;
    vec2 c = max -ortho;
    vec2 d = max +ortho;

    vec2 pos =   vec2(0.0)  ;
     switch(gl_VertexIndex) {
        case 0: pos = a; break;
        case 1: pos = b; break;
        case 2: pos = c; break;
        case 3: pos = d; break;
    }

    gl_Position = vec4(pos , 0.5, 1.0);
}
