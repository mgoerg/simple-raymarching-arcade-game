



#[cfg(target_arch = "wasm32")]
// time in seconds
pub fn get_time_since_start() -> f64 {
    use wasm_bindgen::prelude::*;
    use web_sys::window;
    let performance = window().unwrap().performance().unwrap();

    log::info!("get_time_since_start: {}", performance.now());

    performance.now() / 1000.0
}

#[cfg(not(target_arch = "wasm32"))]
// time in seconds
pub fn get_time_since_start() -> f64 {
    use std::time::Instant;
    use once_cell::sync::Lazy;
    static START: Lazy<Instant> = Lazy::new(|| std::time::Instant::now());
    START.elapsed().as_secs_f64()
}

pub struct FpsCounter {
    fps: f32,
    ups: f32,
    alpha: f32,
    last_render_time: f64,
    last_update_time: f64,
}

impl FpsCounter {
    pub fn new(alpha: f32) -> Self {
        Self {
            fps: 0.0,
            ups: 0.0,
            alpha,
            last_render_time: get_time_since_start(),
            last_update_time: get_time_since_start(),
        }
    }

    pub fn on_render(&mut self) {
        let current_time = get_time_since_start();
        let elapsed = current_time - self.last_render_time;
        self.last_render_time = current_time;

        let current_fps = 1.0 / elapsed as f32;
        self.fps = self.alpha * current_fps + (1.0 - self.alpha) * self.fps;
    }

    pub fn on_update(&mut self) {
        let current_time = get_time_since_start();
        let elapsed = current_time - self.last_update_time;
        self.last_update_time = current_time;

        let current_ups = 1.0 / elapsed as f32;
        self.ups = self.alpha * current_ups + (1.0 - self.alpha) * self.ups;
    }

    pub fn fps(&self) -> f32 {
        self.fps
    }

    pub fn ups(&self) -> f32 {
        self.ups
    }
}