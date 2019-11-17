#version 450

layout(location = 0) in vec4 a_Pos;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_TexCoord;

layout(location = 3) in vec3 inst_pos;
layout(location = 4) in vec3 inst_euler;
layout(location = 5) in float selected;
layout(location = 6) in float team;


layout(location = 0) out vec2 v_TexCoord;
layout(location = 1) out vec3 world_pos;

layout(location = 2) out float v_selected;
layout(location = 3) out float v_team;
layout(location = 4) out vec3 v_world_normal;
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

void main() {
    v_TexCoord = a_TexCoord;
    v_team = team;
    v_selected = selected;

    float sr = sin(inst_euler.x);
    float cr = cos(inst_euler.x);
    float sp = sin(inst_euler.y);
    float cp = cos(inst_euler.y);
    float sy = sin(inst_euler.z);
    float cy = cos(inst_euler.z);

    // mat4 t = mat4(
    //     cy * cp,  cy * sp * sr - sy * cr,  cy * sp * cr + sy * sr,  inst_pos.x, 
    //     sy * cp,  sy * sp * sr + cy * cr,  sy * sp * cr - cy * sr,  inst_pos.y,
    //              -sp,            cp * sr,            cp * cr,  inst_pos.z, 
    //              0,0,0,1);

    mat4 t = mat4(
        cy * cp,                 sy * cp,                -sp                ,0, 
        cy * sp * sr - sy * cr,  sy * sp * sr + cy * cr, cp * sr            ,0,
        cy * sp * cr + sy * sr,  sy * sp * cr - cy * sr, cp * cr            ,0, 
        inst_pos.x,              inst_pos.y            , inst_pos.z         ,1);

    mat3 tn = mat3(
    cy * cp,                 sy * cp,                -sp                ,
    cy * sp * sr - sy * cr,  sy * sp * sr + cy * cr, cp * sr            ,
    cy * sp * cr + sy * sr,  sy * sp * cr - cy * sr, cp * cr            ); 
             

    vec4 world_pos4 = t * a_Pos;
    world_pos = world_pos4.xyz/world_pos4.w;
    gl_Position = cor_proj_view*vec4(world_pos+vec3(0.0),1.0);//  cor_proj_view * t *a_Pos;

    v_world_normal = tn* a_normal;
 
}
