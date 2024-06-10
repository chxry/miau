use std::mem;
use miau::Result;
use miau::ecs::{World, stage, component};
use miau::assets::{Assets, Handle};
use miau::math::Mat4;
use miau::gfx::{
  Renderer, Mesh, Shader, Frame, Vertex, Binding, Bindable, SceneConst, FORMAT, DEPTH_FORMAT,
  SAMPLES, cast,
};
use miau::scene::Transform;
use serde::{Serialize, Deserialize};
// use game_shared::FurConst;

pub struct FurPass {
  pipeline: wgpu::RenderPipeline,
  furconst_layout: wgpu::BindGroupLayout,
}

impl FurPass {
  pub fn new(world: &World) -> Result<Self> {
    let renderer = world.get_resource::<Renderer>().unwrap();
    let shader = world
      .get_resource::<Assets>()
      .unwrap()
      .load::<Shader>("game_shaders.spv")?;
    let furconst_layout =
      renderer
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
          entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
              ty: wgpu::BufferBindingType::Uniform,
              has_dynamic_offset: false,
              min_binding_size: None,
            },
            count: None,
          }],
          label: None,
        });

    let pipeline_layout = renderer
      .device
      .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&renderer.scene_layout, &furconst_layout],
        push_constant_ranges: &[wgpu::PushConstantRange {
          stages: wgpu::ShaderStages::VERTEX,
          range: 0..mem::size_of::<Mat4>() as _,
        }],
        label: None,
      });
    let pipeline = renderer
      .device
      .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
          module: &shader.0,
          entry_point: "main_v",
          buffers: &[wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as _,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2=> Float32x3],
          }],
        },
        fragment: Some(wgpu::FragmentState {
          module: &shader.0,
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
    world.add_system(stage::DRAW, Self::pass);
    Ok(Self {
      pipeline,
      furconst_layout,
    })
  }

  fn pass(world: &World) -> Result {
    let renderer = world.get_resource_mut::<Renderer>().unwrap();
    let frame = world.get_resource_mut::<Frame>().unwrap();
    let pipeline = world.get_resource::<FurPass>().unwrap();
    let mut models = world.get_mut::<FurModel>();
    let mut render_pass = frame
      .encoder
      .begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &renderer.textures.fb,
          resolve_target: Some(&frame.surface_view),
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
          view: &renderer.textures.depth,
          depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
          }),
          stencil_ops: None,
        }),
        occlusion_query_set: None,
        timestamp_writes: None,
        label: None,
      });
    render_pass.set_pipeline(&pipeline.pipeline);
    world
      .get_resource::<Binding<SceneConst>>()
      .unwrap()
      .bind(&mut render_pass, 0);

    for (e, model) in &mut models {
      if let Some(t) = e.get_one_mut::<Transform>() {
        render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, cast(&t.as_mat4()));
        model.consts.update(&renderer.queue);
        model.consts.bind(&mut render_pass, 1);
        model
          .mesh
          .render(&mut render_pass, model.consts.data().layers);
      }
    }
    Ok(())
  }
}

#[component]
#[derive(Serialize, Deserialize)]
pub struct FurModel {
  pub mesh: Handle<Mesh>,
  pub consts: Binding<FurConst>,
}

impl FurModel {
  pub fn new(world: &World, mesh: Handle<Mesh>) -> Self {
    Self {
      mesh,
      consts: Binding::new(FurConst {
        layers: 50,
        density: 1000.0,
        height: 0.25,
        thickness: 2.5,
      }),
    }
  }

  pub fn layers(mut self, layers: u32) -> Self {
    self.consts.data_mut().layers = layers;
    self
  }

  pub fn density(mut self, density: f32) -> Self {
    self.consts.data_mut().density = density;
    self
  }

  pub fn height(mut self, height: f32) -> Self {
    self.consts.data_mut().height = height;
    self
  }

  pub fn thickness(mut self, thickness: f32) -> Self {
    self.consts.data_mut().thickness = thickness;
    self
  }
}

// dont duplicate
#[repr(C)]
#[derive(Serialize, Deserialize)]
pub struct FurConst {
  pub layers: u32,
  pub density: f32,
  pub height: f32,
  pub thickness: f32,
}

impl Bindable for FurConst {
  fn get_layout(world: &World) -> &wgpu::BindGroupLayout {
    &world.get_resource::<FurPass>().unwrap().furconst_layout
  }
}
