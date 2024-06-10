use std::mem;
use winit::window::{Window, CursorIcon};
use winit::event::{WindowEvent, ElementState, MouseButton, MouseScrollDelta, TouchPhase};
use winit::keyboard::{PhysicalKey, KeyCode};
use winit::dpi::PhysicalPosition;
use wgpu::util::DeviceExt;
use imgui::{
  Context, Textures, DrawVert, DrawCmd, BackendFlags, ConfigFlags, Ui, FontSource, Key, MouseCursor,
};
use log::info;
use crate::Result;
use crate::ecs::{World, stage};
use crate::gfx::{Renderer, Shader, Texture, Frame, Binding, SceneConst, DeltaTime, FORMAT, cast_slice};
use crate::assets::Assets;

pub use imgui;

pub struct UiPass {
  pipeline: wgpu::RenderPipeline,
  ctx: Context,
  textures: Textures<Texture>,
  vert_buf: wgpu::Buffer,
  idx_buf: wgpu::Buffer,
}

impl UiPass {
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
        push_constant_ranges: &[],
        label: None,
      });
    let pipeline = renderer
      .device
      .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
          module: &shader.0,
          entry_point: "ui_v",
          buffers: &[wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<DrawVert>() as _,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Unorm8x4],
          }],
        },
        fragment: Some(wgpu::FragmentState {
          module: &shader.0,
          entry_point: "ui_f",
          targets: &[Some(wgpu::ColorTargetState {
            format: FORMAT,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
          })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        label: None,
      });
    let mut ctx = Context::create();
    let mut textures = Textures::new();
    ctx.set_ini_filename(None);
    let io = ctx.io_mut();
    let window = world.get_resource::<Window>().unwrap();
    let size = window.inner_size();
    let scale = window.scale_factor();
    io.display_size = [size.width as _, size.height as _];
    io.display_framebuffer_scale = [scale as _; 2];
    io.backend_flags |= BackendFlags::HAS_MOUSE_CURSORS;
    io.backend_flags |= BackendFlags::HAS_SET_MOUSE_POS;
    io.backend_flags |= BackendFlags::RENDERER_HAS_VTX_OFFSET;
    io.config_flags |= ConfigFlags::NAV_ENABLE_KEYBOARD;
    io.config_flags |= ConfigFlags::NAV_ENABLE_SET_MOUSE_POS;

    let style = ctx.style_mut();
    style.window_rounding = 4.0;
    style.popup_rounding = 4.0;
    style.frame_rounding = 2.0;

    let fonts = ctx.fonts();
    let assets = world.get_resource::<Assets>().unwrap();
    fonts.add_font(&[FontSource::TtfData {
      data: &assets.load_raw("roboto.ttf")?,
      size_pixels: 13.0 * scale as f32,
      config: None,
    }]);
    let font_tex = fonts.build_rgba32_texture();
    fonts.tex_id = textures.insert(Texture::init(
      font_tex.width,
      font_tex.height,
      wgpu::TextureFormat::Rgba8UnormSrgb,
      font_tex.data,
    ));

    let vert_buf = renderer.device.create_buffer(&wgpu::BufferDescriptor {
      size: 0,
      usage: wgpu::BufferUsages::VERTEX,
      mapped_at_creation: false,
      label: None,
    });
    let idx_buf = renderer.device.create_buffer(&wgpu::BufferDescriptor {
      size: 0,
      usage: wgpu::BufferUsages::INDEX,
      mapped_at_creation: false,
      label: None,
    });
    info!("Initialized ImGui {} context.", imgui::dear_imgui_version());

    world.add_system(stage::PRE_DRAW, Self::pre);
    world.add_system(stage::POST_DRAW, Self::pass);
    world.add_system(stage::EVENT, Self::event);
    Ok(Self {
      pipeline,
      ctx,
      textures,
      vert_buf,
      idx_buf,
    })
  }

  fn pre(world: &World) -> Result {
    let pipeline = world.get_resource_mut::<UiPass>().unwrap();
    world.add_resource(unsafe { (pipeline.ctx.new_frame() as *const Ui).read() });
    Ok(())
  }

  fn pass(world: &World) -> Result {
    let renderer = world.get_resource::<Renderer>().unwrap();
    let frame = world.get_resource_mut::<Frame>().unwrap();
    let pipeline = world.get_resource_mut::<UiPass>().unwrap();

    world.take_resource::<Ui>().unwrap();
    let io = pipeline.ctx.io_mut();
    io.update_delta_time(world.get_resource::<DeltaTime>().unwrap().0);
    let window = world.get_resource::<Window>().unwrap();
    if io.want_set_mouse_pos {
      window.set_cursor_position(PhysicalPosition::new(io.mouse_pos[0], io.mouse_pos[1]))?;
    }
    if let Some(c) = pipeline.ctx.mouse_cursor() {
      window.set_cursor_visible(true);
      window.set_cursor_icon(to_winit_cursor(c));
    } else {
      window.set_cursor_visible(false);
    }

    let draw_data = pipeline.ctx.render();
    let mut vertices = Vec::with_capacity(draw_data.total_vtx_count as _);
    let mut indices = Vec::with_capacity(draw_data.total_idx_count as _);
    for list in draw_data.draw_lists() {
      vertices.extend_from_slice(list.vtx_buffer());
      indices.extend_from_slice(list.idx_buffer());
    }

    if (pipeline.vert_buf.size() as usize) < vertices.len() * mem::size_of::<DrawVert>() {
      pipeline.vert_buf = renderer
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
          contents: cast_slice(&vertices),
          usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
          label: None,
        });
    } else {
      renderer
        .queue
        .write_buffer(&pipeline.vert_buf, 0, cast_slice(&vertices));
    }
    indices.resize(
      indices.len() + wgpu::COPY_BUFFER_ALIGNMENT as usize
        - indices.len() % wgpu::COPY_BUFFER_ALIGNMENT as usize,
      0,
    );
    if (pipeline.idx_buf.size() as usize) < indices.len() * mem::size_of::<u16>() {
      pipeline.idx_buf = renderer
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
          contents: cast_slice(&indices),
          usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
          label: None,
        });
    } else {
      renderer
        .queue
        .write_buffer(&pipeline.idx_buf, 0, cast_slice(&indices));
    }

    let mut render_pass = frame
      .encoder
      .begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &frame.surface_view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
        label: None,
      });
    render_pass.set_pipeline(&pipeline.pipeline);
    world
      .get_resource::<Binding<SceneConst>>()
      .unwrap()
      .bind(&mut render_pass, 0);
    render_pass.set_vertex_buffer(0, pipeline.vert_buf.slice(..));
    render_pass.set_index_buffer(pipeline.idx_buf.slice(..), wgpu::IndexFormat::Uint16);
    let mut vert_offset = 0;
    let mut idx_offset = 0;
    for list in draw_data.draw_lists() {
      for cmd in list.commands() {
        if let DrawCmd::Elements { count, cmd_params } = cmd {
          pipeline
            .textures
            .get(cmd_params.texture_id)
            .unwrap()
            .bind(&mut render_pass, 1);
          render_pass.set_scissor_rect(
            cmd_params.clip_rect[0].floor() as _,
            cmd_params.clip_rect[1].floor() as _,
            (cmd_params.clip_rect[2] - cmd_params.clip_rect[0].ceil()) as _,
            (cmd_params.clip_rect[3] - cmd_params.clip_rect[1].ceil()) as _,
          );
          let start = idx_offset as u32 + cmd_params.idx_offset as u32;
          render_pass.draw_indexed(
            start..start + count as u32,
            vert_offset as i32 + cmd_params.vtx_offset as i32,
            0..1,
          );
        }
      }
      vert_offset += list.vtx_buffer().len();
      idx_offset += list.idx_buffer().len();
    }
    Ok(())
  }

  fn event(world: &World) -> Result {
    let pipeline = world.get_resource_mut::<UiPass>().unwrap();
    let io = pipeline.ctx.io_mut();
    match world.get_resource::<WindowEvent>().unwrap() {
      WindowEvent::Resized(size) => {
        io.display_size = [size.width as _, size.height as _];
      }
      WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
        io.display_framebuffer_scale = [*scale_factor as _; 2];
      }
      WindowEvent::MouseInput { state, button, .. } => {
        let pressed = *state == ElementState::Pressed;
        match button {
          MouseButton::Left => io.mouse_down[0] = pressed,
          MouseButton::Right => io.mouse_down[1] = pressed,
          MouseButton::Middle => io.mouse_down[2] = pressed,
          _ => (),
        }
      }
      WindowEvent::CursorMoved { position, .. } => {
        io.mouse_pos = [position.x as _, position.y as _];
      }
      WindowEvent::MouseWheel {
        delta,
        phase: TouchPhase::Moved,
        ..
      } => {
        let (h, v) = match delta {
          MouseScrollDelta::LineDelta(h, v) => (*h, *v),
          MouseScrollDelta::PixelDelta(pos) => (0.01 * pos.x as f32, 0.01 * pos.y as f32),
        };
        io.mouse_wheel_h = h;
        io.mouse_wheel = v;
      }
      WindowEvent::ModifiersChanged(modifiers) => {
        io.key_shift = modifiers.state().shift_key();
        io.key_ctrl = modifiers.state().control_key();
        io.key_alt = modifiers.state().alt_key();
        io.key_super = modifiers.state().super_key();
      }
      WindowEvent::KeyboardInput { event, .. } => {
        if let PhysicalKey::Code(k) = event.physical_key {
          for k in to_imgui_keys(k) {
            io.add_key_event(*k, event.state == ElementState::Pressed);
          }
        }
        if let Some(text) = &event.text {
          for c in text.chars() {
            io.add_input_character(c);
          }
        }
      }
      _ => {}
    }
    Ok(())
  }
}

