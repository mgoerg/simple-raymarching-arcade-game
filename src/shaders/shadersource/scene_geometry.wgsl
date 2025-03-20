



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


fn extrudeSdf(p_z: f32, dxy: f32, h: f32) -> f32 {
    var w = vec2f(dxy, abs(p_z) - h);
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

// fn obstacleShapeSdf(p: vec3f, q: vec3f) -> f32 {
//     var radius = length(q.xz);
//     var thickness = 0.25;
//     return max(hollowSphereSdf(p, radius + thickness * 0.5, thickness), sphereSdf(p - vec3f(q.x, 0.0, q.z), 1.5));
// }

fn vesicaSdf(p: vec3f, a: vec3f, b: vec3f, _w: f32) -> f32 {
    var c = (a + b) * 0.5;
    var h = length(b - a);
    var v = (b - a) / h;
    var y = dot(p - c, v);
    var q = vec2f(length(p - c - y * v), abs(y));
    h = h * 0.5;
    var w = _w * 0.5;
    var d = 0.5 * (h * h - w * w) / w;
    var t = select(vec3f(-d, 0.0, d + w), vec3f(0.0, h, 0.0), h * q.x < d * (q.y - h));
    return length(q - t.xy) - t.z;
}

fn vesicaSdf2(p: vec3f, c: vec3f, ab2: vec3f, w: f32) -> f32 {
    var h = length(ab2) * 2.0;
    var v = normalize(ab2);
    var y = dot(p - c, v);
    var q = vec2f(length(p - c - y * v), abs(y));
    var _h = h * 0.5;
    var _w = w * 0.5;
    var d = 0.5 * (_h * _h - _w * _w) / _w;
    var t = select(vec3f(0.0, _h, 0.0), vec3f(-d, 0.0, d + w), _h * q.x < d * (q.y - _h));
    return length(q - t.xy) - t.z;
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

fn trapezoidSdf(_p: vec2f, r1: f32, r2: f32, he: f32) -> f32 {
    var k1 = vec2f(r2, he);
    var k2 = vec2f(r2 - r1, 2.0 * he);
    var p = vec2f(abs(_p.x), _p.y);
    p.x = abs(p.x);
    let ri = select(r2, r1, p.y < 0.0);
    var ca = vec2f(p.x - min(p.x, ri), abs(p.y) - he);
    var cb = p - k1 + k2 * clamp(dot(k1 - p, k2) / dot(k2, k2), 0.0, 1.0);
    var s = select(1.0, -1.0, cb.x < 0.0 && ca.y < 0.0);
    return s * sqrt(min(dot(ca, ca), dot(cb, cb)));
}


fn obstacleSdfPlanes(p: vec3f, i: i32) -> f32 {
    var o = g_obstacles[i];
    var lane = o.lane;
    var start = o.start;
    var end = o.end;

    // if below 1, move very slowly to 0
    start = select(start, -1.0 / (start - 2.0), start < 1.0);
    end = select(end, -1.0 / (end - 2.0), end < 1.0);

    // start = 4.0 - g_engine.global_time * 0.5;
    // end = 4.0;

    var r1 = end * 0.57735;
    var r2 = start * 0.57735;
    var height = end - start;

    var rot_mat = o.rotation;

    var p_2d = vec2f(p.x, p.z);
    var p1 = rot_mat * p_2d;
    var p2 = p1 + vec2f(0.0, (start + end) / 2.0);
    // rotate p_2d by angle
    // var p_2d_rotated = o.rotation * vec2f(p.x, p.z);
    var dist_2d = trapezoidSdf(p2, r1, r2, height * 0.5);

    var OBSTACLE_HEIGHT = 0.5;

    var dist_2d2 = length(p.xz - vec2f(1.0, 0.0)) - 0.5;

    return extrudeSdf(p.y - 1.5, dist_2d, OBSTACLE_HEIGHT);
}

// fn obstacleSdf(p: vec3f, i: i32) -> f32 {
//     //return sphereSdf(p - g_obstacles[i].center.xyz, 1.5);
    
//     var p_lane = floor(atan2(p.x, p.z) / 6.2831853 * 6.0);
//     var lane = f32(g_obstacles[i].lane);
//     var delta_angle = abs(p_lane - lane);
//     if ( delta_angle % 6.0 > 1.0) {
//         return 1000.0;
//     }
//     var o = g_obstacles[i];
//     var d = hollowSphereSdf(p + o.ctr_a.xyz, o.r1, o.thickness);
//     d = smax(d, ellipsoidSdf(p - o.center.xyz.xyz, o.ellipsoid_radii.xyz));
//     return d;
    
// }

fn obstacle_distance_dir(p: vec3f, rd: vec3f, i: i32) -> f32 {
    return obstacleSdfPlanes(p, i);
}

fn obstacle_distance(p: vec3f, i: i32) -> f32 {
    return obstacleSdfPlanes(p, i);
}

fn player_distance(p: vec3f) -> f32 {
    var position = g_game.player_position.xyz;
    var width = g_game.player_width;
    var tangent = g_game.player_tangent.xyz;

    return vesicaSdf(p, position + tangent * width, position - tangent * width, width + 0.2);
}

struct GroundDistanceMapRval {
    d: f32,
    hex_center: vec3f,
}

fn ground_distance(p: vec3f) -> GroundDistanceMapRval {
    var rval = GroundDistanceMapRval();
    rval.d = 1e20;
    // Hex grid. First obtain center points by transforming to normal 2d grid. Then use sdf for hexagon prisms wrt center points.
    // forwards transform:
    // (x, y) -> x * (1, 0) + y * (0.5, 0.8660254) = (x, x * 0.5 + y * 0.8660254)
    // backwards transform:
    // (x, y) -> (x, (y - 0.5 * x) / 0.8660254)
    var l = sqrt3inv * 2.0;
    var l2 = 0.5;
    var p_transformed = vec2f(p.x, (p.z - l2 * p.x) / l);
    var center_transformed = vec2f(floor(p_transformed.x), floor(p_transformed.y));
    for (var x = -1.0; x <= 2.0; x = x + 1.0) {
        for (var y = -1.0; y <= 2.0; y = y + 1.0) {
            var center_xy = vec2f(center_transformed.x + x, center_transformed.y + y);
            // var hash2d = textureSample(t_noise2d, s_noise2d, center_xy).r;
            // var hash2d = -1.0;
            var hash2d = hash12(center_xy);
            let height_offset = hash2d;
            var center_original_space = vec3f(center_xy.x, height_offset, center_xy.y * l + l2 * center_xy.x);
            var l3 = length(center_original_space.xz);
            center_original_space += UP * clamp(10.0 / (l3 * l3 * l3 * l3) - 1.0, 0.0, 2.1);

            var p_centered = (p - center_original_space + UP * 5.0);
            //d = min(d, sphereSdf(p_centered, 0.8));
            //if (length(center_original_space.xz) > 3.7) {
            // if (length(center_original_space.xz) > 2.5) {
            var current_d = hexPrismSdf(p_centered, vec2f(0.577, 5.0));
            if rval.d > current_d {
                rval.d = current_d;
                rval.hex_center = center_original_space;
            }
            // }
        }
    }
    // This removes some artifacts
    if (rval.d < 10.0) {
        rval.d = min(rval.d, 2.0);
    }
    return rval;
}

fn ground_distance_dir(p: vec3f, ray_direction: vec3f) -> GroundDistanceMapRval {
    var up = vec3f(0.0, 1.0, 0.0);
    var h = 4.0;
    var plane_dist = plane_ray_dist(p, ray_direction, up * h, up);
    if (plane_dist > 0.5) {
        var rval = GroundDistanceMapRval();
        rval.d = plane_dist - 0.25;
        rval.hex_center = vec3f(0.0);
        return rval;
    }
    return ground_distance(p);
}

fn map_dir(p: vec3f, ray_direction: vec3f) -> f32 {
    var d = 1e20;

    for (var i = 0; i < g_obstacle_globals.count; i = i + 1) {
        d = min(d, obstacle_distance_dir(p, ray_direction, i));
        if (d < 0.01) {
            return d;
        }
    }

    d = min(d, ground_distance_dir(p, ray_direction).d);
    d = min(d, player_distance(p));
    return d;
}


fn map(p: vec3f) -> f32 {
    var d = 1e20;
    
    d = min(d, ground_distance(p).d);
    d = min(d, player_distance(p));

    for (var i = 0; i < g_obstacle_globals.count; i = i + 1) {
        d = min(d, obstacle_distance(p, i));
    }
    return d;
}



fn map_color(p: vec3f) -> i32 {
    var d = 1e20;
    var rval = -1;
    var gnd_dist_rval = ground_distance(p);
    if d > gnd_dist_rval.d {
        d = gnd_dist_rval.d;
        if (length(gnd_dist_rval.hex_center.xz) < 1.5) {
            rval = 2;
        } else {
            rval = 0;
        }
    }
    var player_dist = player_distance(p);
    if d > player_dist {
        d = player_dist;
        rval = 1;
    }

    for (var i = 0; i < g_obstacle_globals.count; i = i + 1) {
        var od = obstacle_distance(p, i);
        if d > od {
            d = od;
            rval = 2;
        }
    }

    return rval;
}
