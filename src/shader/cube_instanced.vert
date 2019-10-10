#version 450

layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec2 a_TexCoord;
layout(location = 2) in vec3 a_off;

layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec3 world_pos;
layout(set = 0, binding = 0) uniform Locals {
    mat4 u_Transform;
};

void main() {
    v_TexCoord = a_TexCoord;

    gl_Position = u_Transform * (a_Pos + vec4(a_off,0.0));
    world_pos = a_Pos.xyz +a_off;
}
