pub struct Texture {
	pub texture: wgpu::Texture,
	pub view: wgpu::TextureView,
	pub mip_view: Vec<wgpu::TextureView>,
	pub sampler: Option<wgpu::Sampler>,
	pub size: wgpu::Extent3d,
	mip_count: u32,
	sample_count: u32,
	dimension: wgpu::TextureDimension,
	pub format: wgpu::TextureFormat,
	pub usage: wgpu::TextureUsages,
	label: Option<String>,
	aspect: wgpu::TextureAspect,
}

impl Texture {
	#![allow(clippy::too_many_arguments)]
	pub fn new(
		device: &wgpu::Device,
		label: Option<&str>,
		width: u32,
		height: u32,
		mip_count: u32,
		sample_count: u32,
		dimension: wgpu::TextureDimension,
		format: wgpu::TextureFormat,
		usage: wgpu::TextureUsages,
		aspect: wgpu::TextureAspect,
	) -> Self {
		let size = wgpu::Extent3d {
			width,
			height,
			depth_or_array_layers: 1,
		};
		let texture = device.create_texture(&wgpu::TextureDescriptor {
			label,
			size,
			mip_level_count: mip_count,
			sample_count: sample_count,
			dimension,
			format,
			usage,
		});

		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

		let view_dimension = match dimension {
			wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
			wgpu::TextureDimension::D2 => wgpu::TextureViewDimension::D2,
			wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
		};

		let mut mip_view = Vec::with_capacity(mip_count as usize);
		for i in 0..mip_count {
			mip_view.push(texture.create_view(&wgpu::TextureViewDescriptor {
				label,
				format: Some(format),
				dimension: Some(view_dimension),
				aspect,
				base_mip_level: i,
				mip_level_count: std::num::NonZeroU32::new(1),
				base_array_layer: 0,
				array_layer_count: std::num::NonZeroU32::new(1),
			}));
		}

		let label = match label {
			Some(l) => Some(l.to_owned()),
			None => None,
		};

		Texture {
			texture,
			view,
			mip_view,
			sampler: None,
			format,
			size,
			usage,
			dimension,
			mip_count,
			sample_count,
			label,
			aspect,
		}
	}

	pub fn set_sampler(
		&mut self,
		device: &wgpu::Device,
		label: Option<&str>,
		address_mode: wgpu::AddressMode,
		mag: wgpu::FilterMode,
		min: wgpu::FilterMode,
		mipmap: wgpu::FilterMode,
		lod_min: Option<f32>,
		lod_max: Option<f32>,
		compare: Option<wgpu::CompareFunction>,
		anisotropy_clamp: Option<std::num::NonZeroU8>,
		border_color: Option<wgpu::SamplerBorderColor>,
	) {
		let lod_min = if let Some(min) = lod_min { min } else { 0.0 };
		let lod_max = if let Some(max) = lod_max {
			max
		} else {
			std::f32::MAX
		};

		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			label,
			address_mode_u: address_mode,
			address_mode_v: address_mode,
			address_mode_w: address_mode,
			mag_filter: mag,
			min_filter: min,
			mipmap_filter: mipmap,
			lod_min_clamp: lod_min,
			lod_max_clamp: lod_max,
			compare,
			anisotropy_clamp,
			border_color,
		});

		self.sampler = Some(sampler);
	}

	pub fn recreate(&mut self, device: &wgpu::Device, size: wgpu::Extent3d) {
		let taken_label = std::mem::take(&mut self.label);
		let taken_label = taken_label.unwrap_or("".to_string());
		let label = if taken_label.is_empty() {
			None
		} else {
			Some(taken_label.as_str())
		};
		let texture = device.create_texture(&wgpu::TextureDescriptor {
			label,
			size: size,
			mip_level_count: self.mip_count,
			sample_count: self.sample_count,
			dimension: self.dimension,
			format: self.format,
			usage: self.usage,
		});
		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

		let view_dimension = match self.dimension {
			wgpu::TextureDimension::D1 => wgpu::TextureViewDimension::D1,
			wgpu::TextureDimension::D2 => wgpu::TextureViewDimension::D2,
			wgpu::TextureDimension::D3 => wgpu::TextureViewDimension::D3,
		};

		let mut mip_view = Vec::with_capacity(self.mip_count as usize);
		for i in 0..self.mip_count {
			mip_view.push(texture.create_view(&wgpu::TextureViewDescriptor {
				label,
				format: Some(self.format),
				dimension: Some(view_dimension),
				aspect: self.aspect,
				base_mip_level: i,
				mip_level_count: std::num::NonZeroU32::new(1),
				base_array_layer: 0,
				array_layer_count: std::num::NonZeroU32::new(1),
			}));
		}

		self.texture.destroy();
		self.label = Some(taken_label);
		self.size = size;
		self.texture = texture;
		self.view = view;
		self.mip_view = mip_view;
	}
}
