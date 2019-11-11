#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 1) flat in int v_floor_lwall_fwall_rwall;


layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 0) uniform texture2D t_color;
layout(set = 1, binding = 1) uniform sampler s_color;


layout(set = 1, binding = 2) uniform texture2D t_pos;
layout(set = 1, binding = 3) uniform sampler s_pos;

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

int max_step = 40;

void main() {
    float water_level = 40;
    vec3 world_pos = vec3(v_TexCoord*hmap_size,water_level);
    vec4 view_pos4 = u_View* vec4(world_pos,1.0);
    vec3 view_pos  = view_pos4.xyz/ view_pos4.w;
    vec3 normal = vec3(0,0,1);
    vec3 view_normal = mat3(u_Normal)*normal;
    vec3 reflected = normalize(reflect(normalize(view_pos), normalize(view_normal)));
    vec3 current =  view_pos;
    vec3 current_dir = reflected*15;
    bool found = false;


    int j = 0;
    if(v_floor_lwall_fwall_rwall==0)
    for(int i =0; i<max_step;i++){
        j=i;
        current += current_dir;
        vec4 projected=  u_proj* vec4(current, 1);
        projected.xy /= projected.w;
        projected.xy =projected.xy*0.5 +0.5;

        if (projected.x < 0 ||projected.y<0 || projected.x>1 || projected.y>1 ){
            j=max_step;
            break;
        }
        vec4 world = texture(sampler2D(t_pos, s_pos), projected.xy);
        if ( !(world.w >-0.01) ){
            j=max_step;
            break;
        }

        vec4 view=  u_View * vec4(world.xyz,1.0);
        float depth = view.z/view.w;
        if(depth>current.z ){
            //We overshot, go back with binary search
            found = true;

            current_dir*=0.5;
            current -= current_dir;
            for(int k=0; k< 5; k++){
                current_dir*=0.5;
                vec4 projected=  u_proj* vec4(current, 1);
                projected.xy /= projected.w;
                projected.xy =projected.xy*0.5 +0.5;
                vec4 world = texture(sampler2D(t_pos, s_pos), projected.xy);
                vec4 view=  u_View * vec4(world.xyz,1.0);
                float depth = view.z/view.w;
                if(depth > current.z){
                    current -= current_dir;
                }
                else{
                    current += current_dir;
                }
            }
            break;
        }

    }
     
    vec3 sky_color= vec3(0.1,0.2,0.3);
    vec3 ref_color= sky_color;
    if(found){
        vec4 projected=  u_proj* vec4(current, 1);
        projected.xy /= projected.w;
        projected.xy =projected.xy*0.5 +0.5;

        float alpha =  pow(max(abs(0.5-projected.x),
        abs(0.5-projected.y))/0.5,4);
        ref_color = mix(
            texture(sampler2D(t_color, s_color), projected.xy).xyz,
            ref_color, alpha);
    }
    //if wall
     if(v_floor_lwall_fwall_rwall!=0){
         ref_color*=ref_color;
     }

    o_Target = vec4(ref_color,1.0);
    o_Target = vec4(vec3(float(j)/float(max_step)),1.0);
    o_Target = vec4(vec3(float(found)/1.0),1.0);
    vec3 water_color = vec3(0.3,0.5,1.0);
    o_Target = vec4(mix(water_color,ref_color ,0.8),0.9);
}
