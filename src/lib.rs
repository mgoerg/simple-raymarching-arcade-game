use log::Log;
use time::get_time_since_start;
use wgpu::util::DeviceExt;
use winit::event_loop::{ControlFlow, EventLoopWindowTarget};
use winit::window::{CursorGrabMode, Window};
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

pub mod game;
pub mod time;
pub mod wgsl_utils;

const MAX_WIDTH_WEB: u32 = 960;
const MAX_HEIGHT_WEB: u32 = 540;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::console;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::JsFuture;


#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct EngineUniforms {
    pub resolution_x: f32,
    pub resolution_y: f32,
    pub window_focused: i32,
    pub time: f32,
    //
    pub frame: i32,
    pub global_time: f32,
    pub mouse_x: f32,
    pub mouse_y: f32,
    //
}

pub struct Engine<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    window: &'a Window,
    render_pipeline: wgpu::RenderPipeline,
    engine_uniforms: EngineUniforms,
    engine_uniforms_buffer: wgpu::Buffer,
    engine_bind_group: wgpu::BindGroup,
    render_frame: i32,
    last_time_stamp: f64,
    frame_duration: f32,
    time_accumulator: f32,
    fps_counter: time::FpsCounter,
    surface_configured: bool,
    #[cfg(target_arch = "wasm32")]
    wait_until: f64,
}

fn set_cursor_position(window: &Window, position: winit::dpi::PhysicalPosition<f64>) {
    // only do this if we are not on a browser
    #[cfg(not(target_arch = "wasm32"))]
    {
        window.set_cursor_position(position).expect("Could not set cursor position");
    }
}


#[cfg(target_arch = "wasm32")]
macro_rules! log {
    ($($t:tt)*) => (console::log_1(&format!($($t)*).into()))
}
#[cfg(not(target_arch = "wasm32"))]
macro_rules! log {
    ($($t:tt)*) => (println!($($t)*))
}

