#version 450

layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec2 a_TexCoord;
layout(location = 0) out vec2 v_TexCoord;

layout(set = 0, binding = 0) uniform Locals {
    mat4 u_Transform;
};

void main() {
    v_TexCoord = a_TexCoord;

    int max = 100;
    int x= gl_InstanceIndex%max - max/2;
    int y =(gl_InstanceIndex/max) %max  - max/2;
    int z =(gl_InstanceIndex/(max*max)) - max/2;


    float fx = float(x)*3.0;
    float fy = float(y)*3.0;
    float fz = float(z)*3.0;

    gl_Position = u_Transform * (a_Pos + vec4(fx,fy,fz,1.0));
}
