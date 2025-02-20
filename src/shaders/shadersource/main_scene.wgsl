

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

fn shade(color: vec3f, position: vec3f, light_dir: vec3f, view_dir: vec3f, normal: vec3f) -> vec3f {
    // position: The position of the point being shaded
    // light_dir: The direction from the point to the light source
    // view_dir: The direction from the point to the viewer (camera)
    // normal: The normal vector at the point being shaded

    var ambient = 0.1;
    var diffuse = max(dot(normal, light_dir), 0.0) * 0.7;
    return color * (ambient + diffuse);
}

fn modulo(a: f32, b: f32) -> f32 {
    return a - b * floor(a / b);
}


fn ray_trace_simple(pos: vec3f, dir: vec3f, max_steps: i32) -> f32 {
    var t = 0.0;
    for (var i = 0; i < max_steps; i = i + 1) {
        var d = map_dir(pos + t * dir, dir);
        t = t + d;
    }
    return t;
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



// Fragment shader
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // setup camera
    var aspect = g_engine.resolution_x / g_engine.resolution_y;
    var uv = -(in.clip_position.xy / g_engine.resolution_y - vec2f(aspect * 0.5, 0.5));

    var camera_origin = g_camera.position.xyz;
    var camera_direction = g_camera.direction.xyz;
    var ray_direction = normalize(camera_direction*0.9 + uv.x * g_camera.u_dir.xyz + uv.y * g_camera.v_dir.xyz);

    // raymarch
    var color = vec3f(0.2);
    var transmittance = 1.0;
    var hit = false;

    var max_steps = 80;
    var max_distance = 100.0;
    var epsilon = 0.01;
    var t = 0.0;
    var p = camera_origin;
    var i = 0;
    for (i = 0; i < max_steps; i = i + 1) {
        var d = map_dir(p, ray_direction);
        transmittance *= exp(-0.01 * d);

        if (d < epsilon * (1.0 + t * 0.1)) {
            color = vec3f(0.5, 0.4, 0.6) * transmittance;
            hit = true;
            break;
        }
        t = t + abs(d);
        if (t > max_distance) {
            break;
        }
        var rnd = modulo(t * 1000.0, 1.0); 
        p += abs(d) * (0.99) * ray_direction;
        
        //p += abs(d) * ray_direction;
    }

    if (hit) {
        var n = normal(p);
        var light = normalize(vec3f(-1.0, 0.2, 0.8));
        var view = -ray_direction;
        color = shade(color, p, light, view, n);
        var ambient_occlusion = calcAO(p, n);
        color *= ambient_occlusion;
    } else {
        transmittance = 0.0;
    }

    var depth = clamp(0.0, 1.0, exp(p.y * 0.5));
    transmittance = clamp(0.0, 1.0, transmittance);
    color = mix(color, vec3f(0.1, 0.2, 0.1), 1.0 - transmittance);
    color = mix(color, srgb_to_rgb_color(vec3f(0.2, 0.1, 0.0)), 1.0 - depth);
    //color = vec3(f32(i) / 10);
    return vec4f(color, 1.0);
}