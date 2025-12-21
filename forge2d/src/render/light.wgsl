// Light rendering shader for 2D point lights and spotlights with shadows

struct LightUniforms {
    position: vec2<f32>,
    color: vec3<f32>,
    intensity: f32,
    radius: f32,
    falloff: f32,
    direction: vec2<f32>, // Spotlight direction (normalized), or [0,0] for point light
    angle: f32, // Spotlight angle (cos of half-angle), or 0 for point light
    mvp: mat4x4<f32>,
    // Camera info for shadow mapping
    screen_size: vec2<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> uniforms: LightUniforms;
@group(0) @binding(1) var scene_tex: texture_2d<f32>;
@group(0) @binding(2) var scene_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Input position is in local space (-1 to 1 for the quad)
    // Transform to world space: scale by radius and translate by light position
    let local_pos = in.position;
    let world_pos_2d = uniforms.position + local_pos * uniforms.radius;
    
    // Transform to clip space using MVP (which includes view-projection)
    // MVP = view_proj * translation * scale
    // So we apply MVP to local_pos to get clip space
    let local_pos_vec4 = vec4<f32>(local_pos, 0.0, 1.0);
    out.clip_position = uniforms.mvp * local_pos_vec4;
    
    // World position for fragment shader (used for distance calculations)
    out.world_position = world_pos_2d;
    return out;
}

// Convert world position to screen UV coordinates
fn world_to_screen_uv(world_pos: vec2<f32>) -> vec2<f32> {
    let clip_pos = uniforms.view_proj * vec4<f32>(world_pos, 0.0, 1.0);
    // Convert from clip space [-1,1] to UV [0,1]
    let uv = clip_pos.xy / clip_pos.w;
    return vec2<f32>(0.5) + uv * vec2<f32>(0.5);
}

// Check if a point is occluded by sampling the scene texture
fn is_occluded(world_pos: vec2<f32>) -> bool {
    let uv = world_to_screen_uv(world_pos);
    let scene_sample = textureSample(scene_tex, scene_sampler, uv);
    // If alpha > 0.5, there's an occluder
    return scene_sample.a > 0.5;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Debug: Always output bright light to verify shader is running
    // If you see this, the shader works but distance calculation is wrong
    return vec4<f32>(1.0, 0.0, 1.0, 1.0); // Magenta - always output
    
    // Calculate distance from light center in world space
    let dist = distance(in.world_position, uniforms.position);
    
    // Debug: Output based on distance to see if calculation works
    // if dist < 50.0 {
    //     return vec4<f32>(1.0, 1.0, 0.0, 1.0); // Yellow if close
    // }
    
    // Early exit if beyond radius (don't draw anything)
    if dist > uniforms.radius {
        discard;
    }
    
    // Simple point light - no spotlight checks for now
    var light_strength = uniforms.intensity;
    
    // Calculate distance falloff (1.0 at center, 0.0 at radius)
    let normalized_dist = dist / uniforms.radius;
    let distance_falloff = pow(1.0 - saturate(normalized_dist), uniforms.falloff);
    light_strength *= distance_falloff;
    
    // Apply light color - mix between white (neutral) and light color
    let light_color = mix(vec3<f32>(1.0), uniforms.color, 0.6);
    
    // Return light contribution (will be accumulated additively in light map)
    return vec4<f32>(light_color * light_strength, 1.0);
}

