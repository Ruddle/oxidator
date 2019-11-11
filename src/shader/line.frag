#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) in vec2 v_min;
layout(location = 2) in vec2 v_max;
layout(location = 3) in float v_life;
layout(location = 4) in float v_alpha;
layout(location = 5) in float v_l;
layout(location = 6) in float v_w;
layout(location = 0) out vec4 o_Target;

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
    float alpha = 1-abs(v_TexCoord.y-0.5)/0.5;
    alpha = pow(alpha,2);
    

    alpha = min(alpha, pow(sin(v_TexCoord.x*v_l*0.2)*0.5+0.5,2)  )  ;

    vec3 color = vec3(0,0.5+ 0.5*pow(alpha,0.7),pow(alpha,0.7)*0.5);
    o_Target = vec4(color,alpha);
}
