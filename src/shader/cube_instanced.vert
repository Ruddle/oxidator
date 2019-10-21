#version 450

layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec2 a_TexCoord;

layout(location = 2) in vec4 mata;
layout(location = 3) in vec4 matb;
layout(location = 4) in vec4 matc;
layout(location = 5) in vec4 matd;
layout(location = 6) in float selected;
layout(location = 7) in float team;


layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec3 world_pos;
layout(location = 2) out float v_selected;
layout(location = 3) out float v_team;

layout(set = 0, binding = 0) uniform Locals {
    mat4 u_Transform;
};

void main() {
    v_TexCoord = a_TexCoord;
    v_team = team;
    v_selected = selected;

    mat4 t = mat4(mata,matb,matc,matd);

    gl_Position = u_Transform *t*a_Pos;
//    world_pos = a_Pos.xyz +a_off;
}
