


var<private> g_obstacles: array<Obstacle, 24>;
const DEG_TO_RAD = 0.01745329252;


struct EngineUniforms {
    resolution_x: f32,
    resolution_y: f32,
    window_focused: i32,
    time: f32,

    frame: i32,
    global_time: f32,
    mouse_x: f32,
    mouse_y: f32,
};

@group(0) @binding(0)
var<uniform> engine_uniforms: EngineUniforms;





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
    var x = select(-1.0, 1.0, in_vertex_index == 1 || in_vertex_index == 3 || in_vertex_index == 4);
    var y = select(-1.0, 1.0, in_vertex_index == 2 || in_vertex_index == 4 || in_vertex_index == 5);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}


const UP = vec3f(0.0, 1.0, 0.0);


// Shared

fn srgb_to_rgb_color(srgb_color: vec3f) -> vec3f {
    // rgb_color = ((srgb_color / 255 + 0.055) / 1.055) ^ 2.4
    return pow(srgb_color + 0.055 / 1.055, vec3f(2.4));
}

// https://iquilezles.org/articles/distfunctions/
fn boxSdf(p: vec3f, b: vec3f) -> f32 {
    var q = abs(p) - b;
    return min(max(q.x, max(q.y, q.z)), 0.0) + length(max(q, vec3f(0.0)));
}

// https://iquilezles.org/articles/distfunctions/
fn sphereSdf(p: vec3f, r: f32) -> f32 {
    return length(p) - r;
}

const sqrt3half = 0.8660254;
const sqrt3inv = 0.57735;


// float sdHexPrism( vec3 p, vec2 h )
// {
//   const vec3 k = vec3(-0.8660254, 0.5, 0.57735);
//   p = abs(p);
//   p.xy -= 2.0*min(dot(k.xy, p.xy), 0.0)*k.xy;
//   vec2 d = vec2(
//        length(p.xy-vec2(clamp(p.x,-k.z*h.x,k.z*h.x), h.x))*sign(p.y-h.x),
//        p.z-h.y );
//   return min(max(d.x,d.y),0.0) + length(max(d,0.0));
// }

// https://iquilezles.org/articles/distfunctions/
fn hexPrismSdf(position: vec3f, h: vec2f) -> f32 {
    var k = vec3f(-sqrt3half, 0.5, sqrt3inv);
    var p = abs(position);
    var f = 2.0 * min(dot(k.xy, p.xz), 0.0);
    p -= vec3f(f * k.x, 0.0, f * k.y);
    var d = vec2f(
        length(p.xz - vec2f(clamp(p.x, -k.z * h.x, k.z * h.x), h.x)) * sign(p.z - h.x),
        p.y - h.y
    );
    return min(max(d.x, d.y), 0.0) + length(max(d, vec2f(0.0)));
}

// untested!

fn hexPrismInfSdf(position: vec3f) -> f32 {
    var k = vec3f(sqrt3half, 0.5, sqrt3inv);
    var p = abs(position);
    p -= vec3f(2.0 * min(dot(k.xy, p.xy), 0.0) * k.xy, 1.0);
    var d = length(p.xy - vec2f(clamp(p.x, -k.z * 1.0, k.z * 1.0), 1.0)) * sign(p.y - 1.0);
    return d;
}

// length along a ray to the intersection point
fn plane_ray_dist(ray_origin: vec3f, ray_direction: vec3f, plane_point: vec3f, plane_normal: vec3f) -> f32 {
    let numerator = dot(plane_point - ray_origin, plane_normal);
    let denominator = dot(ray_direction, plane_normal);
    return -numerator / denominator * sign(denominator);
}


