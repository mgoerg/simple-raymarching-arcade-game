



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
const MAX_SAMPLES: usize = 100;

/// Stores a ring buffer of frame/update times for FPS/UPS computations.
pub struct FpsCounter {
    // Ring buffer for storing the latest computed FPS values
    fps_values: [f64; MAX_SAMPLES],
    fps_sum: f64,
    fps_index: usize,

    // Ring buffer for storing the latest computed UPS values
    ups_values: [f64; MAX_SAMPLES],
    ups_sum: f64,
    ups_index: usize,

    // Tracks how many valid samples are currently in the buffers
    sample_count: usize,

    // Last times we recorded a render or an update
    last_render_time: f64,
    last_update_time: f64,
}

impl FpsCounter {
    pub fn new() -> Self {
        let now = get_time_since_start();
        Self {
            fps_values: [0.0; MAX_SAMPLES],
            fps_sum: 0.0,
            fps_index: 0,

            ups_values: [0.0; MAX_SAMPLES],
            ups_sum: 0.0,
            ups_index: 0,

            sample_count: 0,

            last_render_time: now,
            last_update_time: now,
        }
    }

    /// Call this every time you render a frame.
    pub fn on_render(&mut self) {
        let current_time = get_time_since_start();
        let elapsed = current_time - self.last_render_time;
        self.last_render_time = current_time;

        if elapsed > 0.0 {
            let new_fps = 1.0 / elapsed;

            // Remove the old FPS value at this index from the sum
            let old_fps = self.fps_values[self.fps_index];
            self.fps_sum -= old_fps;

            // Insert the new FPS value
            self.fps_values[self.fps_index] = new_fps;
            self.fps_sum += new_fps;

            // Advance the ring index
            self.fps_index = (self.fps_index + 1) % MAX_SAMPLES;

            // Increase sample_count until we reach the ring buffer capacity
            if self.sample_count < MAX_SAMPLES {
                self.sample_count += 1;
            }
        }
    }

    /// Call this every time you update your game/application logic.
    pub fn on_update(&mut self) {
        let current_time = get_time_since_start();
        let elapsed = current_time - self.last_update_time;
        self.last_update_time = current_time;

        if elapsed > 0.0 {
            let new_ups = 1.0 / elapsed;

            // Remove the old UPS value at this index from the sum
            let old_ups = self.ups_values[self.ups_index];
            self.ups_sum -= old_ups;

            // Insert the new UPS value
            self.ups_values[self.ups_index] = new_ups;
            self.ups_sum += new_ups;

            // Advance the ring index
            self.ups_index = (self.ups_index + 1) % MAX_SAMPLES;

            // Increase sample_count if it hasn't reached capacity yet
            if self.sample_count < MAX_SAMPLES {
                self.sample_count += 1;
            }
        }
    }

    /// Returns the average FPS over the last `sample_count` frames.
    pub fn fps(&self) -> f32 {
        if self.sample_count == 0 {
            0.0
        } else {
            // Average of ring buffer values = sum / number of valid samples
            (self.fps_sum / self.sample_count as f64) as f32
        }
    }

    /// Returns the average UPS over the last `sample_count` updates.
    pub fn ups(&self) -> f32 {
        if self.sample_count == 0 {
            0.0
        } else {
            (self.ups_sum / self.sample_count as f64) as f32
        }
    }
}
