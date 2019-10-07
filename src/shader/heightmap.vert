#version 450

layout(location = 0) in vec4 a_Pos;

layout(location = 0) out vec2 v_TexCoord;


layout(set = 0, binding = 0) uniform Locals {
    mat4 u_Transform;
};

layout(set = 1, binding = 0) uniform MapCfg {
    int width;
    int height;
    int width_n;
    int height_n;
    int chunk_size;
};

void main() {
    v_TexCoord = a_Pos.xy / vec2(width,height);
    gl_Position = u_Transform * ( vec4(a_Pos.xyz,1.0) );
}
