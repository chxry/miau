use std::mem;

use glam::{Vec3, Mat4};
use serde::{Serialize, Deserialize};
use shared::{Vertex, ObjConst, SceneConst};
use crate::Result;
use crate::ecs::{World, stage, component};
use crate::assets::{Assets, Handle};
use crate::gfx::{Renderer, Mesh, Texture, Shader, Frame, FORMAT, DEPTH_FORMAT, SAMPLES, cast};
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
    let renderer = Renderer::get();
    let shader = world
      .get_resource::<Assets>()
      .unwrap()
      .load::<Shader>("shaders.spv")?;
    let pipeline_layout = renderer
      .device
      .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&renderer.device.create_bind_group_layout(
          &wgpu::BindGroupLayoutDescriptor {
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
          },
        )],
        push_constant_ranges: &[wgpu::PushConstantRange {
          stages: wgpu::ShaderStages::VERTEX,
          range: 0..(mem::size_of::<SceneConst>() + mem::size_of::<ObjConst>()) as _,
        }],
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
            attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
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
    let frame = world.get_resource_mut::<Frame>().unwrap();
    let pipeline = world.get_resource::<StandardPass>().unwrap();
    let models = world.get::<Model>();
    let mut render_pass = frame
      .encoder
      .begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &frame.textures.fb,
          resolve_target: Some(&frame.surface_view),
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            // store: wgpu::StoreOp::Store,
            store: true,
          },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
          view: &frame.textures.depth,
          depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            // store: wgpu::StoreOp::Store,
            store: true,
          }),
          stencil_ops: None,
        }),
        // occlusion_query_set: None,
        // timestamp_writes: None,
        label: None,
      });
    render_pass.set_pipeline(&pipeline.0);
    render_pass.set_push_constants(wgpu::ShaderStages::VERTEX, 0, unsafe {
      cast(&SceneConst {
        cam: Mat4::perspective_infinite_lh(
          1.4,
          frame.surface.texture.width() as f32 / frame.surface.texture.height() as f32,
          0.01,
        ) * Mat4::look_at_lh(Vec3::splat(5.0), Vec3::ZERO, Vec3::Y),
      })
    });

    for (e, model) in &models {
      if let Some(t) = e.get_one_mut::<Transform>() {
        render_pass.set_push_constants(
          wgpu::ShaderStages::VERTEX,
          mem::size_of::<SceneConst>() as _,
          unsafe {
            cast(&ObjConst {
              transform: t.as_mat4(),
            })
          },
        );
        model.tex.bind(&mut render_pass);
        model.mesh.render(&mut render_pass, 1);
      }
    }
    Ok(())
  }
}
