#version 450

layout(location = 0) in vec3 a_Pos;
layout(location = 1) in vec3 a_Nor;
layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec3 color;

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
    color=   a_Nor;// (a_Pos+vec3(0,0,48)) / vec3(width,height,100);
    color.z=0;
    gl_Position = u_Transform * ( vec4(a_Pos,1.0) );
}
