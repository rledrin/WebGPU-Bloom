extern crate ultraviolet as uv;

pub mod camera;
pub mod mesh;
pub mod texture;

use crate::{bloom, context::Context};
use texture::Texture;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, event::WindowEvent};

use std::collections::HashMap;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Vertex {
	pub position: uv::Vec3,
	pub normal: uv::Vec3,
	pub uv: uv::Vec2,
}

pub struct Renderer {
	pub context: Context,
	pub hdr_texture: Texture,
	pub depth_texture: Texture,
	pub final_pipeline: wgpu::RenderPipeline,
	fullscreen_vertex_buffer: wgpu::Buffer,
	final_bind_group: wgpu::BindGroup,
	final_bind_group_layout: wgpu::BindGroupLayout,
	pub camera: camera::PerspectiveCamera,
	pub meshes: HashMap<String, mesh::Mesh>,
	pub resized: bool,
}

impl Vertex {
	fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
		wgpu::VertexBufferLayout {
			array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 0,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute {
					offset: std::mem::size_of::<uv::Vec3>() as wgpu::BufferAddress,
					shader_location: 1,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute {
					offset: (std::mem::size_of::<uv::Vec3>() * 2) as wgpu::BufferAddress,
					shader_location: 2,
					format: wgpu::VertexFormat::Float32x2,
				},
			],
		}
	}
}

impl Renderer {
	pub fn new(context: Context) -> Self {
		let mut hdr_texture = Texture::new(
			&context.device,
			Some("hdr render texture"),
			context.size.width,
			context.size.height,
			1,
			wgpu::TextureDimension::D2,
			wgpu::TextureFormat::Rgba16Float,
			wgpu::TextureUsages::RENDER_ATTACHMENT
				| wgpu::TextureUsages::STORAGE_BINDING
				| wgpu::TextureUsages::TEXTURE_BINDING,
			wgpu::TextureAspect::All,
		);
		hdr_texture.set_sampler(
			&context.device,
			Some("hdr sampler render texture"),
			wgpu::AddressMode::ClampToEdge,
			wgpu::FilterMode::Linear,
			wgpu::FilterMode::Linear,
			wgpu::FilterMode::Linear,
			Some(-1000.0),
			Some(1000.0),
			None,
			None,
			None,
		);

		let depth_texture = Texture::new(
			&context.device,
			Some("depth texture"),
			context.size.width,
			context.size.height,
			1,
			wgpu::TextureDimension::D2,
			wgpu::TextureFormat::Depth32Float,
			wgpu::TextureUsages::RENDER_ATTACHMENT,
			wgpu::TextureAspect::DepthOnly,
		);

		let final_bind_group_layout =
			context
				.device
				.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
					label: Some("texture_bind_group_layout"),
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
				});

		let shader = context
			.device
			.create_shader_module(&wgpu::include_wgsl!("../../assets/shaders/final.wgsl"));
		let layout = context
			.device
			.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
				label: Some("Render Pipeline Layout"),
				bind_group_layouts: &[&final_bind_group_layout],
				push_constant_ranges: &[],
			});

		let final_pipeline =
			context
				.device
				.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
					label: Some("final render pipeline"),
					layout: Some(&layout),
					vertex: wgpu::VertexState {
						module: &shader,
						entry_point: "vs_main",
						buffers: &[Vertex::layout()],
					},
					fragment: Some(wgpu::FragmentState {
						module: &shader,
						entry_point: "fs_main",
						targets: &[wgpu::ColorTargetState {
							format: context.config.format,
							blend: Some(wgpu::BlendState::REPLACE),
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
					depth_stencil: None,
					multisample: wgpu::MultisampleState {
						count: 1,
						mask: !0,
						alpha_to_coverage_enabled: false,
					},
					multiview: None,
				});
		let fullscreen_quad_data = generate_fullscreen_quad();
		let contents = unsafe { fullscreen_quad_data.align_to::<u8>().1 };
		let fullscreen_vertex_buffer =
			context
				.device
				.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("fullscreen VertexBuffer"),
					contents,
					usage: wgpu::BufferUsages::VERTEX,
				});

		let final_bind_group = context
			.device
			.create_bind_group(&wgpu::BindGroupDescriptor {
				label: Some("final bind group"),
				layout: &final_bind_group_layout,
				entries: &[
					wgpu::BindGroupEntry {
						binding: 0,
						resource: wgpu::BindingResource::TextureView(&hdr_texture.view),
					},
					wgpu::BindGroupEntry {
						binding: 1,
						resource: wgpu::BindingResource::Sampler(
							&hdr_texture.sampler.as_ref().unwrap(),
						),
					},
				],
			});

		let camera = camera::PerspectiveCamera::new(
			uv::Vec3::new(0.0, 0.0, -4.0),
			uv::Vec3::zero(),
			std::f32::consts::FRAC_PI_3,
			context.size.width as f32 / context.size.height as f32,
			0.1,
			1000.0,
		);

		let mut renderer = Renderer {
			context,
			hdr_texture,
			depth_texture,
			final_pipeline,
			fullscreen_vertex_buffer,
			final_bind_group,
			final_bind_group_layout,
			camera,
			meshes: HashMap::with_capacity(2),
			resized: false,
		};
		let pbr_sphere = super::bloom::init_pbr(&renderer);
		let bloom_mesh = super::bloom::init_bloom(&mut renderer);

		renderer.final_bind_group =
			renderer
				.context
				.device
				.create_bind_group(&wgpu::BindGroupDescriptor {
					label: Some("final bind group"),
					layout: &renderer.final_bind_group_layout,
					entries: &[
						wgpu::BindGroupEntry {
							binding: 0,
							resource: wgpu::BindingResource::TextureView(
								&bloom_mesh.material.as_ref().unwrap().bind_groups_textures[3].view,
							),
						},
						wgpu::BindGroupEntry {
							binding: 1,
							resource: wgpu::BindingResource::Sampler(
								&renderer.hdr_texture.sampler.as_ref().unwrap(),
							),
						},
					],
				});

		renderer.meshes.insert("pbr".to_string(), pbr_sphere);
		renderer.meshes.insert("bloom".to_string(), bloom_mesh);
		renderer
	}

	pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
		static mut OK: bool = false;
		if unsafe { OK } {
			self.resized = true;
			let size = wgpu::Extent3d {
				width: new_size.width,
				height: new_size.height,
				depth_or_array_layers: 1,
			};
			self.context.resize(new_size);
			self.depth_texture.recreate(&self.context.device, size);
			self.hdr_texture.recreate(&self.context.device, size);

			let size = wgpu::Extent3d {
				width: new_size.width / 2,
				height: new_size.height / 2,
				depth_or_array_layers: 1,
			};
			for (_, mesh) in self.meshes.iter_mut() {
				if mesh.material.is_some() {
					let material = mesh.material.as_mut().unwrap();
					for text in material.bind_groups_textures.iter_mut() {
						text.recreate(&self.context.device, size)
					}
				}
			}

			self.final_bind_group =
				self.context
					.device
					.create_bind_group(&wgpu::BindGroupDescriptor {
						label: Some("final bind group"),
						layout: &self.final_bind_group_layout,
						entries: &[
							wgpu::BindGroupEntry {
								binding: 0,
								resource: wgpu::BindingResource::TextureView(
									&self
										.meshes
										.get("bloom")
										.as_ref()
										.unwrap()
										.material
										.as_ref()
										.unwrap()
										.bind_groups_textures[3]
										.view,
								),
							},
							wgpu::BindGroupEntry {
								binding: 1,
								resource: wgpu::BindingResource::Sampler(
									&self.hdr_texture.sampler.as_ref().unwrap(),
								),
							},
						],
					});
		} else {
			unsafe { OK = true };
		}
	}

	pub fn input(&mut self, _event: &WindowEvent) -> bool {
		false
	}

	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		let output = self.context.surface.get_current_texture()?;
		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());
		let mut encoder =
			self.context
				.device
				.create_command_encoder(&wgpu::CommandEncoderDescriptor {
					label: Some("Render Encoder"),
				});

		bloom::render_pbr(self, &mut encoder);
		bloom::render_bloom(self, &mut encoder);

		{
			let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("final Render Pass"),
				color_attachments: &[wgpu::RenderPassColorAttachment {
					view: &view,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Clear(wgpu::Color {
							r: 0.1,
							g: 0.5,
							b: 0.7,
							a: 1.0,
						}),
						store: true,
					},
				}],
				depth_stencil_attachment: None,
			});
			render_pass.set_pipeline(&self.final_pipeline);
			render_pass.set_vertex_buffer(0, self.fullscreen_vertex_buffer.slice(..));
			render_pass.set_bind_group(0, &self.final_bind_group, &[]);
			render_pass.draw(0..6, 0..1);
		}

		// submit will accept anything that implements IntoIter
		self.context.queue.submit(std::iter::once(encoder.finish()));
		output.present();

		Ok(())
	}
}

