
use cgmath::{EuclideanSpace, InnerSpace, Vector3};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::common::camera;
use crate::game::{self, Game};
use crate::input::{InputHandler, InputGetInterface};
use crate::time::get_time_since_start;

const MAX_WIDTH_WEB: u32 = 480;
const MAX_HEIGHT_WEB: u32 = 270;

// pub trait UniformBlock: bytemuck::Pod + bytemuck::Zeroable + 'static {
//     /// A textual label used for debugging/logging
//     const LABEL: &'static str;
//     /// Which shader stages use this uniform
//     const VISIBILITY: wgpu::ShaderStages;

//     type SourceData;

//     fn initial_data() -> Self {
//         Self::zeroed() // or some other default
//     }
// }

// pub struct UniformManager {
//     pub uniforms: Vec<Box<dyn UniformBlock>>,
// }

// impl UniformManager {
//     pub fn register_block<T: UniformBlock>(&mut self) {
//         self.uniforms.push(Box::new(T::initial_data()));
//     }
//     pub fn write_blocks(&self, device: &wgpu::Device, queue: &wgpu::Queue) {
//         for uniform in self.uniforms.iter() {
//             let data = uniform as &dyn UniformBlock;
//             queue.write_buffer(&data.buffer, 0, bytemuck::cast_slice(&[data]));
//         }
//     }
// }

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct EngineUniforms {
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

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub position: [f32; 4],
    pub direction: [f32; 4],
    pub up: [f32; 4],
    pub u_dir: [f32; 4],
    pub v_dir: [f32; 4],
}

