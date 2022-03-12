use winit::{dpi::PhysicalSize, window::Window};

pub struct Context {
	pub surface: wgpu::Surface,
	pub device: wgpu::Device,
	pub queue: wgpu::Queue,
	pub config: wgpu::SurfaceConfiguration,
	pub adapter: wgpu::Adapter,
	pub window: Window,
	pub size: PhysicalSize<u32>,
}

impl Context {
	pub async fn new(
		window: Window,
		features: Option<wgpu::Features>,
		limits: Option<wgpu::Limits>,
	) -> Self {
		let size = window.inner_size();

		let instance = wgpu::Instance::new(wgpu::Backends::all());
		let surface = unsafe { instance.create_surface(&window) };
		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(),
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			})
			.await
			.expect("Failed to pick an adaptater.");
		#[cfg(debug_assertions)]
		println!(
			"\t|| Name: {} | Type: {:?} | Backend: {:?} ||\n",
			adapter.get_info().name,
			adapter.get_info().device_type,
			adapter.get_info().backend
		);

		let features = if let Some(f) = features {
			f
		} else {
			wgpu::Features::empty()
		};
		let limits = if let Some(l) = limits {
			l
		} else {
			wgpu::Limits::default()
		};

		let (device, queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					features,
					limits,
					label: None,
				},
				None,
			)
			.await
			.expect("Failed to request for the device and queue.");

		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface.get_preferred_format(&adapter).unwrap(),
			width: size.width,
			height: size.height,
			present_mode: wgpu::PresentMode::Fifo,
		};
		surface.configure(&device, &config);

		Context {
			surface,
			config,
			device,
			queue,
			adapter,
			window,
			size,
		}
	}

	#[allow(unused)]
	pub fn configure_surface(
		&mut self,
		usage: Option<wgpu::TextureUsages>,
		format: Option<wgpu::TextureFormat>,
		size: Option<PhysicalSize<u32>>,
		present_mode: Option<wgpu::PresentMode>,
	) {
		let mut config = self.config.clone();

		if let Some(u) = usage {
			config.usage = u;
		}
		if let Some(f) = format {
			config.format = f;
		}
		if let Some(s) = size {
			config.width = s.width;
			config.height = s.height;
			self.size = s;
		}
		if let Some(pm) = present_mode {
			config.present_mode = pm;
		}

		self.surface.configure(&self.device, &config);
		self.config = config;
	}

	pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.size = new_size;
			self.config.width = new_size.width;
			self.config.height = new_size.height;
			self.surface.configure(&self.device, &self.config);
		}
	}
}
