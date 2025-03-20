

//!include common/constants.wgsl
//!include common/engine_uniforms.wgsl
//!include common/camera_uniforms.wgsl

//!include common/game_uniforms.wgsl
//!include common/obstacle_uniforms.wgsl
//!include scene_geometry.wgsl

@group(0) @binding(0)
var<storage, read> g_engine: EngineUniforms;
@group(0) @binding(1)
var<storage, read> g_camera: CameraUniforms;


@group(1) @binding(0)
var<storage, read> g_game: GameUniforms;
@group(1) @binding(1)
var<storage, read> g_obstacle_globals: ObstacleGlobal;
@group(1) @binding(2)
var<storage, read> g_obstacles: array<Obstacle, 24>;

@group(2) @binding(0)
var t_noise2d: texture_2d<f32>;
@group(2) @binding(1)
var s_noise2d: sampler;


// Vertex shader

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    // for 0, 1, 2 output (-1, -1), (1, -1), (-1, 1)
    // for 3, 4, 5 output (1, -1), (1, 1), (-1, 1)
    var x = select(-1.0, 1.0, (in_vertex_index == 1u) || (in_vertex_index == 3u) || (in_vertex_index == 4u));
    var y = select(-1.0, 1.0, in_vertex_index == 2u || in_vertex_index == 4u || in_vertex_index == 5u);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}


fn normal(p: vec3f) -> vec3f {
    var e = vec2f(0.0001, 0.0);
    return normalize(vec3f(
        map(p + e.xyy) - map(p - e.xyy),
        map(p + e.yxy) - map(p - e.yxy),
        map(p + e.yyx) - map(p - e.yyx)
    ));
}

fn shade_directional_light_intensity(intensity: f32, light_dir: vec3f, view_dir: vec3f, normal: vec3f) -> f32 {
    // light_dir: The direction from the point to the light source
    // normal: The normal vector at the point being shaded

    var diffuse = max(dot(normal, light_dir), 0.0) * intensity;
    return diffuse;
}

fn modulo(a: f32, b: f32) -> f32 {
    return a - b * floor(a / b);
}


struct SimpleRayHit {
    distance: f32,
    hit: bool,
    final_position: vec3f,
}

fn simple_ray_trace(pos: vec3f, dir: vec3f, max_steps: i32) -> SimpleRayHit {
    var result = SimpleRayHit();
    var p = pos;
    var d = map_dir(p, dir);
    if (abs(d) < 0.01) {
        d = 0.1;
    }
    var inside = d < 0.0;
    var t = 0.0;
    p += abs(d) * dir;
    for (var i = 0; i < max_steps; i = i + 1) {
        var d = map_dir(p, dir);
        if (!inside && d < 0.01) { 
            result.hit = true;
            break; 
        }
        inside = d < 0.0;
        p += abs(d) * dir;
        t = t + abs(d);
    }
    result.distance = t;
    result.final_position = p;
    return result;
}


fn calcAO(p: vec3f, n: vec3f) -> f32 {
    var sca = 1.5;
    var dist = 2.0;
    var occ = 0.0;
    for (var i = 0; i < 5; i = i + 1) {
        var hr = f32(i + 1) * 0.15 / dist;
        var d = map(p + n * hr);
        occ += (hr - d) * sca;
        sca *= 0.7;
        if (sca > 1e5) {
            break;
        }
    }
    return clamp(1.0 - occ, 0.0, 1.0);
}


struct Material {
    color: vec3f,
    roughness: f32,
    // metallic: f32,
    reflectivity: f32,
    // emissive: f32,
    _padding: f32,
}

fn material_from_id(id: i32) -> Material {
    var material = Material();
    // Ground
    if (id == 0) {
        material.color = vec3f(0.1, 0.02, 0.02);
        material.reflectivity = 0.9;
        // material.roughness = 0.1;
    }

    // Player
    if (id == 1) {
        material.color = vec3f(0.8, 0.5, 0.3);
        material.reflectivity = 0.8;
        // material.roughness = 0.1;
    }

    // Obstacle
    if (id == 2) {
        material.color = vec3f(0.7, 0.2, 0.2);
        material.reflectivity = 0.0;
        // material.roughness = 0.1;
    }
    return material;
}