impl CameraUniforms {
    pub fn new(position: Vector3<f32>, direction: Vector3<f32>, up: Vector3<f32>) -> Self {
        let right = -direction.cross(up).normalize();
        Self {
            position: vector3_to_array4(position),
            direction: vector3_to_array4(direction),
            up: vector3_to_array4(up),
            u_dir: vector3_to_array4(right),
            v_dir: vector3_to_array4(-right.cross(direction)), 

        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GameUniforms {
    player_angle: f32,
    player_width: f32,
    _padding: [f32; 2],
    player_position: [f32; 4],
    player_tangent: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObstactleGlobalUniform {
    pub count: i32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObstacleUniform {
    rotation: [f32; 4],
    start: f32,
    end: f32,
    lane: u32,
    _padding: [f32; 1],
}

fn smoothstep(edge0: f32, edge1: f32, t: f32) -> f32 {
    let t = ((t - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn vector3_to_array4(v: Vector3<f32>) -> [f32; 4] {
    [v.x, v.y, v.z, 0.0]
}

fn mat2x2_to_array4(m: cgmath::Matrix2<f32>) -> [f32; 4] {
    [m.x.x, m.x.y, m.y.x, m.y.y]
}

impl ObstacleUniform {
    pub fn new(lane: i32, start: f32, end: f32) -> Self {
        let angle = lane as f32 / (6 as f32) * std::f32::consts::PI * 2.0;
        let rotation = cgmath::Matrix2::from_angle(cgmath::Rad(-angle));
        Self {
            lane:lane.try_into().unwrap(),
            start,
            end,
            rotation: mat2x2_to_array4(-rotation),
            _padding: [0.0],
        }
    }
}


pub struct Renderer<'a> {
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,

    pub render_frame: i32,

    // Pipeline(s)
    pub render_pipeline: wgpu::RenderPipeline,

    // All GPU-based uniform data
    pub uniforms: Uniforms,

    pub noise_texture_bind_group: wgpu::BindGroup,

    // Keep track of the current size so we can handle resizes
    pub size: PhysicalSize<u32>,

    // For tracking whether we configured the surface at least once.
    // (useful especially for web since we may do lazy init.)
    pub surface_configured: bool,
}

impl<'a> Renderer<'a> {
    /// Create a new `Renderer` along with the necessary WGPU objects.
    pub async fn new(window: &'a Window, mut size: PhysicalSize<u32>) -> Renderer<'a> {
        // Clamp size for web, to avoid super-large surfaces
        if size.width > MAX_WIDTH_WEB {
            size.width = MAX_WIDTH_WEB;
        }
        if size.height > MAX_HEIGHT_WEB {
            size.height = MAX_HEIGHT_WEB;
        }

        // Create instance and surface
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        // Request adapter and device
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        // Surface configuration
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        // Create uniform buffers
        let uniforms = Self::setup_uniform_buffers(&device, size);

        // Create Texture
        let tex_width = 1024;
        let tex_height = 1024;

        let format = wgpu::TextureFormat::R32Float;
        let mut texture_data = vec![0.0; tex_width as usize * tex_height as usize];
        for y in 0..tex_height {
            for x in 0..tex_width {
                let i = y * tex_width + x;
                let p3 = cgmath::Vector3::new(x as f32, y as f32, x as f32) * 0.1031;
                let mut p3 = cgmath::Vector3::new(p3.x.fract(), p3.y.fract(), p3.z.fract());
                let u = cgmath::Vector3::new(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33);
                let v = cgmath::dot(p3, u);
                p3 += cgmath::Vector3::new(v, v, v);
                let value = ((p3.x + p3.y) * p3.z).fract();
                // fn hash12(p: vec2f) -> f32 {
                //     var p3 = fract(vec3f(p.xyx) * 0.1031);
                //     p3 += dot(p3, p3.yzx + 33.33);
                //     return fract((p3.x + p3.y) * p3.z);
                // }
                
                // let value = (x ^ y) as u16;
                // let mut value = 0;
                // value = (x + y) as u8;
                // if (x as u32) & 3 == 1 && (y as u32) & 3 == 1 {
                //     value = 100;
                // }
                // value = 0.0;
                texture_data[i as usize] = 1.0/0.0;
            }
        }
        let texture_size = wgpu::Extent3d {
            width: tex_width,
            height: tex_height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Noise Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: format,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[format],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(texture_data.as_slice()), 
            wgpu::ImageDataLayout {offset: 0, bytes_per_row: Some(tex_width * 4), rows_per_image: None}, 
            texture_size
        );
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let noise_texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("Noise Texture Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float {filterable: false},
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }, wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler (wgpu::SamplerBindingType::NonFiltering),
                count: None,
            }],
        });
        let noise_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &noise_texture_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            }, wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&texture_sampler),
            }],
            label: Some("Noise Texture Bind Group"),
        });

        // Create render pipeline
        let render_pipeline = Self::create_render_pipeline(
            &device,
            &config,
            &[&uniforms.engine_group_layout, &uniforms.obstacle_bind_group_layout, &noise_texture_bind_group_layout],
        )
        .await;

        Renderer {
            surface,
            device,
            queue,
            config,
            noise_texture_bind_group: noise_texture_bind_group,
            render_frame: 0,
            render_pipeline,
            uniforms,
            size,
            surface_configured: false,
        }
    }

    /// Create the pipeline responsible for rendering.
    async fn create_render_pipeline(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        bind_group_layouts: &[&wgpu::BindGroupLayout],
    ) -> wgpu::RenderPipeline {
        // Load WGSL code. On native, we can read from a file; on WASM, we embed it.
        let shader_code = if cfg!(target_arch = "wasm32") {
            include_str!("shaders/shaderbuild/main_scene.wgsl").into()
        } else {
            let current_dir = std::env::current_dir().unwrap();
            std::fs::read_to_string("src/shaders/shaderbuild/main_scene.wgsl").expect(
                format!(
                    "Failed to read shader file {}{}",
                    current_dir.display(),
                    "shaders/main_scene.wgsl"
                )
                .as_str(),
            )
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: bind_group_layouts,
                push_constant_ranges: &[],
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
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
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
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
        })
    }

    /// Create and initialize all uniform buffers (Engine + Camera + Obstacles).
    fn setup_uniform_buffers(device: &wgpu::Device, size: PhysicalSize<u32>) -> Uniforms {
        let engine_uniforms = EngineUniforms {
            resolution_x: size.width as f32,
            resolution_y: size.height as f32,
            window_focused: 1, // TODO: implement window focus logic
            time: 0.0,
            frame: 0,
            global_time: 0.0,
            mouse_x: 80.0,
            mouse_y: 80.0,
        };

        let engine_uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Engine Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[engine_uniforms]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let camera_uniforms = CameraUniforms::new(
            Vector3::new(0.0, 0.0, 20.0),
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(0.0, 1.0, 0.0),
        );
        let camera_uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniforms]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let (engine_group_layout, engine_bind_group) = Self::create_simple_bind_group(
            &[&engine_uniforms_buffer, &camera_uniforms_buffer],
            device,
            "Engine",
            wgpu::ShaderStages::FRAGMENT,
        );

        // Game uniform data
        let game_uniforms = GameUniforms {
            player_angle: 0.0,
            player_width: 0.5,
            _padding: [0.0; 2],
            player_position: [0.0, 0.0, 0.0, 0.0],
            player_tangent: [0.0, 0.0, 0.0, 0.0],
        };
        let game_uniforms_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Game Uniforms Buffer"),
            contents: bytemuck::cast_slice(&[game_uniforms]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let obstacle_uniforms = vec![ObstacleUniform::new(0, 10.0, 20.0); 24];
        let obstacle_globals = ObstactleGlobalUniform { count: 24 };

        let obstacle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Obstacle Buffer"),
            contents: bytemuck::cast_slice(&obstacle_uniforms),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let obstacle_globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Obstacle Globals Buffer"),
            contents: bytemuck::cast_slice(&[obstacle_globals]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let (game_group_layout, obstacle_bind_group) = Self::create_simple_bind_group(
            &[&game_uniforms_buffer, &obstacle_globals_buffer, &obstacle_buffer],
            device,
            "Obstacle",
            wgpu::ShaderStages::FRAGMENT,
        );

        Uniforms {
            engine_uniforms,
            engine_uniforms_buffer,
            camera_uniforms,
            camera_uniforms_buffer,
            engine_bind_group,
            engine_group_layout,

            game_uniforms,
            game_uniforms_buffer,
            obstacle_uniforms,
            obstacle_buffer,
            obstacle_globals,
            obstacle_globals_buffer,
            obstacle_bind_group_layout: game_group_layout,
            obstacle_bind_group,
        }
    }

    /// Helper to create a bind group & layout for a list of buffers
    fn create_simple_bind_group(
        buffers: &[&wgpu::Buffer],
        device: &wgpu::Device,
        label: &str,
        visibility: wgpu::ShaderStages,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let entries: Vec<wgpu::BindGroupLayoutEntry> = buffers
            .iter()
            .enumerate()
            .map(|(i, _)| wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            })
            .collect();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &entries,
            label: Some(&format!("{} Bind Group Layout", label)),
        });

        let bind_group_entries = buffers
            .iter()
            .enumerate()
            .map(|(i, buffer)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect::<Vec<_>>();

        // Create bind group for the data
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: bind_group_entries.as_slice(),
            label: Some(&format!("{} Bind Group", label)),
        });

        (bind_group_layout, bind_group)
    }

    /// Resize and reconfigure the surface.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        let mut target_size = new_size;
        // For browsers we see requests to large sizes, so clamp them:
        if target_size.width > MAX_WIDTH_WEB {
            target_size.width = MAX_WIDTH_WEB;
        }
        if target_size.height > MAX_HEIGHT_WEB {
            target_size.height = MAX_HEIGHT_WEB;
        }

        if target_size.width > 0 && target_size.height > 0 {
            self.size = target_size;
            self.config.width = self.size.width;
            self.config.height = self.size.height;
            self.surface.configure(&self.device, &self.config);

            // Update engine uniforms
            self.uniforms.engine_uniforms.resolution_x = self.size.width as f32;
            self.uniforms.engine_uniforms.resolution_y = self.size.height as f32;

            self.surface_configured = true;
        }
    }

    fn update_engine_uniforms(&mut self, mouse_x: f32, mouse_y: f32) -> () {
        let engine_uniforms = &mut self.uniforms.engine_uniforms;
        engine_uniforms.time = self.render_frame as f32 / 60.0; //TODO use actual time instead
        engine_uniforms.frame = self.render_frame;
        engine_uniforms.global_time = get_time_since_start() as f32;
        engine_uniforms.mouse_x = mouse_x;
        engine_uniforms.mouse_y = mouse_y;
    }

    fn update_game_uniforms(&mut self, game: &game::Game) -> () {
        let game_uniforms = &mut self.uniforms.game_uniforms;
        game_uniforms.player_angle = game.player_angle;
        game_uniforms.player_width = game.player_width * Game::PLAYER_RADIUS / 2.0;
        let player_position = game.player_position();
        game_uniforms.player_position = vector3_to_array4(player_position);
        game_uniforms.player_tangent = vector3_to_array4(player_position.normalize().cross(Vector3::unit_y()));
    }

    fn update_camera_uniforms(&mut self, camera: &camera::Camera) -> () {
        self.uniforms.camera_uniforms = CameraUniforms::new(camera.eye.to_vec(), camera.direction(), camera.up);
    }

    fn update_obstacles(&mut self, obstacles: Vec<game::Obstacle>) -> () {
        let obstacle_data = &mut self.uniforms.obstacle_uniforms;
        let length = obstacles.len().min(24);
        for i in 0..24 {
            if i >= length {
                obstacle_data[i] = ObstacleUniform::new(0, 0.0, 0.0);
                continue;
            }
            let obs = &obstacles[i];
            let lane = obs.lane;
            let start = obs.start;
            let end = obs.end;
            obstacle_data[i] = ObstacleUniform::new(lane as i32, start, end);
        }

        self.uniforms.obstacle_globals.count = obstacles.len() as i32;
    }

    fn write_buf<T: bytemuck::Pod>(&self, buffer: &wgpu::Buffer, data: &[T]) {
        self.queue.write_buffer(buffer, 0, bytemuck::cast_slice(data));
    }

    
    /// Update uniforms on the GPU before drawing.
    pub fn write_uniform_buffers(
        &mut self,
    ) {
        // group 0: engine + camera
        self.write_buf(
            &self.uniforms.engine_uniforms_buffer,
            &[self.uniforms.engine_uniforms],
        );
        self.write_buf(
            &self.uniforms.camera_uniforms_buffer,
            &[self.uniforms.camera_uniforms],
        );

        // group 1: game
        self.write_buf(
            &self.uniforms.game_uniforms_buffer,
            &[self.uniforms.game_uniforms],
        );
        self.write_buf(
            &self.uniforms.obstacle_globals_buffer,
            &[self.uniforms.obstacle_globals],
        );
        self.write_buf(
            &self.uniforms.obstacle_buffer,
            &self.uniforms.obstacle_uniforms,
        );

    }

    pub fn render(&mut self, game: &Game, input: &InputHandler) -> Result<(), wgpu::SurfaceError> {
        self.update_engine_uniforms(input.get_mouse_x(), input.get_mouse_y());
        self.update_camera_uniforms(&game.camera);
        self.update_game_uniforms(game);
        self.update_obstacles(game.get_obstacles_all());

        self.write_uniform_buffers();


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
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
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
                                b: 0.5,
                                a: 1.0,
                            },
                        ),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniforms.engine_bind_group, &[]);
            render_pass.set_bind_group(1, &self.uniforms.obstacle_bind_group, &[]);
            render_pass.set_bind_group(2, &self.noise_texture_bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        self.render_frame += 1;
        Ok(())
    }
}

/// Holds all buffers, local copies of uniform data, and bind groups.
pub struct Uniforms {
    // -- Engine
    pub engine_uniforms: EngineUniforms,
    pub engine_uniforms_buffer: wgpu::Buffer,

    // -- Game
    pub game_uniforms: GameUniforms,
    pub game_uniforms_buffer: wgpu::Buffer,

    // -- Camera
    pub camera_uniforms: CameraUniforms,
    pub camera_uniforms_buffer: wgpu::Buffer,

    // -- Combined bind group for engine + camera
    pub engine_bind_group: wgpu::BindGroup,
    pub engine_group_layout: wgpu::BindGroupLayout,

    // -- Obstacles
    pub obstacle_uniforms: Vec<ObstacleUniform>,
    pub obstacle_buffer: wgpu::Buffer,
    pub obstacle_globals: ObstactleGlobalUniform,
    pub obstacle_globals_buffer: wgpu::Buffer,
    pub obstacle_bind_group_layout: wgpu::BindGroupLayout,
    pub obstacle_bind_group: wgpu::BindGroup,
}
