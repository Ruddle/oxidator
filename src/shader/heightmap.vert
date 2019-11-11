#version 450


layout(location = 0) in vec2 a_Pos;
layout(location = 1) in float mip;

layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec3 color;
layout(location = 2) out float min_lod;
layout(location = 3) out float max_mip;

layout(set = 0, binding = 0) uniform Locals {
    mat4 cor_proj_view;
    mat4 u_View;
    mat4 u_proj;
    mat4 u_Normal;
    vec2 mouse_pos;
    vec2 resolution;
    vec2 inv_resolution;
    vec2 start_drag;
    float pen_radius;
    float pen_strength;
    vec2 hmap_size;
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

    int res = int(round(pow(2.0,(mip))));
    vec2 cam_pos  =   vec2(floor_res(res,cam_x), floor_res(res,cam_y)); //vec2(266.987); //
    vec2 dim = hmap_size ;
    vec2 pos_xy =  a_Pos.xy+cam_pos + vec2(0.5);
    v_TexCoord =pos_xy/dim;
    min_lod =  texture(sampler2D(height_lod_tex, height_lod_sampler),v_TexCoord).r;
    max_mip  = max(mip, min_lod);
    float z =  textureLod(sampler2D(height_tex, height_sampler),v_TexCoord, max_mip).r;
    vec3 pos = vec3(pos_xy,z);

    color= vec3(max_mip/4);
    float rock_bottom = -40.0;
    float f = fract(max_mip);
    float stride = pow(2,ceil(max_mip));
   

   
    if(pos_xy.x<-stride){
        pos.x= 0.0;
        pos.z = rock_bottom;    
        color = vec3(0,0,1);
    } else if(pos_xy.x<=0.0){
        pos.x= 0.0;
        color = vec3(1,0,0);
    }

  
    if(pos_xy.x> width+stride){
        pos.x= width;
        pos.z = rock_bottom;    
        color = vec3(0,1,1);
    } else
    if(pos_xy.x> width){
        pos.x= width;
        color = vec3(1,0,0);
    }

 
    if(pos_xy.y> height + stride){
        pos.y= height+0;
        pos.z = rock_bottom;  
        color = vec3(0,0,1);  
    }else
    if(pos_xy.y>height){
        pos.y= height;
        color = vec3(1,0,0);
    }

     if(pos_xy.y<-stride){
        pos.y= 0;
        pos.z = rock_bottom;  
        color = vec3(0,0,1);  
    }else
    if(pos_xy.y<-0.0){
        pos.y= 0;
        color = vec3(1,0,0);
    }

   

    v_TexCoord =pos_xy/dim;

    gl_Position = cor_proj_view * ( vec4(pos,1.0) );
}