fn hash12(p: vec2f) -> f32 {
    var p3 = fract(vec3f(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}


// vec3 rot(vec3 p, vec3 ax, float t) {
//     return mix(dot(ax, p) * ax, p, cos(t)) + cross(ax, p) * sin(t);
// }

// float smin( float a, float b, float k )
// {
//     k *= 4.0;
//     float h = max( k-abs(a-b), 0.0 )/k;
//     return min(a,b) - h*h*k*(1.0/4.0);
// }

// float smin(float a, float b) {
//     return smin(a, b, 0.05);
// }


fn rot(p: vec3f, ax: vec3f, t: f32) -> vec3f {
    return mix(dot(ax, p) * ax, p, cos(t)) + cross(ax, p) * sin(t);
}

fn smink(a: f32, b: f32, k: f32) -> f32 {
    var k4 = k * 4.0;
    var h = max(k4 - abs(a - b), 0.0) / k4;
    return min(a, b) - h * h * k4 * (1.0 / 4.0);
}

fn smin(a: f32, b: f32) -> f32 {
    return smink(a, b, 0.05);
}

fn smax(a: f32, b: f32) -> f32 {
    return -smin(-a, -b);
}


fn extrudeSdf(p: vec3f, dxy: f32, h: f32) -> f32 {
    var w = vec2f(dxy, abs(p.z) - h);
    return min(max(w.x, w.y), 0.0) + length(max(w, vec2f(0.0)));
}

fn hollowSphereSdf(p: vec3f, radius: f32, thickness: f32) -> f32 {
    return abs(length(p) - radius) - thickness;
}


fn ellipsoidSdf(p: vec3f, r: vec3f) -> f32 {
    var k0 = length(p / r);
    var k1 = length(p / (r * r));
    return k0 * (k0 - 1.0) / k1;
}

fn obstacleShapeSdf(p: vec3f, q: vec3f) -> f32 {
    var radius = length(q.xz);
    var thickness = 0.25;
    return max(hollowSphereSdf(p, radius + thickness * 0.5, thickness), sphereSdf(p - vec3f(q.x, 0.0, q.z), 1.5));
}

fn sphereExactIntersection(p: vec3f, d: vec3f, r: f32) -> f32 {
    var b = dot(p, d);
    var c = dot(p, p) - r * r;
    var h = b * b - c;
    if (h < 0.0) {
        return -1.0;
    }
    return -b - sqrt(h);
}

struct Obstacle {
    lane: i32,
    distance: f32,
    angle:  f32,
    center: vec3f,
    radius: f32,
    thickness: f32,
    ctr_a: vec3f,
    ellipsoid_radii: vec3f,
    r1: f32,
}

fn create_obstacle(lane: i32, distance: f32, i: i32) {
    var o = g_obstacles[i];
    o.lane = lane;
    o.distance = distance;
    o.angle = f32(o.lane) * 60.0 * DEG_TO_RAD;
    var ctr = vec3f(sin(o.angle), 0.0, cos(o.angle));
    var center1 = ctr * (1.5 + o.distance);
    center1.y = 2.0;
    var center2 = ctr * 1.5;
    center2 = vec3f(center2.x, 1.5 + o.distance, center2.z);
    center2 *= exp(o.distance * 0.1 + 0.5);
    o.center = mix(center2, center1, smoothstep(-1.0, 1.0, o.distance));
    o.radius = length(o.center.xz);
    o.thickness = 0.2;
    o.ctr_a = o.center - vec3f(0.0, o.center.y, 0.0) * 2.0;
    o.ellipsoid_radii = vec3f(o.radius * 0.483, 0.03 * o.distance + 0.5, o.radius * 0.483);
    o.r1 = 2.0*o.radius + o.thickness * 0.5;
    g_obstacles[i] = o;
}

// lane should be between 0 and 5
fn obstacleSdf(p: vec3f, i: i32) -> f32 {
    return sphereSdf(p - g_obstacles[i].center, 1.5);
    /*
    var p_lane = floor(atan2(p.x, p.z) / 6.2831853 * 6.0);
    if (abs((p_lane - f32(g_obstacles[i].lane)) % 6) > 1.0) {
        return 1000.0;
    }
    var o = g_obstacles[i];
    var d = hollowSphereSdf(p + o.ctr_a, o.r1, o.thickness);
    d = smax(d, ellipsoidSdf(p - o.center, o.ellipsoid_radii));
    return d;
    */
}

fn obstacleDist(p: vec3f, rd: vec3f, i: i32) -> f32 {
    var o = g_obstacles[i];
    var d = sphereExactIntersection(p - o.center, rd, 1.5);
    if (d < 0.0) {
        return 1000.0;
    }
    return d;
}

fn ground_distance(p: vec3f) -> f32 {
    var d = 1e20;
    var l = sqrt3inv * 2.0;
    var l2 = 0.5;
    var p_transformed = vec2f(p.x, (p.z - l2 * p.x) / l);
    var center_transformed = vec2f(floor(p_transformed.x), floor(p_transformed.y));
    for (var x = -1.0; x <= 2.0; x = x + 1.0) {
        for (var y = -1.0; y <= 2.0; y = y + 1.0) {
            var center_xy = vec2f(center_transformed.x + x, center_transformed.y + y);
            var center_original_space = vec3f(center_xy.x, hash12(center_xy), center_xy.y * l + l2 * center_xy.x);
            var l3 = length(center_original_space.xz);
            center_original_space -= UP *  40.0 / (l3 * l3 * l3);

            var p_centered = (p - center_original_space + UP * 5.0);
            //d = min(d, sphereSdf(p_centered, 0.8));
            //if (length(center_original_space.xz) > 3.7) {
            if (length(center_original_space.xz) > 2.5) {
                d = min(d, hexPrismSdf(p_centered, vec2f(0.577, 5.0)));
            }
        }
    }
    if (d < 10.0) {
        d = min(d, 2.0);
    }
    return d;
}


fn map_dir(p: vec3f, ray_direction: vec3f) -> f32 {
    var d = 1e20;
    
    var up = vec3f(0.0, 1.0, 0.0);
    var h = 4.0;
    var plane_dist = plane_ray_dist(p, ray_direction, up * h, up);
    if (plane_dist > 0.1) {
        return plane_dist - 0.05;
    }
    //d = min(d, plane_dist);

    for (var i = 0; i < 24; i = i + 1) {
        d = min(d, obstacleDist(p, ray_direction, i));
        if (d < 0.1) {
            return d;
        }
    }

    //d = min(d, plane_ray_dist(p, ray_direction, up, up));
    d = min(d, ground_distance(p));
    return d;
}


fn map(p_: vec3f) -> f32 {
    // forwards transform:
    // (x, y) -> x * (1, 0) + y * (0.5, 0.8660254) = (x, x * 0.5 + y * 0.8660254)
    // backwards transform:
    // (x, y) -> (x, (y - 0.5 * x) / 0.8660254)

    var p = p_;
    var d = 1e20;
    //d = min(d, dot(p, vec3(0.0, 1.0, 0.0)) - 1.0);
    
    d = min(d, ground_distance(p));
    

    for (var i = 0; i < 24; i = i + 1) {
        d = min(d, obstacleSdf(p, i));
    }
    return d;
}

fn normal(p: vec3f) -> vec3f {
    var e = vec3f(0.0001, 0.0, 0.0);
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
    var aspect = engine_uniforms.resolution_x / engine_uniforms.resolution_y;
    var uv = -(in.clip_position.xy / engine_uniforms.resolution_y - vec2f(aspect * 0.5, 0.5));

    var angle = engine_uniforms.mouse_x * DEG_TO_RAD;
    var angle_y = engine_uniforms.mouse_y * DEG_TO_RAD;
    //var angle = 0.0;
    //var angle_y = 70.0 * DEG_TO_RAD;
    //var angle_y = f32(engine_uniforms.frame) * 0.01;
    var direction = normalize(vec3f(sin(angle) * cos(angle_y), -sin(angle_y), -cos(angle) * cos(angle_y)));

    var up = vec3f(0.0, 1.0, 0.0);
    var cameraU = normalize(cross(up, direction));
    var cameraV = cross(direction, cameraU);

    var camera_origin = (-direction * 20.0) ;
    var camera_target = vec3f(0.0, 0.0, 0.0);
    var camera_direction = normalize(camera_target - camera_origin);
    var ray_direction = normalize(camera_direction*0.9 + uv.x * cameraU + uv.y * cameraV);

    for (var i = 0; i < 24; i = i + 1) {
        create_obstacle(i % 6, 10.0 - engine_uniforms.time + f32(i), i);
    }

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