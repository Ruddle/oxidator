#version 450

layout(location = 0) in vec2 a_Pos;



layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec3 color;

layout(set = 0, binding = 0) uniform Locals {
    mat4 u_Transform;
};

layout(set = 1, binding = 0) uniform MapCfg {
    int width;
    int height;
};


layout(set = 1, binding = 3) uniform texture2D height_tex;
layout(set = 1, binding = 4) uniform sampler height_sampler;


void main() {
    v_TexCoord = a_Pos.xy / vec2(width,height);





    float z =  texture(sampler2D(height_tex, height_sampler),v_TexCoord ).r;



    color = vec3(a_Pos, z )/ vec3(width,height,1.0);
    gl_Position = u_Transform * ( vec4(vec3(a_Pos,z),1.0) );
}
