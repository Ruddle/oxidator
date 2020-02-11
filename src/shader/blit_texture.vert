#version 450

//min_screen [0,1]
layout(location = 0) in vec2 min_screen;
layout(location = 1) in vec2 max_screen;

//min_tex [0,1]
layout(location = 2) in vec2 min_tex;
layout(location = 3) in vec2 max_tex;

layout(location = 0) out vec2 v_TexCoord;

void main() {
    vec2 tc = vec2(0.0);
    switch(gl_VertexIndex) {
        case 0: tc = vec2(1.0, 0.0); break;
        case 1: tc = vec2(1.0, 1.0); break;
        case 2: tc = vec2(0.0, 0.0); break;
        case 3: tc = vec2(0.0, 1.0); break;
    }

    v_TexCoord = min_tex +tc*(max_tex-min_tex);
    vec2 screen_px = min_screen +tc*(max_screen-min_screen);
    gl_Position = vec4(screen_px * 2.0 - 1.0, 0.5, 1.0);
}