impl<'a> Engine<'a> {
    async fn new(window: &'a Window) -> Engine<'a> {
        let mut size = window.inner_size();
        {
            if size.width > MAX_WIDTH_WEB {
                size.width = MAX_WIDTH_WEB;
            }
            if size.height > MAX_HEIGHT_WEB {
                size.height = MAX_HEIGHT_WEB;
            }
        }
        let target_size = size.clone();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web, we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: Default::default(),
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: target_size.width,
            height: target_size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };


        // Uniforms

        let engine_uniforms = EngineUniforms {
            resolution_x: target_size.width as f32,
            resolution_y: target_size.height as f32,
            window_focused: 1, // TODO: Implement window focus
            time: 0.0,
            frame: 0,
            global_time: 0.0,
            mouse_x: 0.0,
            mouse_y: 70.0,
        };

        let engine_uniforms_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Engine Uniforms Buffer"),
                contents: bytemuck::cast_slice(&[engine_uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let engine_uniforms_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("engine_uniforms_bind_group_layout"),
        });

        let engine_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &engine_uniforms_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: engine_uniforms_buffer.as_entire_binding(),
                }
            ],
            label: Some("engine_bind_group"),
        });


        // Pipeline
        let shader_code = if cfg!(target_arch = "wasm32") {
            include_str!("shaders/shader.wgsl").into()
        } else {
            let current_dir = std::env::current_dir().unwrap();
            std::fs::read_to_string("src/shaders/shader.wgsl")
                .expect(format!("Failed to read shader file {}{}", current_dir.display(), "shaders/shader.wgsl").as_str())
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &engine_uniforms_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main", 
                buffers: &[], // 2.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { 
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, 
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, 
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, 
            multisample: wgpu::MultisampleState {
                count: 1, 
                mask: !0, 
                alpha_to_coverage_enabled: false, 
            },
            multiview: None, 
            cache: None, 
        });



        let fps_counter = time::FpsCounter::new(0.1);


        Self {
            window: window,
            surface,
            device,
            queue,
            config,
            size: target_size,
            render_pipeline,
            engine_uniforms,
            engine_uniforms_buffer,
            engine_bind_group,
            render_frame: 0,
            last_time_stamp: (time::get_time_since_start()), // in seconds
            frame_duration: 1.0 / 60.0,
            time_accumulator: 0.0,
            fps_counter: fps_counter,
            surface_configured: false,
            #[cfg(target_arch = "wasm32")]
            wait_until: 0.0,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        let mut target_size = new_size;
        // for browsers we see requests to resize to unreasonable sizes, we limit the size to MAX_WIDTH_WEBxMAX_HEIGHT_WEB
        {
            if target_size.width > MAX_WIDTH_WEB {
                target_size.width = MAX_WIDTH_WEB;
            }
            if target_size.height > MAX_HEIGHT_WEB {
                target_size.height = MAX_HEIGHT_WEB;
            }
        }
        let size = target_size.clone();
    
        if size.width > 0 && size.height > 0 {
            self.size = size;
            self.config.width = self.size.width;
            self.config.height = self.size.height;
            self.surface.configure(&self.device, &self.config);
            self.engine_uniforms.resolution_x = self.size.width as f32;
            self.engine_uniforms.resolution_y = self.size.height as f32;
            let screen_center: winit::dpi::PhysicalPosition<f64> = winit::dpi::PhysicalPosition::new(self.size.width as f64 / 2.0, self.size.height as f64 / 2.0);
            set_cursor_position(&self.window, screen_center);
        }
    }
    
    fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    fn update(&mut self) {
        // pass
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.engine_uniforms.time = self.render_frame as f32 / 60.0;
        self.engine_uniforms.frame = self.render_frame;
        self.queue.write_buffer(&self.engine_uniforms_buffer, 0, bytemuck::cast_slice(&[self.engine_uniforms]));

        self.engine_uniforms.global_time = get_time_since_start() as f32;
        if (self.render_frame % 10) == 0 {
            let window_title = format!("Frame {}, FPS: {:.2}, UPS: {:.2}", self.render_frame, self.fps_counter.fps(), self.fps_counter.ups());
            self.window.set_title(&window_title);
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    // This is what @location(0) in the fragment shader targets
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(
                                #[cfg(target_arch = "wasm32")]
                                wgpu::Color {
                                    r: 0.2,
                                    g: 0.2,
                                    b: 0.8,
                                    a: 1.0,
                                },
                                #[cfg(not(target_arch = "wasm32"))]
                                wgpu::Color {
                                    r: 0.2,
                                    g: 0.2,
                                    b: 0.2,
                                    a: 1.0,
                                }
                            ),
                            store: wgpu::StoreOp::Store,
                        }
                    })
                ],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.engine_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    async fn handle_window_event(&mut self, event: &WindowEvent, event_loop_window_target: &EventLoopWindowTarget<()>) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => event_loop_window_target.exit(),
            WindowEvent::Resized(physical_size) => {
                log::info!("physical_size: {physical_size:?}");
                self.surface_configured = true;
                self.resize(*physical_size);
            }
            WindowEvent::CursorMoved {device_id, position} => {
                let screen_center: winit::dpi::PhysicalPosition<f64> = winit::dpi::PhysicalPosition::new(self.size.width as f64 / 2.0, self.size.height as f64 / 2.0);
                let screen_center_cg = cgmath::Vector2::new(screen_center.x as f32, screen_center.y as f32);

                //print!("Cursor moved: {:?}", position);
                const SENSITIVITY: f32 = 40.0;
                let size_height = self.size.height as f32;
                let position = cgmath::Vector2::new(position.x as f32, position.y as f32);
                
                let pos = (position - screen_center_cg) / size_height * SENSITIVITY;
                let pos_x = pos.x;
                let pos_y = pos.y;

                self.engine_uniforms.mouse_x += pos_x;
                // modulo 360
                self.engine_uniforms.mouse_x = self.engine_uniforms.mouse_x % 360.0;
                self.engine_uniforms.mouse_y += pos_y as f32;
                // clamp between -90 and 90
                self.engine_uniforms.mouse_y = self.engine_uniforms.mouse_y.max(-89.0).min(89.0);
                set_cursor_position(&self.window, screen_center);
            }

            WindowEvent::RedrawRequested => {
                // This tells winit that we want another frame after this one
                self.window().request_redraw();

                if !self.surface_configured {
                    return;
                }
                #[cfg(target_arch = "wasm32")]
                {
                    if get_time_since_start() < self.wait_until {
                        return;
                    }
                }
                
                if self.time_accumulator > 0.0 {
                    self.last_time_stamp = get_time_since_start();
                    self.time_accumulator -= 1.0 / 60.0;
                    self.update();
                    self.fps_counter.on_update();
                    let elapsed = (get_time_since_start() -  self.last_time_stamp) as f32;
                    self.time_accumulator += elapsed;
                } 
                if self.time_accumulator < 1.0 / 60.0 {
                    self.last_time_stamp = get_time_since_start();
                    match self.render() {
                        Ok(_) => {}
                        // Reconfigure the surface if it's lost or outdated
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            self.resize(self.size)
                        }
                        // The system is out of memory, we should probably quit
                        Err(wgpu::SurfaceError::OutOfMemory) => {
                            log::error!("OutOfMemory");
                            event_loop_window_target.exit();
                        }

                        // This happens when the frame takes too long to present
                        Err(wgpu::SurfaceError::Timeout) => {
                            log::warn!("Surface timeout")
                        }
                    }
                    let elapsed = (get_time_since_start() -  self.last_time_stamp) as f32;
                    self.time_accumulator += elapsed;
                    self.last_time_stamp = get_time_since_start();
                    self.render_frame += 1;
                    self.fps_counter.on_render();
                    if self.time_accumulator < 1.0 / 60.0 {
                        let delta = 1.0 / 60.0 - self.time_accumulator;
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            std::thread::sleep(std::time::Duration::from_secs_f64(delta as f64));  
                        }
                        #[cfg(target_arch = "wasm32")]
                        { self.wait_until = get_time_since_start() + delta as f64; }
                        self.time_accumulator += delta;
                    }
                }
            }
            _ => {}
        }
    }
}


