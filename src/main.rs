use std::mem::MaybeUninit;
use winit::window::{WindowBuilder, Window};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, WindowEvent};
use winit::dpi::PhysicalSize;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;

#[tokio::main]
async fn main() -> Result {
  tracing_subscriber::fmt::init();
  let event_loop = EventLoop::new();
  let window = WindowBuilder::new().build(&event_loop)?;
  let mut renderer = Renderer::new(&window).await?;
  event_loop.run(move |event, _, control_flow| match event {
    Event::WindowEvent { event, .. } => match event {
      WindowEvent::Resized(size) => renderer.resize(size),
      WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
      _ => {}
    },
    Event::RedrawRequested(..) => {
      let surface = renderer.surface.get_current_texture().unwrap();
      let surface_view = surface
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
      renderer.render(surface_view);
      surface.present();
    }
    Event::MainEventsCleared => window.request_redraw(),
    _ => {}
  });
}

struct Renderer {
  surface: wgpu::Surface,
  device: wgpu::Device,
  queue: wgpu::Queue,
  pipeline: wgpu::RenderPipeline,
  fb: MaybeUninit<Texture>,
  depth: MaybeUninit<Texture>,
}

impl Renderer {
  async fn new(window: &Window) -> Result<Self> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let surface = unsafe { instance.create_surface(&window)? };
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
      })
      .await
      .unwrap();
    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
          limits: wgpu::Limits::default(),
          label: None,
        },
        None,
      )
      .await?;

    let shader = device.create_shader_module(wgpu::include_spirv!(env!("shaders.spv")));
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      layout: None,
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: "main_v",
        buffers: &[],
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: "main_f",
        targets: &[Some(wgpu::ColorTargetState {
          format: FORMAT,
          blend: Some(wgpu::BlendState::REPLACE),
          write_mask: wgpu::ColorWrites::ALL,
        })],
      }),
      primitive: wgpu::PrimitiveState::default(),
      depth_stencil: Some(wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
      }),
      multisample: wgpu::MultisampleState {
        count: SAMPLES,
        mask: !0,
        alpha_to_coverage_enabled: false,
      },
      multiview: None,
      label: None,
    });

    let mut r = Renderer {
      surface,
      device,
      queue,
      pipeline,
      fb: MaybeUninit::uninit(),
      depth: MaybeUninit::uninit(),
    };
    r.resize(window.inner_size());
    Ok(r)
  }

  fn resize(&mut self, size: PhysicalSize<u32>) {
    self.surface.configure(
      &self.device,
      &wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: FORMAT,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
      },
    );
    // probably leaking old textures
    self.fb.write(self.create_tex(
      size.width,
      size.height,
      SAMPLES,
      FORMAT,
      wgpu::TextureUsages::RENDER_ATTACHMENT,
    ));
    self.depth.write(self.create_tex(
      size.width,
      size.height,
      SAMPLES,
      DEPTH_FORMAT,
      wgpu::TextureUsages::RENDER_ATTACHMENT,
    ));
  }

  fn render(&self, target: wgpu::TextureView) {
    let mut encoder = self
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &unsafe { self.fb.assume_init_ref() }.view,
        resolve_target: Some(&target),
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
          store: true,
        },
      })],
      depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
        view: &unsafe { self.depth.assume_init_ref() }.view,
        depth_ops: Some(wgpu::Operations {
          load: wgpu::LoadOp::Clear(1.0),
          store: true,
        }),
        stencil_ops: None,
      }),
      label: None,
    });
    render_pass.set_pipeline(&self.pipeline);
    render_pass.draw(0..3, 0..1);
    drop(render_pass);

    self.queue.submit([encoder.finish()]);
  }

  fn create_tex(
    &self,
    width: u32,
    height: u32,
    sample_count: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
  ) -> Texture {
    let texture = self.device.create_texture(&wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage,
      view_formats: &[],
      label: None,
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Texture { texture, view }
  }
}

struct Texture {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
}