fn to_imgui_keys(keycode: KeyCode) -> &'static [Key] {
  match keycode {
    KeyCode::Tab => &[Key::Tab],
    KeyCode::ArrowLeft => &[Key::LeftArrow],
    KeyCode::ArrowRight => &[Key::RightArrow],
    KeyCode::ArrowUp => &[Key::UpArrow],
    KeyCode::ArrowDown => &[Key::DownArrow],
    KeyCode::PageUp => &[Key::PageUp],
    KeyCode::PageDown => &[Key::PageDown],
    KeyCode::Home => &[Key::Home],
    KeyCode::End => &[Key::End],
    KeyCode::Insert => &[Key::Insert],
    KeyCode::Delete => &[Key::Delete],
    KeyCode::Backspace => &[Key::Backspace],
    KeyCode::Space => &[Key::Space],
    KeyCode::Enter => &[Key::Enter],
    KeyCode::Escape => &[Key::Escape],
    KeyCode::ControlLeft => &[Key::LeftCtrl, Key::ModCtrl],
    KeyCode::ShiftLeft => &[Key::LeftShift, Key::ModShift],
    KeyCode::AltLeft => &[Key::LeftAlt, Key::ModAlt],
    KeyCode::SuperLeft => &[Key::LeftSuper, Key::ModSuper],
    KeyCode::ControlRight => &[Key::RightCtrl, Key::ModCtrl],
    KeyCode::ShiftRight => &[Key::RightShift, Key::ModShift],
    KeyCode::AltRight => &[Key::RightAlt, Key::ModAlt],
    KeyCode::SuperRight => &[Key::RightSuper, Key::ModSuper],
    KeyCode::ContextMenu => &[Key::Menu],
    KeyCode::Digit0 => &[Key::Alpha0],
    KeyCode::Digit1 => &[Key::Alpha1],
    KeyCode::Digit2 => &[Key::Alpha2],
    KeyCode::Digit3 => &[Key::Alpha3],
    KeyCode::Digit4 => &[Key::Alpha4],
    KeyCode::Digit5 => &[Key::Alpha5],
    KeyCode::Digit6 => &[Key::Alpha6],
    KeyCode::Digit7 => &[Key::Alpha7],
    KeyCode::Digit8 => &[Key::Alpha8],
    KeyCode::Digit9 => &[Key::Alpha9],
    KeyCode::KeyA => &[Key::A],
    KeyCode::KeyB => &[Key::B],
    KeyCode::KeyC => &[Key::C],
    KeyCode::KeyD => &[Key::D],
    KeyCode::KeyE => &[Key::E],
    KeyCode::KeyF => &[Key::F],
    KeyCode::KeyG => &[Key::G],
    KeyCode::KeyH => &[Key::H],
    KeyCode::KeyI => &[Key::I],
    KeyCode::KeyJ => &[Key::J],
    KeyCode::KeyK => &[Key::K],
    KeyCode::KeyL => &[Key::L],
    KeyCode::KeyM => &[Key::M],
    KeyCode::KeyN => &[Key::N],
    KeyCode::KeyO => &[Key::O],
    KeyCode::KeyP => &[Key::P],
    KeyCode::KeyQ => &[Key::Q],
    KeyCode::KeyR => &[Key::R],
    KeyCode::KeyS => &[Key::S],
    KeyCode::KeyT => &[Key::T],
    KeyCode::KeyU => &[Key::U],
    KeyCode::KeyV => &[Key::V],
    KeyCode::KeyW => &[Key::W],
    KeyCode::KeyX => &[Key::X],
    KeyCode::KeyY => &[Key::Y],
    KeyCode::KeyZ => &[Key::Z],
    KeyCode::F1 => &[Key::F1],
    KeyCode::F2 => &[Key::F2],
    KeyCode::F3 => &[Key::F3],
    KeyCode::F4 => &[Key::F4],
    KeyCode::F5 => &[Key::F5],
    KeyCode::F6 => &[Key::F6],
    KeyCode::F7 => &[Key::F7],
    KeyCode::F8 => &[Key::F8],
    KeyCode::F9 => &[Key::F9],
    KeyCode::F10 => &[Key::F10],
    KeyCode::F11 => &[Key::F11],
    KeyCode::F12 => &[Key::F12],
    KeyCode::Quote => &[Key::Apostrophe],
    KeyCode::Comma => &[Key::Comma],
    KeyCode::Minus => &[Key::Minus],
    KeyCode::Period => &[Key::Period],
    KeyCode::Slash => &[Key::Slash],
    KeyCode::Semicolon => &[Key::Semicolon],
    KeyCode::Equal => &[Key::Equal],
    KeyCode::BracketLeft => &[Key::LeftBracket],
    KeyCode::Backslash => &[Key::Backslash],
    KeyCode::BracketRight => &[Key::RightBracket],
    KeyCode::Backquote => &[Key::GraveAccent],
    KeyCode::CapsLock => &[Key::CapsLock],
    KeyCode::ScrollLock => &[Key::ScrollLock],
    KeyCode::NumLock => &[Key::NumLock],
    KeyCode::PrintScreen => &[Key::PrintScreen],
    KeyCode::Pause => &[Key::Pause],
    KeyCode::Numpad0 => &[Key::Keypad0],
    KeyCode::Numpad1 => &[Key::Keypad1],
    KeyCode::Numpad2 => &[Key::Keypad2],
    KeyCode::Numpad3 => &[Key::Keypad3],
    KeyCode::Numpad4 => &[Key::Keypad4],
    KeyCode::Numpad5 => &[Key::Keypad5],
    KeyCode::Numpad6 => &[Key::Keypad6],
    KeyCode::Numpad7 => &[Key::Keypad7],
    KeyCode::Numpad8 => &[Key::Keypad8],
    KeyCode::Numpad9 => &[Key::Keypad9],
    KeyCode::NumpadDecimal => &[Key::KeypadDecimal],
    KeyCode::NumpadDivide => &[Key::KeypadDivide],
    KeyCode::NumpadMultiply => &[Key::KeypadMultiply],
    KeyCode::NumpadSubtract => &[Key::KeypadSubtract],
    KeyCode::NumpadAdd => &[Key::KeypadAdd],
    KeyCode::NumpadEnter => &[Key::KeypadEnter],
    KeyCode::NumpadEqual => &[Key::KeypadEqual],
    _ => &[],
  }
}

fn to_winit_cursor(cursor: MouseCursor) -> CursorIcon {
  match cursor {
    MouseCursor::Arrow => CursorIcon::Default,
    MouseCursor::TextInput => CursorIcon::Text,
    MouseCursor::ResizeAll => CursorIcon::Move,
    MouseCursor::ResizeNS => CursorIcon::NsResize,
    MouseCursor::ResizeEW => CursorIcon::EwResize,
    MouseCursor::ResizeNESW => CursorIcon::NeswResize,
    MouseCursor::ResizeNWSE => CursorIcon::NwseResize,
    MouseCursor::Hand => CursorIcon::Grab,
    MouseCursor::NotAllowed => CursorIcon::NotAllowed,
  }
}
