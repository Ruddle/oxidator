#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) in vec3 color;
layout(location = 0) out vec4 o_Target;
layout(set = 0, binding = 1) uniform texture2D t_Color;
layout(set = 0, binding = 2) uniform sampler s_Color;

layout(set = 1, binding = 0) uniform MapCfg {
    float width;
    float height;
    float cam_x;
    float cam_y;
};
layout(set = 1, binding = 1) uniform texture2D t_Color_checker;
layout(set = 1, binding = 2) uniform sampler s_Color_checker;

layout(set = 1, binding = 3) uniform texture2D height_tex;
layout(set = 1, binding = 4) uniform sampler height_sampler;



void main() {
    vec4 tex = texture(sampler2D(t_Color, s_Color), v_TexCoord);
    vec4 tex_checker = texture(sampler2D(t_Color_checker, s_Color_checker),
                                v_TexCoord* vec2(width/2.0,height/2.0));

    vec2 pos_xy = v_TexCoord* vec2(width,height);
    vec3 pos = vec3(pos_xy, texture(sampler2D(height_tex, height_sampler),v_TexCoord ).r );

    vec2 a_xy=  pos_xy+vec2(1,0) ;
    vec3 a = vec3(a_xy, texture(sampler2D(height_tex, height_sampler),a_xy/ vec2(width,height) ).r );
    vec2 b_xy=  pos_xy+vec2(0,1) ;
    vec3 b = vec3(b_xy, texture(sampler2D(height_tex, height_sampler),b_xy/ vec2(width,height) ).r );
    vec3 normal = cross(a-pos, b-pos);


    vec3 c2 = vec3(   pow(max(abs(color.x-0.5) ,abs(color.y-0.5))*2.0,16)  );
//
    o_Target =  mix(vec4(normal,1.0), tex_checker, 0.1);
}
