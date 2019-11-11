#version 450

layout(location = 0) out vec2 v_TexCoord;


layout(set = 0, binding = 0) uniform Locals {
    mat4 u_Transform;
};

void main() {
    vec2 tc = vec2(0.0);
    switch(gl_VertexIndex) {
        case 0: tc = vec2(1.0, 0.0); break;
        case 1: tc = vec2(1.0, 1.0); break;
        case 2: tc = vec2(0.0, 0.0); break;
        case 3: tc = vec2(0.0, 1.0); break;
    }
    v_TexCoord = tc;
    vec2 pos = tc*2048;
    gl_Position = u_Transform *vec4(pos,40.0,1.0);
}
