#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 0) uniform texture2D t_pos;
layout(set = 1, binding = 1) uniform sampler s_pos;
layout(set = 0, binding = 3) uniform Locals {
    vec2 mouse_pos;
};

void main() {

    vec3 mouse_pos_world = texture(sampler2D(t_pos, s_pos), mouse_pos).xyz;
    vec3 pos = texture(sampler2D(t_pos, s_pos), v_TexCoord).xyz;

    float dist = length(mouse_pos_world - pos);

    float distance_to_sphere  = length(10.0 - dist);

    float base = clamp(1-distance_to_sphere,0.0,1.0);
    float alpha=  pow(base,2.0);

    o_Target = vec4(vec3(1.0),alpha);
}
