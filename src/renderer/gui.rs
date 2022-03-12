use egui_wgpu_backend::RenderPass;
use egui_winit_platform::{Platform, PlatformDescriptor};

pub struct Gui {
	pub platform: Platform,
	pub render_pass: RenderPass,
}

impl Gui {
	pub fn new(
		window: &winit::window::Window,
		device: &wgpu::Device,
		surface_format: wgpu::TextureFormat,
		msaa_samples: u32,
	) -> Self {
		let size = window.inner_size();

		let platform = Platform::new(PlatformDescriptor {
			physical_width: size.width,
			physical_height: size.height,
			scale_factor: window.scale_factor(),
			font_definitions: egui::FontDefinitions::default(),
			style: Default::default(),
		});

		let render_pass = RenderPass::new(device, surface_format, msaa_samples);

		Gui {
			platform,
			render_pass,
		}
	}
}

pub enum Event {
	RequestRedraw,
}

/// This is the repaint signal type that egui needs for requesting a repaint from another thread.
/// It sends the custom RequestRedraw event to the winit event loop.
pub struct ExampleRepaintSignal(pub std::sync::Mutex<winit::event_loop::EventLoopProxy<Event>>);

impl epi::backend::RepaintSignal for ExampleRepaintSignal {
	fn request_repaint(&self) {
		self.0.lock().unwrap().send_event(Event::RequestRedraw).ok();
	}
}
