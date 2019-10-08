#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) in vec3 color;
layout(location = 0) out vec4 o_Target;
layout(set = 0, binding = 1) uniform texture2D t_Color;
layout(set = 0, binding = 2) uniform sampler s_Color;

layout(set = 1, binding = 0) uniform MapCfg {
    int width;
    int height;
};
layout(set = 1, binding = 1) uniform texture2D t_Color_checker;
layout(set = 1, binding = 2) uniform sampler s_Color_checker;

layout(set = 1, binding = 3) uniform texture2D height_tex;
layout(set = 1, binding = 4) uniform sampler height_sampler;

void main() {
    vec4 tex = texture(sampler2D(t_Color, s_Color), v_TexCoord);
    vec4 tex_checker = texture(sampler2D(t_Color_checker, s_Color_checker),
                                v_TexCoord* vec2(width/2.0,height/2.0));



    float z = texture(sampler2D(height_tex, height_sampler), v_TexCoord).r;

    //
    o_Target =  mix(vec4(vec3(z,z,z),1.0), tex_checker, 0.1);
}
