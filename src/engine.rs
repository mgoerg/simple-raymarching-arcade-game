use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
    window::Window,
};

use crate::{
    game::Game, 
    input::{
        InputDriveInterface, 
        InputHandler
    }, 
    time::get_time_since_start,
    renderer::Renderer,
};



pub struct EngineContext<'window> {
    pub input: &'window InputHandler<'window>,
    pub renderer: &'window Renderer<'window>,
}

pub struct Engine<'window> {
    pub window: &'window Window,
    pub game: Game,
    pub input: InputHandler<'window>,
    pub renderer: Renderer<'window>,

    // We keep track of frames/time
    pub last_time_stamp: f64,
    pub frame_duration: f64,
    pub time_accumulator: f64,
    pub fps_counter: crate::time::FpsCounter,

    #[cfg(target_arch = "wasm32")]
    pub wait_until: f64,
}

impl<'window> Engine<'window> {
    pub async fn new(window: &'window Window) -> Engine<'window> {
        // Create our Renderer
        let size = window.inner_size();
        let renderer = Renderer::new(window, size).await;

        // Initialize time-related fields
        let fps_counter = crate::time::FpsCounter::new();
        let now = get_time_since_start();

        let mut game = Game::new(size.width as f32 / size.height as f32);
        let mut input = InputHandler::new(window);
        input.activate();

        let context = EngineContext {
            input: &input,
            renderer: &renderer,
        };

        Engine {
            window,
            renderer,
            input,
            game,
            // context,
            last_time_stamp: now,
            frame_duration: 1.0 / 60.0,
            time_accumulator: 0.0,
            fps_counter,
            #[cfg(target_arch = "wasm32")]
            wait_until: 0.0,
        }
    }

    /// Handle input (keyboard, mouse, etc.)
    pub fn input(&mut self, event: &WindowEvent) -> bool {
        // Return `true` if event has been handled to prevent further processing
        false
    }

    /// Update logic (no rendering) each discrete timestep
    pub fn update(&mut self, dt: f32) {
        self.input.update(dt);
        let engine_context = EngineContext {
            input: &mut self.input,
            renderer: &mut self.renderer,
        };
        self.game.update(dt, &engine_context);
        // self.game.update(dt, &self.input);
    }

    /// then let the `Renderer` do the actual GPU updates + rendering.
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let res = self.renderer.render(&self.game, &self.input);

        if (self.renderer.render_frame % 10) == 0 {
            let title = format!(
                "Frame {}, FPS: {:.2}, UPS: {:.2}",
                self.renderer.render_frame,
                self.fps_counter.fps(),
                self.fps_counter.ups()
            );
            self.window.set_title(&title);
        }

        res
    }

    /// Called whenever the window is resized.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.resize(new_size);
        self.input.window_resized(new_size);
    }


    /// Called each time a `WindowEvent` happens.
    /// This is invoked from the event loop, so it can handle everything
    /// from closing the window to resizing, etc.
    pub async fn handle_window_event(
        &mut self,
        event: &WindowEvent,
        event_loop_window_target: &EventLoopWindowTarget<()>,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop_window_target.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                match event {
                    KeyEvent {
                        state: ElementState::Pressed, 
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    } => {
                        event_loop_window_target.exit();
                        return;
                    }
                    _ => {}
                }
                self.input.handle_event(event);
            }
            WindowEvent::Resized(physical_size) => {
                self.renderer.surface_configured = true;
                self.resize(*physical_size);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.input.cursor_moved(position);
            }
            WindowEvent::RedrawRequested => {
                // request redraw
                self.window.request_redraw();

                if !self.renderer.surface_configured {
                    return;
                }
                #[cfg(target_arch = "wasm32")]
                {
                    if get_time_since_start() < self.wait_until {
                        return;
                    }
                }

                // Time stepping
                let now = get_time_since_start();
                let dt = now - self.last_time_stamp;
                self.last_time_stamp = now;

                self.time_accumulator += dt;
                let mut updates_count = 0;
                while self.time_accumulator >= self.frame_duration && updates_count < 1 {
                    self.time_accumulator -= self.frame_duration;
                    self.update(dt as f32);
                    self.fps_counter.on_update();
                    updates_count += 1;
                }

                // Render
                let frame_start_time = get_time_since_start();
                match self.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        self.resize(self.renderer.size);
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        log::error!("OutOfMemory");
                        event_loop_window_target.exit();
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        log::warn!("Surface timeout");
                    }
                    Err(e) => {
                        log::error!("Failed to render: {:?}", e);
                    }
                }
                self.fps_counter.on_render();

                let frame_time = get_time_since_start() - frame_start_time;
                if frame_time < self.frame_duration {
                    let remain = self.frame_duration - frame_time;

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        std::thread::sleep(std::time::Duration::from_secs_f64(remain));
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        self.wait_until = get_time_since_start() + remain;
                    }
                }
            }
            _ => {}
        }
    }
}
