#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) in vec3 world_pos;
layout(location = 2) in float v_selected;
layout(location = 3) in float v_team;
layout(location = 4) in vec3 v_world_normal;

layout(location = 0) out vec4 o_Target;
layout(location = 1) out vec4 position_att;
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

void main() {
    vec4 tex = texture(sampler2D(t_Color, s_Color), v_TexCoord);

    position_att = vec4(world_pos, v_selected );

    vec3 color = vec3(1);
    if(v_team < -0.1){
        color = vec3(1);
    }
    else if( v_team == 0.0){
        color = vec3(0,0.3,1);
    }else if(v_team < 1.1){
        color = vec3(1,0.0,0);
    }


    vec3 diffuse= mix(tex.xyz, color,0.5);;
       //blinn phong
    const vec3 ambientColor = vec3(0.05);
    const vec3 diffuseColor = vec3(1.0, 1.0, 1.0);
    const vec3 specColor = vec3(0.2);
    vec3 lightPos = vec3(-10000,1000,12000);

    vec3 vertPos = world_pos;
    vec3 lightDir = normalize(lightPos - vertPos);
    vec3 normal = v_world_normal;

    float lambertian = 0.35+max(dot(lightDir,normal), 0.0);
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

    o_Target =    vec4(phong, 1.0);
}
