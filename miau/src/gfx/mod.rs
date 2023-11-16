pub mod standard;

use std::{slice, mem};
use wgpu::util::DeviceExt;
use winit::window::Window;
use winit::dpi::PhysicalSize;
use glam::{Vec2, Vec3, Mat4};
use obj::{Obj, TexturedVertex};
use standard::StandardPass;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use crate::ecs::{World, stage};
use crate::assets::asset;
use crate::{Result, world};

pub use miau_shared::*;

// move to renderer
pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const SAMPLES: u32 = 4;

pub struct Renderer {
  pub surface: wgpu::Surface,
  pub device: wgpu::Device,
  pub queue: wgpu::Queue,
  pub textures: Box<Textures>,
  pub scene_layout: wgpu::BindGroupLayout,
  pub scene_buf: wgpu::Buffer,
  pub scene_bind_group: wgpu::BindGroup,
  pub tex_layout: wgpu::BindGroupLayout,
}

impl Renderer {
  pub async fn init(world: &World) -> Result {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let window = world.get_resource::<Window>().unwrap();
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

    let scene_buf = device.create_buffer(&wgpu::BufferDescriptor {
      size: mem::size_of::<SceneConst>() as _,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
      label: None,
    });
    let scene_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      }],
      label: None,
    });
    let scene_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &scene_layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: scene_buf.as_entire_binding(),
      }],
      label: None,
    });
    let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    });

    world.add_resource(Self {
      surface,
      device,
      queue,
      textures,
      scene_layout,
      scene_buf,
      scene_bind_group,
      tex_layout,
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
    self.queue.write_buffer(
      &self.scene_buf,
      0,
      cast(&SceneConst {
        cam: Mat4::perspective_infinite_lh(
          1.4,
          surface.texture.width() as f32 / surface.texture.height() as f32,
          0.01,
        ) * Mat4::look_at_lh(Vec3::splat(5.0), Vec3::ZERO, Vec3::Y),
      }),
    );

    world.add_resource(Frame {
      surface,
      surface_view,
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

pub struct Textures {
  pub fb: wgpu::TextureView,
  pub depth: wgpu::TextureView,
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
  pub surface: wgpu::SurfaceTexture,
  pub surface_view: wgpu::TextureView,
  pub encoder: &'a mut wgpu::CommandEncoder,
}

#[asset(Texture::load)]
pub struct Texture {
  pub texture: wgpu::Texture,
  pub view: wgpu::TextureView,
  pub sampler: wgpu::Sampler,
  pub bind_group: wgpu::BindGroup,
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
        layout: &renderer.tex_layout,
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

  pub fn bind<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
    render_pass.set_bind_group(1, &self.bind_group, &[]);
  }
}

#[asset(Mesh::load)]
pub struct Mesh {
  pub vert_buf: wgpu::Buffer,
  pub idx_buf: wgpu::Buffer,
  pub len: u32,
}

impl Mesh {
  pub fn new(verts: &[Vertex], indices: &[u32]) -> Self {
    let device = &Renderer::get().device;
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

  fn load(data: &[u8]) -> Result<Self> {
    let obj: Obj<TexturedVertex, u32> = obj::load_obj(data)?;
    Ok(Self::new(
      &obj
        .vertices
        .iter()
        .map(|v| Vertex {
          pos: v.position.into(),
          uv: Vec2::new(v.texture[0], 1.0 - v.texture[1]),
          normal: v.normal.into(),
        })
        .collect::<Vec<_>>(),
      &obj.indices,
    ))
  }

  pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, instances: u32) {
    render_pass.set_vertex_buffer(0, self.vert_buf.slice(..));
    render_pass.set_index_buffer(self.idx_buf.slice(..), wgpu::IndexFormat::Uint32);
    render_pass.draw_indexed(0..self.len, 0, 0..instances);
  }
}

#[asset(Shader::load)]
pub struct Shader(pub wgpu::ShaderModule);

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

pub trait Bindable {
  fn get_layout(world: &World) -> wgpu::BindGroupLayout;
}

pub struct Binding<T: Bindable> {
  data: T,
  dirty: bool,
  buf: wgpu::Buffer,
  bind_group: wgpu::BindGroup,
}

impl<T: Bindable> Binding<T> {
  pub fn new(data: T) -> Self {
    let device = &Renderer::get().device;
    let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      contents: cast(&data),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      label: None,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      layout: &T::get_layout(world()),
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: buf.as_entire_binding(),
      }],
      label: None,
    });
    Self {
      data,
      dirty: false,
      buf,
      bind_group,
    }
  }

  pub fn data(&self) -> &T {
    &self.data
  }

  pub fn data_mut(&mut self) -> &mut T {
    self.dirty = true;
    &mut self.data
  }

  pub fn update(&mut self, queue: &wgpu::Queue) {
    if self.dirty {
      queue.write_buffer(&self.buf, 0, cast(&self.data));
      self.dirty = false;
    }
  }

  pub fn bind<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, n: u32) {
    render_pass.set_bind_group(n, &self.bind_group, &[]);
  }
}

impl<T: Bindable + Serialize> Serialize for Binding<T> {
  fn serialize<S: Serializer>(&self, se: S) -> Result<S::Ok, S::Error> {
    self.data.serialize(se)
  }
}

impl<'de, T: Bindable + Deserialize<'de>> Deserialize<'de> for Binding<T> {
  fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
    T::deserialize(de).map(|d| Binding::new(d))
  }
}

pub fn cast_slice<T>(t: &[T]) -> &[u8] {
  unsafe { slice::from_raw_parts(t.as_ptr() as _, mem::size_of_val(t)) }
}

pub fn cast<T>(t: &T) -> &[u8] {
  cast_slice(slice::from_ref(t))
}
