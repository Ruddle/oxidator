#version 450

layout(location = 0) in vec2 v_TexCoord;
layout(location = 0) out vec4 o_Target;

layout(set = 1, binding = 0) uniform texture2D t_color;
layout(set = 1, binding = 1) uniform sampler s_color;

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


float rgb2luma(vec3 rgb){
    return sqrt(dot(rgb, vec3(0.299, 0.587, 0.114)));
}

float luma(vec2 offset){
    return rgb2luma(texture(sampler2D(t_color, s_color), v_TexCoord + offset*inv_resolution).rgb);
}

void main() {


    vec3 color =  texture(sampler2D(t_color, s_color), v_TexCoord).rgb;
    float l= luma(vec2(0));

    float l_n = luma(vec2(0,1));
    float l_s = luma(vec2(0,-1));
    float l_e = luma(vec2(1,0));
    float l_w = luma(vec2(-1,0));

    float lumaMin = min(l,min(min(l_s,l_n),min(l_e,l_w)));
    float lumaMax = max(l,max(max(l_s,l_n),max(l_e,l_w)));

    float lumaRange = lumaMax - lumaMin;

    if ( lumaRange> min(0.0312 , lumaMax*0.125)){
        float l_ne = luma(vec2(1,1));
        float l_se = luma(vec2(1,-1));
        float l_nw = luma(vec2(-1,1));
        float l_sw = luma(vec2(-1,-1));

        float l_ns = l_n + l_s;
        float l_we = l_e + l_w;


        float l_nc = l_nw + l_ne;
        float l_sc = l_se + l_sw;
        float l_ec = l_se + l_ne;
        float l_wc = l_nw + l_sw;

        float edgeHorizontal =  abs(-2.0 * l_w + l_wc)  +
         abs(-2.0 * l + l_ns ) * 2.0     +
        abs(-2.0 * l_e + l_ec);

        float edgeVertical =    abs(-2.0 * l_n + l_nc)  +
         abs(-2.0 * l + l_we) * 2.0   + 
         abs(-2.0 * l_s + l_sc);

        bool isHorizontal = (edgeHorizontal >= edgeVertical);
        // color = vec3(0.5+ (edgeVertical- edgeHorizontal)/2.0);



        // Select the two neighboring texels lumas in the opposite direction to the local edge.
        float luma1 = isHorizontal ? l_s : l_w;
        float luma2 = isHorizontal ? l_n : l_e;
        // Compute gradients in this direction.
        float gradient1 = luma1 - l;
        float gradient2 = luma2 - l;

        // Which direction is the steepest ?
        bool is1Steepest = abs(gradient1) >= abs(gradient2);

        // Gradient in the corresponding direction, normalized.
        float gradientScaled = 0.25*max(abs(gradient1),abs(gradient2));
        // color = vec3(gradientScaled);



        // Choose the step size (one pixel) according to the edge direction.
        float stepLength = isHorizontal ? inv_resolution.y : inv_resolution.x;

        // Average luma in the correct direction.
        float lumaLocalAverage = 0.0;

        if(is1Steepest){
            // Switch the direction
            stepLength = - stepLength;
            lumaLocalAverage = 0.5*(luma1 + l);
        } else {
            lumaLocalAverage = 0.5*(luma2 + l);
        }

        // Shift UV in the correct direction by half a pixel.
        vec2 currentUv = v_TexCoord;
        if(isHorizontal){
            currentUv.y += stepLength * 0.5;
        } else {
            currentUv.x += stepLength * 0.5;
        }

        ///First iteration exploration

        // Compute offset (for each iteration step) in the right direction.
        vec2 offset = isHorizontal ? vec2(inv_resolution.x,0.0) : vec2(0.0,inv_resolution.y);
        // Compute UVs to explore on each side of the edge, orthogonally. The QUALITY allows us to step faster.
        vec2 uv1 = currentUv - offset;
        vec2 uv2 = currentUv + offset;

        // Read the lumas at both current extremities of the exploration segment, and compute the delta wrt to the local average luma.
        float lumaEnd1 = rgb2luma(texture(sampler2D(t_color, s_color), uv1).rgb);
        float lumaEnd2 = rgb2luma(texture(sampler2D(t_color, s_color), uv2).rgb);
        lumaEnd1 -= lumaLocalAverage;
        lumaEnd2 -= lumaLocalAverage;

        // If the luma deltas at the current extremities are larger than the local gradient, we have reached the side of the edge.
        bool reached1 = abs(lumaEnd1) >= gradientScaled;
        bool reached2 = abs(lumaEnd2) >= gradientScaled;
        bool reachedBoth = reached1 && reached2;

        // If the side is not reached, we continue to explore in this direction.
        if(!reached1){
            uv1 -= offset;
        }
        if(!reached2){
            uv2 += offset;
        }  


        // If both sides have not been reached, continue to explore.
        if(!reachedBoth){

            float QUALITY[12]= float[12](1.0,1.0,1.0,1.0,1.0,1.5, 2.0, 2.0, 2.0, 2.0, 4.0, 8.0);
            for(int i = 2; i < 12; i++){
                // If needed, read luma in 1st direction, compute delta.
                if(!reached1){
                    lumaEnd1 = rgb2luma(texture(sampler2D(t_color, s_color), uv1).rgb);
                    lumaEnd1 = lumaEnd1 - lumaLocalAverage;
                }
                // If needed, read luma in opposite direction, compute delta.
                if(!reached2){
                    lumaEnd2 = rgb2luma(texture(sampler2D(t_color, s_color), uv2).rgb);
                    lumaEnd2 = lumaEnd2 - lumaLocalAverage;
                }
                // If the luma deltas at the current extremities is larger than the local gradient, we have reached the side of the edge.
                reached1 = abs(lumaEnd1) >= gradientScaled;
                reached2 = abs(lumaEnd2) >= gradientScaled;
                reachedBoth = reached1 && reached2;

                // If the side is not reached, we continue to explore in this direction, with a variable quality.
                if(!reached1){
                    uv1 -= offset * QUALITY[i];
                }
                if(!reached2){
                    uv2 += offset * QUALITY[i];
                }

             
                // If both sides have been reached, stop the exploration.
                if(reachedBoth){ break;}
            }
        }


        // Compute the distances to each extremity of the edge.
        float distance1 = isHorizontal ? (v_TexCoord.x - uv1.x) : (v_TexCoord.y - uv1.y);
        float distance2 = isHorizontal ? (uv2.x - v_TexCoord.x) : (uv2.y - v_TexCoord.y);

        // In which direction is the extremity of the edge closer ?
        bool isDirection1 = distance1 < distance2;
        float distanceFinal = min(distance1, distance2);

        // Length of the edge.
        float edgeThickness = (distance1 + distance2);

        // UV offset: read in the direction of the closest side of the edge.
        float pixelOffset = - distanceFinal / edgeThickness + 0.5;

        // Is the luma at center smaller than the local average ?
        bool isLumaCenterSmaller = l < lumaLocalAverage;

        // If the luma at center is smaller than at its neighbour, the delta luma at each end should be positive (same variation).
        // (in the direction of the closer side of the edge.)
        bool correctVariation = ((isDirection1 ? lumaEnd1 : lumaEnd2) < 0.0) != isLumaCenterSmaller;

        // If the luma variation is incorrect, do not offset.
        float finalOffset = correctVariation ? pixelOffset : 0.0;

        // Sub-pixel shifting
        // Full weighted average of the luma over the 3x3 neighborhood.
        float lumaAverage = (1.0/12.0) * (2.0 * (l_ns + l_we) + l_ec + l_wc);
        // Ratio of the delta between the global average and the center luma, over the luma range in the 3x3 neighborhood.
        float subPixelOffset1 = clamp(abs(lumaAverage - l)/lumaRange,0.0,1.0);
        float subPixelOffset2 = (-2.0 * subPixelOffset1 + 3.0) * subPixelOffset1 * subPixelOffset1;
        // Compute a sub-pixel offset based on this delta.
        float subPixelOffsetFinal = subPixelOffset2 * subPixelOffset2 * 0.75;

        // Pick the biggest of the two offsets.
        finalOffset = max(finalOffset,subPixelOffsetFinal);
        // color = vec3(finalOffset);

        // Compute the final UV coordinates.
        vec2 finalUv = v_TexCoord;
        if(isHorizontal){
            finalUv.y += finalOffset * stepLength;
        } else {
            finalUv.x += finalOffset * stepLength;
        }

        // Read the color at the new UV coordinates, and use it.
        color = texture(sampler2D(t_color, s_color), finalUv).rgb;
    }
 

    o_Target = vec4(color,1.0);
}
