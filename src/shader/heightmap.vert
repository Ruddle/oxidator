#version 450


layout(location = 0) in vec2 a_Pos;
layout(location = 1) in float mip;

layout(location = 0) out vec2 v_TexCoord;

layout(location = 1) out vec3 color;

layout(set = 0, binding = 0) uniform Locals {
    mat4 u_Transform;
};

layout(set = 1, binding = 0) uniform MapCfg {
    float width;
    float height;
    float ring_size;
    float cam_x;
    float cam_y;
};


layout(set = 1, binding = 3) uniform texture2D height_tex;
layout(set = 1, binding = 4) uniform sampler height_sampler;


void main() {

    vec2 cam_pos  =   vec2(cam_x, cam_y); //vec2(0); //


    vec2 dim = vec2(width,height);


    vec2 pos_xy = clamp(
    a_Pos.xy+cam_pos + vec2(0.5)
    , vec2(0.0), dim);

    vec2 heightCoord=  ( pos_xy )/ vec2(width,height);
    v_TexCoord =heightCoord;// a_Pos.xy / dim;



    float z =  textureLod(sampler2D(height_tex, height_sampler),v_TexCoord,mip).r;
    vec3 pos = vec3(pos_xy,z);

    vec2 a_xy=  pos_xy+vec2(1,0) ;
    vec3 a = vec3(a_xy, texture(sampler2D(height_tex, height_sampler),a_xy/ vec2(width,height) ).r );
    vec2 b_xy=  pos_xy+vec2(0,1) ;
    vec3 b = vec3(b_xy, texture(sampler2D(height_tex, height_sampler),b_xy/ vec2(width,height) ).r );
    vec3 normal = cross(a-pos, b-pos);


    color = vec3(mip/10.0,0.5,0.5);

    gl_Position = u_Transform * ( vec4(pos,1.0) );
}
