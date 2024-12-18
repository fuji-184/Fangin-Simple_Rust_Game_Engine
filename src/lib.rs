mod renderer_backend;

use renderer_backend::*;

use anyhow::{Context, Result};
use tracing::{error, info};
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{
        ActiveEventLoop,
        ControlFlow,
        EventLoop
    },
    window::{
        Window,
        WindowId,
        WindowAttributes
    },
    application::ApplicationHandler,
    dpi::PhysicalSize
};
use tokio::runtime::Runtime;
use std::sync::Arc;

pub struct GraphicState<'lifetime_1> {
    surface: wgpu::Surface<'lifetime_1>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    triangle_mesh: wgpu::Buffer,
    quad_mesh: mesh_builder::Mesh,
    triangle_material: materials::Material,
    quad_material: materials::Material
}

impl<'lifetime_1> GraphicState<'lifetime_1> {
    pub fn new(window: Arc<Window>) -> GraphicState<'lifetime_1> {
    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {

        let size = Arc::clone(&window).inner_size();

        let instance_descriptor = wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(), ..Default::default()
        };
        let instance = wgpu::Instance::new(instance_descriptor);

        let surface = instance.create_surface(Arc::clone(&window)).expect("Failed to create surface in wgpu");

        let adapter_descriptor = wgpu::RequestAdapterOptionsBase {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false
        };
        let adapter = instance.request_adapter(&adapter_descriptor).await.unwrap();

        let device_descriptor = wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            label: Some("Device"),
            memory_hints: wgpu::MemoryHints::default()
        };
        let (device, queue) = adapter
                .request_device(&device_descriptor, None)
                .await
                .unwrap();

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_capabilities.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_capabilities.present_modes[0],
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2
        };
        surface.configure(&device, &config);

        let triangle_mesh = mesh_builder::make_triangle(&device);
        let quad_mesh = mesh_builder::make_quad(&device);

        let material_bind_group_layout: wgpu::BindGroupLayout;
        {
            let mut builder = bind_group_layout::Builder::new(&device);
            builder.add_material();
            material_bind_group_layout = builder.build("Material bind group layout");
        }

        let render_pipeline: wgpu::RenderPipeline;
        {
            let mut builder = pipeline::Builder::new(&device);
            builder.add_vertex_buffer_layout(mesh_builder::Vertex::get_layout());
            builder.set_shader_module("shader.wgsl", "vs_main", "fs_main");
            builder.set_pixel_format(config.format);
            builder.add_bind_group_layout(&material_bind_group_layout);
            render_pipeline = builder.build_pipeline("Render pipelne");
        }

        let quad_material = materials::Material::new("gambar.png", &device, &queue, &material_bind_group_layout);
        let triangle_material = materials::Material::new("gambar.png", &device, &queue, &material_bind_group_layout);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            triangle_mesh,
            quad_mesh,
            triangle_material,
            quad_material
        }
    })
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self) {
        let drawable = self.surface.get_current_texture().unwrap();
        let image_view_descriptor = wgpu::TextureViewDescriptor::default();
        let image_view = drawable.texture.create_view(&image_view_descriptor);

        let command_encoder_descriptor = wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder")
        };
        let mut command_encoder = self.device.create_command_encoder(&command_encoder_descriptor);

        let color_attachment = wgpu::RenderPassColorAttachment {
            view: &image_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.0,
                    g: 0.0,
                    b: 1.0,
                    a: 1.0
                }),
                store: wgpu::StoreOp::Store
            }
        };

        let render_pass_descriptor = wgpu::RenderPassDescriptor {
            label: Some("Some Pass"),
            color_attachments: &[Some(color_attachment)],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None
        };

        {
            let mut render_pass = command_encoder.begin_render_pass(&render_pass_descriptor);
            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_vertex_buffer(0, self.quad_mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.quad_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_bind_group(0, &self.quad_material.bind_group, &[]);
            render_pass.draw_indexed(0..6, 0, 0..1);

            render_pass.set_bind_group(0, &self.triangle_material.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.triangle_mesh.slice(..));
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(command_encoder.finish()));

        drawable.present();

    }
}


#[derive(Default)]
pub struct App<'lifetime_1> {
    window: Option<Arc<Window>>,
    graphic_state: Option<GraphicState<'lifetime_1>>
}

impl ApplicationHandler for App<'_> {

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(event_loop.create_window(Window::default_attributes().with_title("Fuji Engine")).expect("Failed to create window in resumed"));
        self.window = Some(window.clone());

        let graphic_state = GraphicState::new(window.clone());
        self.graphic_state = Some(graphic_state);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("The close button was pressed");
                event_loop.exit();
            },

            WindowEvent::RedrawRequested => {
                self.window.as_ref().expect("Failed to get window in request redraw").request_redraw();

                self.graphic_state.as_mut().unwrap().render();
            },

            WindowEvent::Resized(physical_size) => self.graphic_state.as_mut().unwrap().resize(physical_size),

            _ => ()
        }

        //Ok(())
    }
}

pub fn run() -> Result<()> {
    let event_loop = EventLoop::builder().build().with_context(|| "Failed to create event loop")?;

    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::default();

    event_loop.run_app(&mut app);

    Ok(())
}

pub fn run_async() -> Result<()> {
    let runtime = Runtime::new().with_context(|| "Failed to create Tokio runtime")?;
    runtime.block_on(async {
        run()?;
        Ok(())
    })
}
