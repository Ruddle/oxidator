#version 450

layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec2 v_min;
layout(location = 2) out vec2 v_max;
layout(location = 3) out float v_life;
layout(location = 4) out float v_alpha;

layout(location = 0) in vec2 min;
layout(location = 1) in vec2 max;
layout(location = 2) in float life;
layout(location = 3) in float alpha;


void main() {
    v_min = min;
    v_max = max;
    v_life = life;
    v_alpha = alpha;

    vec2 tc = vec2(0.0);
    switch(gl_VertexIndex) {
        case 0: tc = vec2(1.0, 0.0); break;
        case 1: tc = vec2(1.0, 1.0); break;
        case 2: tc = vec2(0.0, 0.0); break;
        case 3: tc = vec2(0.0, 1.0); break;
    }
    v_TexCoord = tc;

    vec2 pos =   vec2(0.0)  ;
     switch(gl_VertexIndex) {
        case 0: pos = vec2(max.x, min.y); break;
        case 1: pos = vec2(max.x, max.y); break;
        case 2: pos = vec2(min.x, min.y); break;
        case 3: pos = vec2(min.x, max.y); break;
    }

    gl_Position = vec4(pos , 0.5, 1.0);
}
