use std::mem;
use glam::Mat4;
use serde::{Serialize, Deserialize};
use crate::Result;
use crate::ecs::{World, stage, component};
use crate::assets::{Assets, Handle};
use crate::gfx::{
  Renderer, Mesh, Texture, Shader, Frame, Vertex, Binding, SceneConst, FORMAT, DEPTH_FORMAT,
  SAMPLES, cast,
};
use crate::scene::Transform;

#[component]
#[derive(Serialize, Deserialize)]
pub struct Model {
  pub mesh: Handle<Mesh>,
  pub tex: Handle<Texture>,
}

pub struct StandardPass(pub wgpu::RenderPipeline);

impl StandardPass {
  pub fn new(world: &World) -> Result<Self> {
    let renderer = world.get_resource::<Renderer>().unwrap();
    let shader = world
      .get_resource::<Assets>()
      .unwrap()
      .load::<Shader>("miau_shaders.spv")?;
    let pipeline_layout = renderer
      .device
      .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[&renderer.scene_layout, &renderer.tex_layout],
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
    Ok(Self(pipeline))
  }

  fn pass(world: &World) -> Result {
    let renderer = world.get_resource::<Renderer>().unwrap();
    let frame = world.get_resource_mut::<Frame>().unwrap();
    let pipeline = world.get_resource::<StandardPass>().unwrap();
    let models = world.get::<Model>();
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
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
          }),
          stencil_ops: None,
        }),
        occlusion_query_set: None,
        timestamp_writes: None,
        label: None,
      });
    render_pass.set_pipeline(&pipeline.0);
    world
      .get_resource::<Binding<SceneConst>>()
      .unwrap()
      .bind(&mut render_pass, 0);
    for (e, model) in &models {
      if let Some(t) = e.get_one_mut::<Transform>() {
        render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, cast(&t.as_mat4()));
        model.tex.bind(&mut render_pass);
        model.mesh.render(&mut render_pass, 1);
      }
    }
    Ok(())
  }
}