pub fn generate_fullscreen_quad() -> Vec<Vertex> {
	let mut fullscreen_quad_data = Vec::with_capacity(6);
	fullscreen_quad_data.push(Vertex {
		position: uv::Vec3::new(-1.0, -1.0, 0.0),
		uv: uv::Vec2::new(0.0, 1.0),
		..Default::default()
	});
	fullscreen_quad_data.push(Vertex {
		position: uv::Vec3::new(1.0, -1.0, 0.0),
		uv: uv::Vec2::new(1.0, 1.0),
		..Default::default()
	});
	fullscreen_quad_data.push(Vertex {
		position: uv::Vec3::new(-1.0, 1.0, 0.0),
		uv: uv::Vec2::new(0.0, 0.0),
		..Default::default()
	});
	fullscreen_quad_data.push(Vertex {
		position: uv::Vec3::new(1.0, -1.0, 0.0),
		uv: uv::Vec2::new(1.0, 1.0),
		..Default::default()
	});
	fullscreen_quad_data.push(Vertex {
		position: uv::Vec3::new(1.0, 1.0, 0.0),
		uv: uv::Vec2::new(1.0, 0.0),
		..Default::default()
	});
	fullscreen_quad_data.push(Vertex {
		position: uv::Vec3::new(-1.0, 1.0, 0.0),
		uv: uv::Vec2::new(0.0, 0.0),
		..Default::default()
	});
	fullscreen_quad_data
}
