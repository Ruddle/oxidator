#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) in vec3 color;
layout(location = 2) in float min_lod;
layout(location = 3) in float max_mip;
layout(location = 0) out vec4 o_Target;
layout(location = 1) out vec4 o_position_att;
layout(location = 2) out vec2 o_normal;

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
const vec3 specColor = vec3(0.2);

float linlerp(float v, float min, float max){
    return (v-min)/(max-min);
}

vec3 normal_at(vec2 uv,float lod){
    float r=  textureLod(sampler2D(height_tex, height_sampler),uv +vec2(1,0)/ vec2(width,height),lod ).r;
    float l=  textureLod(sampler2D(height_tex, height_sampler),uv+ vec2(-1,0)/ vec2(width,height),lod ).r;
    float u=  textureLod(sampler2D(height_tex, height_sampler),uv+vec2(0,1)/ vec2(width,height),lod ).r;
    float d=  textureLod(sampler2D(height_tex, height_sampler),uv +vec2(0,-1)/ vec2(width,height),lod ).r;
    return normalize(vec3(-(r-l), (d-u), 2));
}

float slope_of(vec3 normal){
    return 1-asin(normal.z)/(3.141592/2.0);
}

void main() {
    vec4 tex = texture(sampler2D(t_Color, s_Color), v_TexCoord);
    vec4 tex_checker = texture(sampler2D(t_Color_checker, s_Color_checker),
    (v_TexCoord) * vec2(width/2.0,height/2.0) + vec2(0.5/2.0));

    vec2 pos_xy = v_TexCoord* vec2(width,height);

    vec2 tex_coord_floor =vec2(floor(pos_xy.x),floor(pos_xy.y)) / vec2(width,height);

    float lod = max(textureQueryLod(sampler2D(height_tex, height_sampler),v_TexCoord).x,min_lod);

    vec3 pos = vec3(pos_xy, textureLod(sampler2D(height_tex, height_sampler),v_TexCoord,lod ).r );

    vec3 normal = normal_at(v_TexCoord,lod);
    float slope = slope_of(normal);

    vec3 diffuse=  mix(vec3(0.5,0.4,0.3),tex_checker.xyz,0.041);

    float ground_end= 1/90.0;
    float grass_start= 2/90.0;

    vec3 grass_color = vec3(0.45,0.7,0.2);
    vec3 sand_color = vec3(1.0,0.8,0.7);

    grass_color = mix(grass_color,sand_color, min(1,max(46.0- pos.z,0)*2 )); 
    
    if  (slope > ground_end){
        diffuse = mix(diffuse,grass_color, linlerp(slope,ground_end,grass_start) );
    }
    if (slope > grass_start){
        diffuse= grass_color;
    }
    if (slope > 45/90.0){
        diffuse = vec3(0.5);
    }

    //blinn phong
    vec3 lightPos = vec3(-10000,1000,12000);

    vec3 vertPos = pos;
    vec3 lightDir = normalize(lightPos - vertPos);

    float lambertian = max(dot(lightDir,normal), 0.0);
    float specular = 0.0;

    if(lambertian > 0.0) {
        mat3 rot = mat3(u_View);
        vec3 camera_pos = -u_View[3].xyz*rot;
        vec3 viewDir = normalize( camera_pos - vertPos);
        vec3 halfDir = normalize(lightDir + viewDir);
        float specAngle = max(dot(halfDir, normal), 0.0);
        specular = 1.0*pow(specAngle, 32.0);
    }
    
    vec3 phong = vec3(ambientColor +
    lambertian* diffuse +
    specular*specColor);


    // phong =mix(color, phong,0.1);
    if(
        pos.y <=0.5 || 
        pos.x <=0.5 || 
        pos.x >= width-0.5  ||
        pos.y >= height  
    ){
        phong= vec3(0.1);
    }

    o_normal = normal.xy;
    o_position_att = vec4(pos, 0.0);
    o_Target =   vec4(phong,1.0);
}
