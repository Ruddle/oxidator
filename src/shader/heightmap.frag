#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) in vec3 color;
layout(location = 0) out vec4 o_Target;
layout(location = 1) out vec4 position_att;
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



const vec3 ambientColor = vec3(0.05);
const vec3 diffuseColor = vec3(1.0, 1.0, 1.0);
const vec3 specColor = vec3(0.8);

void main() {
    vec4 tex = texture(sampler2D(t_Color, s_Color), v_TexCoord);
    vec4 tex_checker = texture(sampler2D(t_Color_checker, s_Color_checker),
                                v_TexCoord* vec2(width/2.0,height/2.0));

    vec2 pos_xy = v_TexCoord* vec2(width,height);

    vec2 tex_coord_floor =vec2(floor(pos_xy.x),floor(pos_xy.y)) / vec2(width,height);
    vec3 pos = vec3(pos_xy, texture(sampler2D(height_tex, height_sampler),v_TexCoord ).r );

    vec2 a_xy=  pos_xy+vec2(1,0) ;
    vec3 a = vec3(a_xy, texture(sampler2D(height_tex, height_sampler),a_xy/ vec2(width,height) ).r );
    vec2 b_xy=  pos_xy+vec2(0,1) ;
    vec3 b = vec3(b_xy, texture(sampler2D(height_tex, height_sampler),b_xy/ vec2(width,height) ).r );
    vec3 normal = normalize(cross(a-pos, b-pos));


    //blinn phong
    vec3 lightPos = vec3(0.0*width/2.0,0.0*height/2.0,2000.0);

    vec3 vertPos = pos;
    vec3 lightDir = normalize(lightPos - vertPos);

    float lambertian = max(dot(lightDir,normal), 0.0);
    float specular = 0.0;

    if(lambertian > 0.0) {
        vec3 viewDir = normalize(-vertPos);
        vec3 halfDir = normalize(lightDir + viewDir);
        float specAngle = max(dot(halfDir, normal), 0.0);
        specular = pow(specAngle, 16.0);

    }

    float m = 0.1;
    if(gl_FragCoord.z > 0.37){
        m=0.1;//0.0
    }

    vec3 phong = vec3(ambientColor +
    lambertian* mix(tex.xyz,tex_checker.xyz,m) +
    specular*specColor);


    position_att = vec4(pos, 1.0);
    o_Target =   vec4(phong,1.0);
}
