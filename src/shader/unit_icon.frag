#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) in vec2 v_center;
layout(location = 2) in float v_size;
layout(location = 3) in float v_team;

layout(location = 0) out vec4 o_Target;

layout(set = 0, binding = 3) uniform Locals {
    vec2 mouse_pos;
    vec2 resolution;
    vec2 inv_resolution;
    vec2 start_drag;
};

void main() {

    vec3 color = vec3(1.0);

    if (v_team< 0){
        //Unit is selected
        color = vec3(1.0);
    }
    else if( v_team == 0.0){
        color = vec3(0,0.3,1);
    }else if(v_team < 1.1){
        color = vec3(1,0.0,0);
    }
    float alpha=1;

    float distance_from_center = length(v_TexCoord-vec2(0.5))*sqrt(2);
    float circle_radius = 0.4;
    float distance_from_circle = abs(circle_radius - distance_from_center);
    float proximity = 1-distance_from_circle;
    float thickness = 0.90;
    float proximity_thick = min(proximity,thickness)/thickness;
     alpha = pow(proximity_thick,6);

    color*= pow(alpha,2);

    o_Target = vec4(color,alpha);
}
