#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) in vec2 v_min;
layout(location = 2) in vec2 v_max;
layout(location = 3) in float v_life;
layout(location = 4) in float v_alpha;
layout(location = 5) in float v_l;
layout(location = 6) in float v_w;
layout(location = 0) out vec4 o_Target;

layout(set = 0, binding = 3) uniform Locals {
    vec2 mouse_pos;
    vec2 resolution;
    vec2 inv_resolution;
    vec2 start_drag;
};

void main() {
    float alpha = 1-abs(v_TexCoord.y-0.5)/0.5;
    alpha = pow(alpha,2);
    

    alpha = min(alpha, pow(sin(v_TexCoord.x*v_l*0.2)*0.5+0.5,2)  )  ;

    vec3 color = vec3(0,0.5+ 0.5*pow(alpha,0.7),pow(alpha,0.7)*0.5);
    o_Target = vec4(color,alpha);
}
