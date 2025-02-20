struct Obstacle {
    rotation: mat2x2<f32>,
    start: f32,
    end: f32,
    lane: u32,
    _padding: f32,
}

struct ObstacleGlobal {
    count: i32,
}
