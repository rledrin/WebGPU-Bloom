use wgpu::util::DeviceExt;

use super::{texture::Texture, Vertex};

pub struct Material {
	pub render_pipeline: Option<wgpu::RenderPipeline>,
	pub compute_pipeline: Option<wgpu::ComputePipeline>,
	pub bind_group: Vec<wgpu::BindGroup>,
	pub bind_group_layout: Vec<wgpu::BindGroupLayout>,
	pub bind_groups_buffers: Vec<wgpu::Buffer>,
	pub bind_groups_textures: Vec<Texture>,
	pub push_constant: Vec<wgpu::PushConstantRange>,
}

pub struct Mesh {
	pub vertex_buffer: Option<wgpu::Buffer>,
	pub index_buffer: Option<wgpu::Buffer>,
	pub draw_count: u32,
	pub model: ultraviolet::Mat4,
	pub material: Option<Material>,
}

pub trait IndexType {}
impl IndexType for u16 {}
impl IndexType for u32 {}

impl Mesh {
	#![allow(unused)]
	pub fn new<T: IndexType>(
		device: &wgpu::Device,
		label: Option<&str>,
		vertex_buffer_data: &Vec<Vertex>,
		index_buffer_data: Option<&Vec<T>>,
		material: Option<Material>,
	) -> Self {
		let (index_buffer, draw_count) = match index_buffer_data {
			Some(data) => {
				let content = unsafe { data.align_to::<u8>().1 };
				(
					Some(
						device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
							label,
							contents: content,
							usage: wgpu::BufferUsages::INDEX,
						}),
					),
					data.len() as u32,
				)
			}
			None => (None, vertex_buffer_data.len() as u32),
		};

		let content = unsafe { vertex_buffer_data.align_to::<u8>().1 };
		let vertex_buffer = match vertex_buffer_data.is_empty() {
			true => None,
			false => Some(
				device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label,
					contents: content,
					usage: wgpu::BufferUsages::VERTEX,
				}),
			),
		};

		Mesh {
			vertex_buffer,
			index_buffer,
			draw_count,
			material,
			model: ultraviolet::Mat4::identity(),
		}
	}
}

impl Material {
	#![allow(unused)]
	pub fn new(number_of_bind_group: usize, number_of_push_constant: usize) -> Self {
		Material {
			render_pipeline: None,
			compute_pipeline: None,
			bind_group: Vec::with_capacity(number_of_bind_group),
			bind_group_layout: Vec::with_capacity(number_of_bind_group),
			bind_groups_buffers: vec![],
			bind_groups_textures: vec![],
			push_constant: Vec::with_capacity(number_of_push_constant),
		}
	}

	pub fn add_bind_group(
		&mut self,
		device: &wgpu::Device,
		label: Option<&str>,
		layout_entries: Vec<wgpu::BindGroupLayoutEntry>,
		bindings: Vec<wgpu::BindingResource>,
	) {
		if layout_entries.len() != bindings.len() && !bindings.is_empty() {
			panic!("layout_entries.len() != bindings.len().");
		}

		let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label,
			entries: &layout_entries,
		});

		if !bindings.is_empty() {
			let mut binding_entries = Vec::with_capacity(bindings.len());
			for (i, resource) in bindings.into_iter().enumerate() {
				binding_entries.push(wgpu::BindGroupEntry {
					binding: i as u32,
					resource,
				});
			}
			self.bind_group
				.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
					label,
					layout: &layout,
					entries: &binding_entries,
				}));
		}
		self.bind_group_layout.push(layout);
	}

	pub fn add_push_constant(&mut self, stage: wgpu::ShaderStages, range: std::ops::Range<u32>) {
		self.push_constant.push(wgpu::PushConstantRange {
			stages: stage,
			range,
		});
	}

	pub fn set_render_pipeline(
		&mut self,
		device: &wgpu::Device,
		label: Option<&str>,
		shader: wgpu::ShaderModuleDescriptor,
		render_format: wgpu::TextureFormat,
		depth_format: Option<wgpu::TextureFormat>,
	) {
		let shader = device.create_shader_module(&shader);
		let render_pipeline_layout =
			device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Render Pipeline Layout"),
				bind_group_layouts: &self.bind_group_layout.iter().map(|x| x).collect::<Vec<_>>(),
				push_constant_ranges: &self.push_constant,
			});

		let depth = match depth_format {
			Some(f) => Some(wgpu::DepthStencilState {
				format: f,
				depth_write_enabled: true,
				depth_compare: wgpu::CompareFunction::Less,
				stencil: wgpu::StencilState::default(),
				bias: wgpu::DepthBiasState::default(),
			}),
			None => None,
		};

		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: label,
			layout: Some(&render_pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: "vs_main",
				buffers: &[Vertex::layout()],
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: "fs_main",
				targets: &[wgpu::ColorTargetState {
					format: render_format,
					blend: Some(wgpu::BlendState {
						color: wgpu::BlendComponent::REPLACE,
						alpha: wgpu::BlendComponent::REPLACE,
					}),
					write_mask: wgpu::ColorWrites::ALL,
				}],
			}),
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleList,
				strip_index_format: None,
				front_face: wgpu::FrontFace::Ccw,
				cull_mode: Some(wgpu::Face::Back),
				polygon_mode: wgpu::PolygonMode::Fill,
				unclipped_depth: false,
				conservative: false,
			},
			depth_stencil: depth,
			multisample: wgpu::MultisampleState {
				count: 1,
				mask: !0,
				alpha_to_coverage_enabled: false,
			},
			multiview: None,
		});
		self.render_pipeline = Some(render_pipeline);
	}

	pub fn set_compute_pipeline(
		&mut self,
		device: &wgpu::Device,
		label: Option<&str>,
		shader: wgpu::ShaderModuleDescriptor,
	) {
		let shader = device.create_shader_module(&shader);
		let compute_pipeline_layout =
			device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("compute Pipeline Layout"),
				bind_group_layouts: &self.bind_group_layout.iter().map(|x| x).collect::<Vec<_>>(),
				push_constant_ranges: &self.push_constant,
			});
		let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
			label,
			layout: Some(&compute_pipeline_layout),
			module: &shader,
			entry_point: "cs_main",
		});
		self.compute_pipeline = Some(compute_pipeline);
	}

	pub fn set_compute_pipeline_spv(
		&mut self,
		device: &wgpu::Device,
		label: Option<&str>,
		shader: wgpu::ShaderModuleDescriptorSpirV,
	) {
		let shader = unsafe { device.create_shader_module_spirv(&shader) };
		let compute_pipeline_layout =
			device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("compute Pipeline Layout"),
				bind_group_layouts: &self.bind_group_layout.iter().map(|x| x).collect::<Vec<_>>(),
				push_constant_ranges: &self.push_constant,
			});
		let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
			label,
			layout: Some(&compute_pipeline_layout),
			module: &shader,
			entry_point: "main",
		});
		self.compute_pipeline = Some(compute_pipeline);
	}
}
