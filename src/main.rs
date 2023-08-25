use std::{slice, mem};
use std::fs::File;
use std::io::BufReader;
use wgpu::util::DeviceExt;
use winit::window::{WindowBuilder, Window};
use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, WindowEvent};
use winit::dpi::PhysicalSize;
use glam::{Vec3, Mat4};
use obj::{Obj, TexturedVertex};
use log::LevelFilter;
use shared::{Vertex, SceneConst, ObjConst};

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;

#[tokio::main]
async fn main() -> Result {
  env_logger::builder()
    .filter_level(LevelFilter::Info)
    .filter(Some("wgpu_core"), LevelFilter::Warn)
    .init();
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
      let mut encoder = renderer
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
      let surface = renderer.surface.get_current_texture().unwrap();
      let surface_view = surface
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

      renderer.render(&mut encoder, &surface.texture, &surface_view);

      renderer.queue.submit([encoder.finish()]);
      surface.present();
    }
    Event::MainEventsCleared => window.request_redraw(),
    _ => {}
  });
}

pub struct Renderer {
  surface: wgpu::Surface,
  device: wgpu::Device,
  queue: wgpu::Queue,
  pipeline: wgpu::RenderPipeline,
  textures: Textures,
}

impl Renderer {
  async fn new(window: &Window) -> Result<Self> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let surface = unsafe { instance.create_surface(&window)? };
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
      })
      .await
      .unwrap();
    let (device, queue) = adapter
      .request_device(
        &wgpu::DeviceDescriptor {
          features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES
            | wgpu::Features::PUSH_CONSTANTS,
          limits: wgpu::Limits {
            max_push_constant_size: 128,
            ..Default::default()
          },
          label: None,
        },
        None,
      )
      .await?;

    let shader = device.create_shader_module(wgpu::include_spirv!(env!("shaders.spv")));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: &[],
      push_constant_ranges: &[wgpu::PushConstantRange {
        stages: wgpu::ShaderStages::VERTEX,
        range: 0..(mem::size_of::<SceneConst>() + mem::size_of::<ObjConst>()) as _,
      }],
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: "main_v",
        buffers: &[wgpu::VertexBufferLayout {
          array_stride: mem::size_of::<Vertex>() as _,
          step_mode: wgpu::VertexStepMode::Vertex,
          attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3],
        }],
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

    let size = window.inner_size();
    let textures = Textures::new(&device, size);
    let mut r = Renderer {
      surface,
      device,
      queue,
      pipeline,
      textures,
    };
    r.resize(size);
    Ok(r)
  }

  fn resize(&mut self, size: PhysicalSize<u32>) {
    self.textures = Textures::new(&self.device, size);
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
  }

  fn render(
    &self,
    encoder: &mut wgpu::CommandEncoder,
    tex: &wgpu::Texture,
    view: &wgpu::TextureView,
  ) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &self.textures.fb.view,
        resolve_target: Some(view),
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
          store: true,
        },
      })],
      depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
        view: &self.textures.depth.view,
        depth_ops: Some(wgpu::Operations {
          load: wgpu::LoadOp::Clear(1.0),
          store: true,
        }),
        stencil_ops: None,
      }),
      label: None,
    });
    render_pass.set_pipeline(&self.pipeline);
    render_pass.set_push_constants(
      wgpu::ShaderStages::VERTEX,
      0,
      cast(&SceneConst {
        cam: Mat4::perspective_infinite_lh(1.4, tex.width() as f32 / tex.height() as f32, 0.01)
          * Mat4::look_at_lh(Vec3::splat(5.0), Vec3::ZERO, Vec3::Y),
      }),
    );
    render_pass.set_push_constants(
      wgpu::ShaderStages::VERTEX,
      mem::size_of::<SceneConst>() as _,
      cast(&ObjConst {
        transform: Mat4::from_rotation_y(0.5),
      }),
    );
    // unsafe { self.test.assume_init_ref() }.render(&mut render_pass);
  }
}

struct Textures {
  fb: Texture,
  depth: Texture,
}

impl Textures {
  fn new(device: &wgpu::Device, size: PhysicalSize<u32>) -> Self {
    Self {
      fb: Texture::new(
        device,
        size.width,
        size.height,
        SAMPLES,
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
      ),
      depth: Texture::new(
        device,
        size.width,
        size.height,
        SAMPLES,
        DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
      ),
    }
  }
}

struct Texture {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
}

impl Texture {
  fn new(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    sample_count: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
  ) -> Texture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
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

struct Mesh {
  vert_buf: wgpu::Buffer,
  idx_buf: wgpu::Buffer,
  len: u32,
}

impl Mesh {
  fn new(device: &wgpu::Device, verts: &[Vertex], indices: &[u32]) -> Self {
    Mesh {
      vert_buf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: cast_slice(verts),
        usage: wgpu::BufferUsages::VERTEX,
        label: None,
      }),
      idx_buf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: cast_slice(indices),
        usage: wgpu::BufferUsages::INDEX,
        label: None,
      }),
      len: indices.len() as _,
    }
  }

  fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
    render_pass.set_vertex_buffer(0, self.vert_buf.slice(..));
    render_pass.set_index_buffer(self.idx_buf.slice(..), wgpu::IndexFormat::Uint32);
    render_pass.draw_indexed(0..self.len, 0, 0..1);
  }
}

fn cast_slice<T>(t: &[T]) -> &[u8] {
  unsafe { slice::from_raw_parts(t.as_ptr() as _, mem::size_of_val(t)) }
}

fn cast<T>(t: &T) -> &[u8] {
  cast_slice(slice::from_ref(t))
}
