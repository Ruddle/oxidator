#version 450


layout(location = 0) in vec2 a_Pos;
layout(location = 1) in float mip;

layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec3 color;
layout(location = 2) out float min_lod;
layout(location = 3) out float max_mip;

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

    // vec2 pos_xy = clamp(
    // a_Pos.xy+cam_pos + vec2(0.5)
    // , vec2(0.0), dim);

     vec2 pos_xy =  a_Pos.xy+cam_pos + vec2(0.5);

 

    v_TexCoord =pos_xy/dim;

    min_lod =  texture(sampler2D(height_lod_tex, height_lod_sampler),v_TexCoord).r;
    max_mip  = max(mip, min_lod);

    float z =  textureLod(sampler2D(height_tex, height_sampler),v_TexCoord, max_mip).r;
    vec3 pos = vec3(pos_xy,z);

    float rock_bottom = -40.0;
    if(pos_xy.y<-0.0){
        pos.y= 0;
        pos.z = rock_bottom;    
    }

    if(pos_xy.x<-0.0){
        pos.x= 0;
        pos.z = rock_bottom;    
    }


    if(pos_xy.x>width - pow(2,max_mip) ){
        pos.x= width;
    }

    if(pos_xy.x>width  ){
        pos.x= width;
        pos.z = rock_bottom;    
    }

    if(pos_xy.y>height){
        pos.y= height;
        pos.z = rock_bottom;    
    }

    color = vec3(min_lod/4.0,min_lod/4.0,min_lod/4.0);

    gl_Position = u_Transform * ( vec4(pos,1.0) );
}
