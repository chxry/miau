use winit::window::Window;
use winit::event::WindowEvent;
use egui_wgpu_backend::ScreenDescriptor;
use egui::{SidePanel, TopBottomPanel, CentralPanel};
use crate::{Renderer, FORMAT};

pub struct UiPipeline {
  ctx: egui::Context,
  platform: egui_winit::State,
  renderer: egui_wgpu_backend::RenderPass,
}

impl UiPipeline {
  pub fn new(renderer: &Renderer, window: &Window) -> Self {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(window.scale_factor() as _);
    let platform = egui_winit::State::new(&window);
    let renderer = egui_wgpu_backend::RenderPass::new(&renderer.device, FORMAT, 1);
    Self {
      ctx,
      platform,
      renderer,
    }
  }

  pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
    self.platform.on_event(&self.ctx, event).consumed
  }

  pub fn render(
    &mut self,
    renderer: &Renderer,
    encoder: &mut wgpu::CommandEncoder,
    window: &Window,
    tex: &wgpu::Texture,
    view: &wgpu::TextureView,
  ) {
    self.ctx.begin_frame(self.platform.take_egui_input(&window));

    TopBottomPanel::bottom("bottom").show(&self.ctx, |ui| {
      ui.label("owo");
    });
    SidePanel::left("left").show(&self.ctx, |ui| {
      ui.label("uwu");
    });
    SidePanel::right("right").show(&self.ctx, |ui| {
      ui.label(":3");
    });
    let out = self.ctx.end_frame();
    self
      .platform
      .handle_platform_output(&window, &self.ctx, out.platform_output);
    self
      .renderer
      .add_textures(&renderer.device, &renderer.queue, &out.textures_delta)
      .unwrap();
    let size = ScreenDescriptor {
      physical_width: tex.width(),
      physical_height: tex.height(),
      scale_factor: self.ctx.pixels_per_point(),
    };
    let primitives = self.ctx.tessellate(out.shapes);
    self
      .renderer
      .update_buffers(&renderer.device, &renderer.queue, &primitives, &size);
    self
      .renderer
      .execute(encoder, view, &primitives, &size, None)
      .unwrap();
  }
}
