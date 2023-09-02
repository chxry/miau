use std::{slice, mem};
use std::sync::OnceLock;
use wgpu::util::DeviceExt;
use winit::window::Window;
use winit::dpi::PhysicalSize;
use glam::{Vec3, Mat4};
use crate::ecs::World;
use crate::{Result, Transform};

pub use shared::{Vertex, SceneConst, ObjConst};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;

static mut RENDERER: OnceLock<Renderer> = OnceLock::new();

pub fn init(window: &Window) {
  unsafe {
    let _ = RENDERER.set(pollster::block_on(Renderer::new(window)).unwrap());
  }
}

pub fn renderer() -> &'static mut Renderer {
  unsafe { RENDERER.get_mut().unwrap() }
}

pub struct Renderer {
  pub surface: wgpu::Surface,
  pub device: wgpu::Device,
  pub queue: wgpu::Queue,
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
          limits: adapter.limits(),
          label: None,
        },
        None,
      )
      .await?;

    Ok(Self {
      surface,
      device,
      queue,
    })
  }

  pub fn resize(&mut self, size: PhysicalSize<u32>) {
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
}

pub struct Pipeline {
  pipeline: wgpu::RenderPipeline,
  textures: Textures,
  test2: TextureSampler,
}

impl Pipeline {
  pub fn new(size: PhysicalSize<u32>) -> Result<Self> {
    let device = &renderer().device;
    let shader = device.create_shader_module(wgpu::include_spirv!(env!("shaders.spv")));
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: None,
      bind_group_layouts: &[
        &device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          entries: &[
            wgpu::BindGroupLayoutEntry {
              binding: 0,
              visibility: wgpu::ShaderStages::FRAGMENT,
              ty: wgpu::BindingType::Texture {
                multisampled: false,
                view_dimension: wgpu::TextureViewDimension::D2,
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
              },
              count: None,
            },
            wgpu::BindGroupLayoutEntry {
              binding: 1,
              visibility: wgpu::ShaderStages::FRAGMENT,
              ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
              count: None,
            },
          ],
          label: None,
        }),
      ],
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
          attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
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

    let img = image::load_from_memory(include_bytes!("../garfield.png"))?;
    let test2 = Texture::new(
      img.width(),
      img.height(),
      1,
      wgpu::TextureFormat::Rgba8UnormSrgb,
      wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    )
    .create_sampler(&pipeline.get_bind_group_layout(0));
    test2.tex.write(&img.to_rgba8());

    Ok(Self {
      pipeline,
      textures: Textures::new(size),
      test2,
    })
  }

  pub fn resize(&mut self, size: PhysicalSize<u32>) {
    self.textures = Textures::new(size);
  }

  pub fn frame(&mut self, world: &World) {
    let renderer = renderer();
    let mut encoder = renderer
      .device
      .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let surface = renderer.surface.get_current_texture().unwrap();
    let surface_view = surface
      .texture
      .create_view(&wgpu::TextureViewDescriptor::default());

    {
      let meshes = world.get::<std::rc::Rc<Mesh>>();
      let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &self.textures.fb.view,
          resolve_target: Some(&surface_view),
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
          cam: Mat4::perspective_infinite_lh(
            1.4,
            surface.texture.width() as f32 / surface.texture.height() as f32,
            0.01,
          ) * Mat4::look_at_lh(Vec3::splat(5.0), Vec3::ZERO, Vec3::Y),
        }),
      );
      self.test2.bind(&mut render_pass);
      for (e, mesh) in &meshes {
        if let Some(t) = e.get_one::<Transform>() {
          render_pass.set_push_constants(
            wgpu::ShaderStages::VERTEX,
            mem::size_of::<SceneConst>() as _,
            cast(&ObjConst {
              transform: t.as_mat4(),
            }),
          );
          mesh.render(&mut render_pass);
        }
      }
    }

    renderer.queue.submit([encoder.finish()]);
    surface.present();
  }
}

struct Textures {
  fb: Texture,
  depth: Texture,
}

impl Textures {
  fn new(size: PhysicalSize<u32>) -> Self {
    Self {
      fb: Texture::new(
        size.width,
        size.height,
        SAMPLES,
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
      ),
      depth: Texture::new(
        size.width,
        size.height,
        SAMPLES,
        DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
      ),
    }
  }
}

pub struct Texture {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
}

impl Texture {
  pub fn new(
    width: u32,
    height: u32,
    sample_count: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
  ) -> Self {
    let texture = renderer().device.create_texture(&wgpu::TextureDescriptor {
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
    Self { texture, view }
  }

  pub fn create_sampler(self, layout: &wgpu::BindGroupLayout) -> TextureSampler {
    TextureSampler::from(self, layout)
  }

  pub fn write(&self, data: &[u8]) {
    renderer().queue.write_texture(
      wgpu::ImageCopyTexture {
        texture: &self.texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      data,
      wgpu::ImageDataLayout {
        offset: 0,
        bytes_per_row: Some(4 * self.texture.width()),
        rows_per_image: Some(self.texture.height()),
      },
      self.texture.size(),
    );
  }
}

pub struct TextureSampler {
  tex: Texture,
  sampler: wgpu::Sampler,
  bind_group: wgpu::BindGroup,
}

impl TextureSampler {
  fn from(tex: Texture, layout: &wgpu::BindGroupLayout) -> Self {
    let device = &renderer().device;
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: wgpu::BindingResource::TextureView(&tex.view),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::Sampler(&sampler),
        },
      ],
      label: None,
    });
    Self {
      tex,
      sampler,
      bind_group,
    }
  }

  fn bind<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
    render_pass.set_bind_group(0, &self.bind_group, &[]);
  }
}

pub struct Mesh {
  vert_buf: wgpu::Buffer,
  idx_buf: wgpu::Buffer,
  len: u32,
}

impl Mesh {
  pub fn new(verts: &[Vertex], indices: &[u32]) -> Self {
    let device = &renderer().device;
    Self {
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
