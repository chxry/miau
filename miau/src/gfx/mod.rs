pub mod standard;

use std::{slice, mem};
use wgpu::util::DeviceExt;
use winit::window::Window;
use winit::dpi::PhysicalSize;
use glam::Vec2;
use obj::{Obj, TexturedVertex};
use standard::StandardPass;
use crate::ecs::{World, stage};
use crate::assets::asset;
use crate::{Result, world};

pub use shared::{Vertex, SceneConst, ObjConst};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SAMPLES: u32 = 4;

pub struct Renderer {
  surface: wgpu::Surface,
  device: wgpu::Device,
  queue: wgpu::Queue,
  textures: Box<Textures>,
}

impl Renderer {
  pub async fn init(window: &Window, world: &World) -> Result {
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
    let textures = Box::new(Textures::new(&device, window.inner_size()));
    world.add_resource(Self {
      surface,
      device,
      queue,
      textures,
    });

    world.add_resource(StandardPass::new(world)?);
    Ok(())
  }

  pub fn resize(&mut self, size: PhysicalSize<u32>) {
    self.textures = Box::new(Textures::new(&self.device, size));
    self.surface.configure(
      &self.device,
      &wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: FORMAT,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::AutoVsync,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
      },
    );
  }

  pub fn frame(&mut self, world: &World) {
    let encoder = Box::new(
      self
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default()),
    );
    let surface = self.surface.get_current_texture().unwrap();
    let surface_view = surface
      .texture
      .create_view(&wgpu::TextureViewDescriptor::default());

    world.add_resource(Frame {
      surface,
      surface_view,
      textures: Box::leak(unsafe { mem::transmute_copy::<_, Box<Textures>>(&self.textures) }),
      encoder: Box::leak(encoder),
    });
    world.run_system(stage::DRAW);

    let frame = world.take_resource::<Frame>().unwrap();
    self
      .queue
      .submit([unsafe { Box::from_raw(frame.encoder) }.finish()]);
    frame.surface.present();
  }

  fn get() -> &'static Self {
    world().get_resource().unwrap()
  }
}

struct Textures {
  fb: wgpu::TextureView,
  depth: wgpu::TextureView,
}

impl Textures {
  fn new(device: &wgpu::Device, size: PhysicalSize<u32>) -> Self {
    let desc = wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width: size.width,
        height: size.height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: SAMPLES,
      dimension: wgpu::TextureDimension::D2,
      format: FORMAT,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      view_formats: &[],
      label: None,
    };
    Self {
      fb: device
        .create_texture(&desc)
        .create_view(&wgpu::TextureViewDescriptor::default()),
      depth: device
        .create_texture(&wgpu::TextureDescriptor {
          format: DEPTH_FORMAT,
          ..desc
        })
        .create_view(&wgpu::TextureViewDescriptor::default()),
    }
  }
}

pub struct Frame<'a> {
  surface: wgpu::SurfaceTexture,
  surface_view: wgpu::TextureView,
  textures: &'a Textures,
  encoder: &'a mut wgpu::CommandEncoder,
}

#[asset(Texture::load)]
pub struct Texture {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
  sampler: wgpu::Sampler,
  bind_group: wgpu::BindGroup,
}

impl Texture {
  pub fn new(width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
    let renderer = Renderer::get();
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
      size: wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
      view_formats: &[],
      label: None,
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = renderer
      .device
      .create_sampler(&wgpu::SamplerDescriptor::default());
    let bind_group = renderer
      .device
      .create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &world()
          .get_resource::<StandardPass>()
          .unwrap()
          .0
          .get_bind_group_layout(0),
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&view),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&sampler),
          },
        ],
        label: None,
      });
    Self {
      texture,
      view,
      sampler,
      bind_group,
    }
  }

  fn load(data: &[u8]) -> Result<Self> {
    let img = image::load_from_memory(data)?;
    let tex = Texture::new(
      img.width(),
      img.height(),
      wgpu::TextureFormat::Rgba8UnormSrgb,
    );
    tex.write(&img.to_rgba8());
    Ok(tex)
  }

  pub fn write(&self, data: &[u8]) {
    Renderer::get().queue.write_texture(
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

  fn bind<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
    render_pass.set_bind_group(0, &self.bind_group, &[]);
  }
}

#[asset(Mesh::load)]
pub struct Mesh {
  vert_buf: wgpu::Buffer,
  idx_buf: wgpu::Buffer,
  len: u32,
}

impl Mesh {
  pub fn new(verts: &[Vertex], indices: &[u32]) -> Self {
    let device = &Renderer::get().device;
    Self {
      vert_buf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: unsafe { cast_slice(verts) },
        usage: wgpu::BufferUsages::VERTEX,
        label: None,
      }),
      idx_buf: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        contents: unsafe { cast_slice(indices) },
        usage: wgpu::BufferUsages::INDEX,
        label: None,
      }),
      len: indices.len() as _,
    }
  }

  fn load(data: &[u8]) -> Result<Self> {
    let obj: Obj<TexturedVertex, u32> = obj::load_obj(data)?;
    Ok(Self::new(
      &obj
        .vertices
        .iter()
        .map(|v| Vertex {
          pos: v.position.into(),
          uv: Vec2::new(v.texture[0], 1.0 - v.texture[1]),
        })
        .collect::<Vec<_>>(),
      &obj.indices,
    ))
  }

  fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, instances: u32) {
    render_pass.set_vertex_buffer(0, self.vert_buf.slice(..));
    render_pass.set_index_buffer(self.idx_buf.slice(..), wgpu::IndexFormat::Uint32);
    render_pass.draw_indexed(0..self.len, 0, 0..instances);
  }
}

#[asset(Shader::load)]
pub struct Shader(wgpu::ShaderModule);

impl Shader {
  fn load(data: &[u8]) -> Result<Self> {
    Ok(Self(Renderer::get().device.create_shader_module(
      wgpu::ShaderModuleDescriptor {
        source: wgpu::util::make_spirv(data),
        label: None,
      },
    )))
  }
}

unsafe fn cast_slice<T>(t: &[T]) -> &[u8] {
  slice::from_raw_parts(t.as_ptr() as _, mem::size_of_val(t))
}

unsafe fn cast<T>(t: &T) -> &[u8] {
  cast_slice(slice::from_ref(t))
}
