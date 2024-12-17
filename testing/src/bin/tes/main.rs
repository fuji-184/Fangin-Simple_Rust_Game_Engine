use anyhow::{Context, Result};
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoopBuilder;
use winit::window::WindowBuilder;
use winit::platform::windows::WindowBuilderExtWindows;
use winit::platform::unix::WindowBuilderExtUnix;
use std::num::NonZeroU32;
use std::ffi::OsStr;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration, RenderPipeline, BindGroup};
use winit::dpi::Size;
use winit::window::WindowBuilderAttributes;
use std::borrow::Cow;
use winit::platform::windows::WindowBuilderExtWindows;
use winit::platform::unix::WindowBuilderExtUnix;
use std::num::NonZeroU32;

#[derive(Default)]
struct App {
    window: Option<Window>,
    surface: Option<Surface>,
    device: Option<Device>,
    queue: Option<Queue>,
    config: Option<SurfaceConfiguration>,
    pipeline: Option<RenderPipeline>,
    bind_group: Option<BindGroup>,
}

impl App {
    fn new(event_loop: &EventLoop<()>, window: Window) -> Self {
        let size = window.inner_size();
        let (device, queue) = pollster::block_on(async {
            let instance = wgpu::Instance::new(wgpu::Backends::all());
            let surface = unsafe { instance.create_surface(&window) };
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::default(),
                    compatible_surface: Some(&surface),
                    ..Default::default()
                })
                .await
                .expect("No suitable GPU found");

            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::default(),
                    label: None,
                })
                .await
                .expect("Failed to create device");

            (device, queue)
        });

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        surface.configure(&device, &config);

        let vs_src = include_str!("shaders/vertex_shader.wgsl");
        let fs_src = include_str!("shaders/fragment_shader.wgsl");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(fs_src)),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[],
            label: Some("bind_group_layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
            label: Some("pipeline_layout"),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "main",
                targets: &[Some(config.format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("render_pipeline"),
        });

        let text = "hello";
        let vertices = text.chars().flat_map(|c| {
            let x = c as f32 * 20.0;
            let y = 50.0;
            vec![x, y]
        }).collect::<Vec<f32>>();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex_buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[],
            label: Some("bind_group"),
        });

        App {
            window: Some(window),
            surface: Some(surface),
            device: Some(device),
            queue: Some(queue),
            config: Some(config),
            pipeline: Some(pipeline),
            bind_group: Some(bind_group),
        }
    }

    fn render(&mut self) {
        let frame = self.surface.as_ref().expect("No surface available").get_current_texture().expect("Failed to acquire next surface texture");
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.as_ref().expect("No device available").create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("command_encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(self.pipeline.as_ref().expect("No pipeline available"));
            render_pass.set_bind_group(0, self.bind_group.as_ref().expect("No bind group available"), &[]);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.as_ref().expect("No queue available").submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    fn run(self) -> Result<()> {
        let event_loop = EventLoopBuilder::new().with_event_loop(&self.window).with_context(|| "Failed to create event loop")?;
        let mut app = App::new(&event_loop, self.window.unwrap());

        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { event, window_id } if window_id == app.window.unwrap().id() => {
                    app.window_event(&event, window_id, event);
                }
                Event::RedrawRequested(window_id) if window_id == app.window.unwrap().id() => {
                    app.render();
                }
                Event::MainEventsCleared => {
                    app.window.as_ref().unwrap().request_redraw();
                }
                _ => *control_flow = ControlFlow::Wait,
            }
        })
    }
}

fn main() -> Result<()> {
    App::default().run()
}

