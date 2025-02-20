use winit::{event::Event, event_loop::EventLoopWindowTarget};

mod engine;
mod renderer;
mod time;
mod input;
mod game;
mod common;

pub async fn run() {
    // Initialize logging, window, etc., the same as before.
    let event_loop = winit::event_loop::EventLoop::new().expect("Failed to create event loop");
    let window = winit::window::WindowBuilder::new()
        .build(&event_loop)
        .expect("Failed to create window");

    // On WASM, insert the canvas, etc. (omitted here for brevity)

    // Create our Engine
    let mut engine = crate::engine::Engine::new(&window).await;

    #[cfg(not(target_arch = "wasm32"))]
    event_loop
        .run(move |event, event_loop_window_target| {
            // On native, we can block on futures:
            futures::executor::block_on(event_loop_handler(
                event,
                event_loop_window_target,
                &mut engine,
            ));
        })
        .expect("Event loop failed");
    
    #[cfg(target_arch = "wasm32")]
    event_loop
        .run(move |event, event_loop_window_target| {
            // On wasm, typically just run async in a callback
            event_loop_handler(event, event_loop_window_target, &mut engine);
        })
        .expect("Event loop failed");
}

pub async fn event_loop_handler(
    event: Event<()>,
    event_loop_window_target: &EventLoopWindowTarget<()>,
    engine: &mut crate::engine::Engine<'_>,
) {
    match event {
        Event::WindowEvent { event, window_id } if window_id == engine.window.id() => {
            if !engine.input(&event) {
                engine.handle_window_event(&event, event_loop_window_target).await;
            }
        }
        _ => {}
    }
}
