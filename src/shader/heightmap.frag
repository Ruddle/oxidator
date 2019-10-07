#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 0) out vec4 o_Target;
layout(set = 0, binding = 1) uniform texture2D t_Color;
layout(set = 0, binding = 2) uniform sampler s_Color;


layout(set = 1, binding = 0) uniform MapCfg {
    int width;
    int height;
    int width_n;
    int height_n;
    int chunk_size;
};
layout(set = 1, binding = 1) uniform texture2D t_Color_checker;
layout(set = 1, binding = 2) uniform sampler s_Color_checker;



void main() {
    vec4 tex = texture(sampler2D(t_Color, s_Color), v_TexCoord);
    vec4 tex_checker = texture(sampler2D(t_Color_checker, s_Color_checker), v_TexCoord* vec2(width_n,height_n));
    o_Target =mix(tex, tex_checker, 0.1);
}
