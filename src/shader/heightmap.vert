#version 450


layout(location = 0) in vec2 a_Pos;
layout(location = 1) in float mip;

layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec3 color;
layout(location = 2) out float min_lod;

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

layout(set = 1, binding = 5) uniform texture2D height_lod_tex;
layout(set = 1, binding = 6) uniform sampler height_lod_sampler;

float floor_res(int res, float val ){
    return floor(int(val)/res)*res;
}

void main() {

    int res = int(round(pow(2.0,mip)));
    vec2 cam_pos  =   vec2(floor_res(res,cam_x), floor_res(res,cam_y)); //vec2(0); //
    vec2 dim = vec2(width,height);

    vec2 pos_xy = clamp(
    a_Pos.xy+cam_pos + vec2(0.5)
    , vec2(0.0), dim);

    vec2 heightCoord = pos_xy/vec2(width,height);
    v_TexCoord =heightCoord;// a_Pos.xy / dim;

    min_lod =  texture(sampler2D(height_lod_tex, height_lod_sampler),v_TexCoord).r;

    float max_mip  = max(mip, min_lod);

    float z =  textureLod(sampler2D(height_tex, height_sampler),v_TexCoord, max_mip).r;
    vec3 pos = vec3(pos_xy,z);

    color = vec3(min_lod/4.0,min_lod/4.0,min_lod/4.0);

    gl_Position = u_Transform * ( vec4(pos,1.0) );
}
