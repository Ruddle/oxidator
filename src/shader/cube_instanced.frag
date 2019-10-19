#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) in vec3 world_pos;
layout(location = 2) in float v_param;

layout(location = 0) out vec4 o_Target;
layout(location = 1) out vec4 position_att;
layout(set = 0, binding = 1) uniform texture2D t_Color;
layout(set = 0, binding = 2) uniform sampler s_Color;

void main() {
    vec4 tex = texture(sampler2D(t_Color, s_Color), v_TexCoord);

    position_att = vec4(world_pos, v_param );

    o_Target =    mix(tex, vec4(0.5,0.5,0.5,1.0),0.01);
}