fn srgb_to_rgb(srgb_color: vec3f) -> vec3f {
    return pow((srgb_color + vec3f(0.055)) / 1.055, vec3f(2.4));
}

fn rgb_to_srgb(rgb_color: vec3f) -> vec3f {
    return pow(rgb_color, vec3f(0.416666)) * 1.055 - 0.055;
} 

// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // setup camera
    let aspect = g_engine.resolution_x / g_engine.resolution_y;
    let uv = -(in.clip_position.xy / g_engine.resolution_y - vec2f(aspect * 0.5, 0.5));

    let camera_origin = g_camera.position.xyz;
    let camera_direction = g_camera.direction.xyz;
    var ray_direction = normalize(camera_direction*0.9 + uv.x * g_camera.u_dir.xyz + uv.y * g_camera.v_dir.xyz);

    // raymarch
    var transmittance = 1.0;

    let max_steps = 80;
    let max_bounces = 3;
    let max_distance = 100.0;
    let epsilon = 0.01;
    var t = 0.0;
    var p = camera_origin;
    
    // Vary scene scale
    // let t_frac = (2.0 * g_engine.global_time) - floor(2.0 * g_engine.global_time);
    // let t_mod = floor(g_engine.global_time * 2.0) % 4.0;
    // let foo = (1.0 - t_frac);
    // var factor = 0.0;
    // if t_mod == 0.0 {
    //     factor = foo * 0.07;
    // } else if t_mod == 1.0 {
    //     factor = foo * 0.03;
    // } else if t_mod == 2.0 {
    //     factor = foo * 0.04;
    // } else if t_mod == 3.0 {
    //     factor = foo * 0.03;
    // }
    // p *= (1.0 - factor);
    
    let background_color = vec3f(0.2);
    var color = background_color;

    var bounce = 0;
    var step = 0;
    var hit_infty = false;

    var min_player_distance = 1e20;

    for (bounce = 0; bounce < max_bounces; bounce = bounce + 1) {
        var hit = false;
        for (; step < max_steps; step = step + 1) {
            // note step is not reset after a bounce
            var d = map_dir(p, ray_direction);
            transmittance *= exp(-0.01 * d);
            if (transmittance < 0.01) {
                hit_infty = true;
                break;
            }

            if (d < epsilon * (1.0 + t * 0.1)) {
                hit = true;
                break;
            }
            t = t + abs(d);
            if (t > max_distance) {
                hit_infty = true;
                break;
            }


            min_player_distance = min(min_player_distance, player_distance(p));

            var rnd = modulo(t * 1000.0, 1.0); 
            p += abs(d) * (0.99) * ray_direction;
        }

        if (hit) {
            var n = normal(p);

            var material_id = map_color(p);
            var material = material_from_id(material_id);
            var current_hit_color = material.color;

            // Lighting
            var light = normalize(vec3f(-0.4, 1.0, 0.4));
            var view = -ray_direction;
            var in_shade = simple_ray_trace(p, light, 20).hit;
            var diffuse_intensity = select(shade_directional_light_intensity(0.4, light, view, n), 0.0, in_shade);
            var ambient = 0.1;
            var ambient_occlusion = calcAO(p, n);
            current_hit_color = current_hit_color * (diffuse_intensity + ambient * ambient_occlusion);

            // Depth Fog for low y values
            var depth = clamp(0.0, 1.0, exp(p.y * 0.5));
            let depth_fog_color = vec3f(0.2, 0.1, 0.0);
            current_hit_color = mix(current_hit_color, depth_fog_color, 1.0 - depth);

            // Transmittance fog
            // transmittance = clamp(0.0, 1.0, transmittance);
            color = mix(color, current_hit_color, transmittance);
            
            if (material.reflectivity > 0.0) {
                ray_direction = reflect(ray_direction, n);
                p += 0.1 * n;
                transmittance *= material.reflectivity;
            } else {
                break;
            }
        } else {
            // color = mix(color, background_color, transmittance);
            // color = vec3f(0.0);
            // transmittance = 0.0;
            break;
        }
    }
    // if (hit_infty) {
    //     color = mix(color, background_color, 1.0 - transmittance);
    // }

    color += 0.1 * vec3f(1.0, 0.8, 0.6) * clamp(0.2 / (min_player_distance + 0.3), 0.0, 0.3);

    //color = vec3(f32(i) / 10);
    return vec4f(color, 1.0);
}