#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Warn).expect("Couldn't initialize logger");
            //wasm_logger::init(wasm_logger::Config::default());
            console::log_1(&"Hello from Rust!".into());
        } else {
            env_logger::init();
            log::info!("Hello from Rst!");
        }
    }


    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Failed to create window");

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        let _ = window.request_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        window.set_cursor_grab(CursorGrabMode::Confined).expect("Failed to set cursor grab mode");
        window.set_cursor_visible(false);
    }
    set_cursor_position(&window, winit::dpi::PhysicalPosition::new(40.0, 40.0));

    let mut engine = Engine::new(&window).await;

    #[cfg(target_arch = "wasm32")]
    event_loop.run(
            move |event, event_loop_window_target| {
                event_loop_handler(event, event_loop_window_target, &mut engine);
            },
        )
        .expect("Failed during event loop. Exiting.");

    #[cfg(not(target_arch = "wasm32"))]
    event_loop.run(move |event, event_loop_window_target| {
        futures::executor::block_on(event_loop_handler(event, event_loop_window_target, &mut engine));
    }).expect("Failed during event loop. Exiting.");
}

pub async fn event_loop_handler(event: Event<()>, event_loop_window_target: &EventLoopWindowTarget<()>, engine: &mut Engine<'_>) {
    match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == engine.window.id() => {
            if !engine.input(event) {
                engine.handle_window_event(event, event_loop_window_target).await;
            }
        }
        _ => {}
    }
}