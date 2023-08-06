use std::time::Instant;
use winit::window::Window;
use winit::event::Event;
use imgui::{Context, StyleColor, FontSource};
use imgui_winit_support::WinitPlatform;
use imgui_wgpu::Renderer as ImguiRenderer;
use crate::{Renderer, Texture, FORMAT};

pub struct ImguiPipeline {
  imgui: Context,
  platform: WinitPlatform,
  renderer: ImguiRenderer,
  last_frame: Instant,
}

impl ImguiPipeline {
  pub fn new(renderer: &Renderer, window: &Window) -> Self {
    let mut imgui = Context::create();
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    platform.attach_window(
      imgui.io_mut(),
      window,
      imgui_winit_support::HiDpiMode::Default,
    );
    imgui.set_ini_filename(None);
    let style = imgui.style_mut();
    style[StyleColor::Text] = [1.00, 1.00, 1.00, 1.00];
    style[StyleColor::TextDisabled] = [0.50, 0.50, 0.50, 1.00];
    style[StyleColor::WindowBg] = [0.10, 0.10, 0.10, 1.00];
    style[StyleColor::ChildBg] = [0.00, 0.00, 0.00, 0.00];
    style[StyleColor::PopupBg] = [0.19, 0.19, 0.19, 0.92];
    style[StyleColor::Border] = [0.19, 0.19, 0.19, 0.29];
    style[StyleColor::BorderShadow] = [0.00, 0.00, 0.00, 0.24];
    style[StyleColor::FrameBg] = [0.05, 0.05, 0.05, 0.54];
    style[StyleColor::FrameBgHovered] = [0.19, 0.19, 0.19, 0.54];
    style[StyleColor::FrameBgActive] = [0.20, 0.22, 0.23, 1.00];
    style[StyleColor::TitleBg] = [0.00, 0.00, 0.00, 1.00];
    style[StyleColor::TitleBgActive] = [0.06, 0.06, 0.06, 1.00];
    style[StyleColor::TitleBgCollapsed] = [0.00, 0.00, 0.00, 1.00];
    style[StyleColor::MenuBarBg] = [0.14, 0.14, 0.14, 1.00];
    style[StyleColor::ScrollbarBg] = [0.05, 0.05, 0.05, 0.54];
    style[StyleColor::ScrollbarGrab] = [0.34, 0.34, 0.34, 0.54];
    style[StyleColor::ScrollbarGrabHovered] = [0.40, 0.40, 0.40, 0.54];
    style[StyleColor::ScrollbarGrabActive] = [0.56, 0.56, 0.56, 0.54];
    style[StyleColor::CheckMark] = [0.33, 0.67, 0.86, 1.00];
    style[StyleColor::SliderGrab] = [0.34, 0.34, 0.34, 0.54];
    style[StyleColor::SliderGrabActive] = [0.56, 0.56, 0.56, 0.54];
    style[StyleColor::Button] = [0.05, 0.05, 0.05, 0.54];
    style[StyleColor::ButtonHovered] = [0.19, 0.19, 0.19, 0.54];
    style[StyleColor::ButtonActive] = [0.20, 0.22, 0.23, 1.00];
    style[StyleColor::Header] = [0.00, 0.00, 0.00, 0.52];
    style[StyleColor::HeaderHovered] = [0.00, 0.00, 0.00, 0.36];
    style[StyleColor::HeaderActive] = [0.20, 0.22, 0.23, 0.33];
    style[StyleColor::Separator] = [0.28, 0.28, 0.28, 0.29];
    style[StyleColor::SeparatorHovered] = [0.44, 0.44, 0.44, 0.29];
    style[StyleColor::SeparatorActive] = [0.40, 0.44, 0.47, 1.00];
    style[StyleColor::ResizeGrip] = [0.28, 0.28, 0.28, 0.29];
    style[StyleColor::ResizeGripHovered] = [0.44, 0.44, 0.44, 0.29];
    style[StyleColor::ResizeGripActive] = [0.40, 0.44, 0.47, 1.00];
    style[StyleColor::Tab] = [0.00, 0.00, 0.00, 0.52];
    style[StyleColor::TabHovered] = [0.14, 0.14, 0.14, 1.00];
    style[StyleColor::TabActive] = [0.20, 0.20, 0.20, 0.36];
    style[StyleColor::TabUnfocused] = [0.00, 0.00, 0.00, 0.52];
    style[StyleColor::TabUnfocusedActive] = [0.14, 0.14, 0.14, 1.00];
    // style[StyleColor::DockingPreview] = [0.33, 0.67, 0.86, 1.00];
    // style[StyleColor::DockingEmptyBg] = [0.10, 0.10, 0.10, 1.00];
    style[StyleColor::PlotLines] = [1.00, 0.00, 0.00, 1.00];
    style[StyleColor::PlotLinesHovered] = [1.00, 0.00, 0.00, 1.00];
    style[StyleColor::PlotHistogram] = [1.00, 0.00, 0.00, 1.00];
    style[StyleColor::PlotHistogramHovered] = [1.00, 0.00, 0.00, 1.00];
    style[StyleColor::TableHeaderBg] = [0.00, 0.00, 0.00, 0.52];
    style[StyleColor::TableBorderStrong] = [0.00, 0.00, 0.00, 0.52];
    style[StyleColor::TableBorderLight] = [0.28, 0.28, 0.28, 0.29];
    style[StyleColor::TableRowBg] = [0.00, 0.00, 0.00, 0.00];
    style[StyleColor::TableRowBgAlt] = [1.00, 1.00, 1.00, 0.06];
    style[StyleColor::TextSelectedBg] = [0.20, 0.22, 0.23, 1.00];
    style[StyleColor::DragDropTarget] = [0.33, 0.67, 0.86, 1.00];
    style[StyleColor::NavHighlight] = [0.05, 0.05, 0.05, 0.54];
    style[StyleColor::NavWindowingHighlight] = [0.19, 0.19, 0.19, 0.54];
    style[StyleColor::NavWindowingDimBg] = [1.00, 0.00, 0.00, 0.20];
    style[StyleColor::ModalWindowDimBg] = [1.00, 0.00, 0.00, 0.35];
    style.window_rounding = 4.0;
    style.popup_rounding = 4.0;
    style.frame_rounding = 2.0;
    imgui.fonts().add_font(&[FontSource::TtfData {
      data: include_bytes!("roboto.ttf"),
      size_pixels: 15.0,
      config: None,
    }]);
    let renderer = imgui_wgpu::Renderer::new(
      &mut imgui,
      &renderer.device,
      &renderer.queue,
      imgui_wgpu::RendererConfig {
        texture_format: FORMAT,
        ..Default::default()
      },
    );
    Self {
      imgui,
      platform,
      renderer,
      last_frame: Instant::now(),
    }
  }

  pub fn handle_event<T>(&mut self, window: &Window, event: &Event<T>) {
    self
      .platform
      .handle_event(self.imgui.io_mut(), window, event);
  }

  pub fn render(
    &mut self,
    renderer: &Renderer,
    window: &Window,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
  ) {
    let now = Instant::now();
    self.imgui.io_mut().update_delta_time(now - self.last_frame);
    self.last_frame = now;
    self
      .platform
      .prepare_frame(self.imgui.io_mut(), window)
      .unwrap();
    let ui = self.imgui.frame();
    ui.window("meow").build(|| {
      ui.text("miau! ");
    });
    // ui.show_demo_window(&mut true);
    self.platform.prepare_render(ui, window);

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: None,
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Load,
          store: true,
        },
      })],
      depth_stencil_attachment: None,
    });
    self
      .renderer
      .render(
        self.imgui.render(),
        &renderer.queue,
        &renderer.device,
        &mut render_pass,
      )
      .unwrap();
    drop(render_pass);
  }
}
