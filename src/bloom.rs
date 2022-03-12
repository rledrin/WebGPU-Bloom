extern crate ultraviolet as uv;

use wgpu::util::DeviceExt;

use super::renderer::mesh;
use crate::renderer::{
	mesh::Material,
	texture::{self, Texture},
	Renderer, Vertex,
};

pub const BLOOM_MIP_COUNT: usize = 7;

#[repr(C, align(16))]
pub struct PbrParam {
	pub cam_pos: uv::Vec3,
	pub metallic: f32,
	pub albedo: uv::Vec3,
	pub roughness: f32,
	pub emissive_color: uv::Vec3,
	pub ao: f32,
	pub light_position: uv::Vec3,
	pub emissive_intensity: f32,
	pub light_color: uv::Vec3,
}

#[repr(C, align(16))]
struct BloomParam {
	parameters: uv::Vec4, // (x) threshold, (y) threshold - knee, (z) knee * 2, (w) 0.25 / knee
	intensity: f32,
	combine_constant: f32,
}

fn load_sphere() -> Vec<Vertex> {
	let input = std::io::BufReader::new(std::io::Cursor::new(include_bytes!(
		"../assets/obj/uv_sphere.obj"
	)));
	let vertices: obj::Obj<obj::Vertex, u32> = obj::load_obj(input).unwrap();
	let mut vertex_data = Vec::<Vertex>::with_capacity(vertices.indices.len());

	for ind in vertices.indices.into_iter() {
		let position = vertices.vertices[ind as usize].position;
		let normal = vertices.vertices[ind as usize].normal;
		vertex_data.push(Vertex {
			position: uv::Vec3::new(position[0], position[1], position[2]),
			normal: uv::Vec3::new(normal[0], normal[1], normal[2]),
			uv: uv::Vec2::zero(),
		});
	}

	vertex_data
}

pub fn init_pbr(renderer: &Renderer) -> mesh::Mesh {
	let param = vec![PbrParam {
		cam_pos: renderer.camera.position,
		metallic: 0.0,
		albedo: uv::Vec3::new(1.0, 0.0, 0.0),
		roughness: 0.2,
		emissive_color: uv::Vec3::zero(),
		ao: 0.01,
		light_position: uv::Vec3::new(-4.0, 5.0, -5.0),
		emissive_intensity: 0.0,
		light_color: uv::Vec3::new(25.0, 25.0, 25.0),
	}];

	let vertex_data = load_sphere();

	let mut pbr_mesh = mesh::Mesh::new::<u32>(
		&renderer.context.device,
		Some("pbrMesh"),
		&vertex_data,
		None,
		None,
	);

	let mut matrices = Vec::with_capacity(2);
	matrices.push(renderer.camera.view_proj);
	matrices.push(pbr_mesh.model);

	let content = unsafe { matrices.align_to::<u8>().1 };
	let matrix_buffer =
		renderer
			.context
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("pbr Matix buffer"),
				contents: content,
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			});

	let content = unsafe { param.align_to::<u8>().1 };
	let param_buffer =
		renderer
			.context
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("pbr param buffer"),
				contents: content,
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			});

	let mut pbr_mat = mesh::Material::new(1, 0);
	pbr_mat.add_bind_group(
		&renderer.context.device,
		Some("matrices and PbrParam bind group"),
		vec![
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::VERTEX,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: std::num::NonZeroU64::new(
						std::mem::size_of::<ultraviolet::Mat4>() as u64 * 2u64,
					),
				},
				count: None,
			},
			wgpu::BindGroupLayoutEntry {
				binding: 1,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Buffer {
					ty: wgpu::BufferBindingType::Uniform,
					has_dynamic_offset: false,
					min_binding_size: std::num::NonZeroU64::new(
						std::mem::size_of::<PbrParam>() as u64
					),
				},
				count: None,
			},
		],
		vec![
			wgpu::BindingResource::Buffer(wgpu::BufferBinding {
				buffer: &matrix_buffer,
				offset: 0,
				size: std::num::NonZeroU64::new((std::mem::size_of::<uv::Mat4>() * 2) as u64),
			}),
			wgpu::BindingResource::Buffer(wgpu::BufferBinding {
				buffer: &param_buffer,
				offset: 0,
				size: std::num::NonZeroU64::new(std::mem::size_of::<PbrParam>() as u64),
			}),
		],
	);
	pbr_mat.bind_groups_buffers.push(matrix_buffer);
	pbr_mat.bind_groups_buffers.push(param_buffer);

	pbr_mat.set_render_pipeline(
		&renderer.context.device,
		Some("pbr pipeline"),
		wgpu::include_wgsl!("../assets/shaders/pbr.wgsl"),
		renderer.hdr_texture.format,
		Some(renderer.depth_texture.format),
	);

	pbr_mesh.material = Some(pbr_mat);
	pbr_mesh
}

pub fn render_pbr(renderer: &Renderer, encoder: &mut wgpu::CommandEncoder) {
	let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
		label: Some("hdr Render Pass"),
		color_attachments: &[wgpu::RenderPassColorAttachment {
			view: &renderer.hdr_texture.view,
			resolve_target: None,
			ops: wgpu::Operations {
				load: wgpu::LoadOp::Clear(wgpu::Color {
					r: 0.0,
					g: 0.0,
					b: 0.0,
					a: 1.0,
				}),
				store: true,
			},
		}],
		depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
			view: &renderer.depth_texture.view,
			depth_ops: Some(wgpu::Operations {
				load: wgpu::LoadOp::Clear(1.0),
				store: false,
			}),
			stencil_ops: None,
		}),
	});
	render_pass.set_pipeline(
		renderer
			.meshes
			.get("pbr")
			.unwrap()
			.material
			.as_ref()
			.unwrap()
			.render_pipeline
			.as_ref()
			.unwrap(),
	);
	render_pass.set_vertex_buffer(
		0,
		renderer
			.meshes
			.get("pbr")
			.unwrap()
			.vertex_buffer
			.as_ref()
			.unwrap()
			.slice(..),
	);
	render_pass.set_bind_group(
		0,
		&renderer
			.meshes
			.get("pbr")
			.unwrap()
			.material
			.as_ref()
			.unwrap()
			.bind_group[0],
		&[],
	);
	render_pass.draw(0..renderer.meshes.get("pbr").unwrap().draw_count, 0..1);
}

fn create_bloom_bind_group(
	renderer: &Renderer,
	bloom_mat: &Material,
	output_image: &wgpu::TextureView,
	input_image: &wgpu::TextureView,
	bloom_image: &wgpu::TextureView,
	sampler: &wgpu::Sampler,
	parameters: &wgpu::Buffer,
) -> wgpu::BindGroup {
	let parameters = wgpu::BufferBinding {
		buffer: parameters,
		offset: 0,
		size: std::num::NonZeroU64::new(std::mem::size_of::<BloomParam>() as u64),
	};

	let bind_group = renderer
		.context
		.device
		.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("Bloom Bind Group"),
			layout: &bloom_mat.bind_group_layout[0],
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(output_image),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::TextureView(input_image),
				},
				wgpu::BindGroupEntry {
					binding: 2,
					resource: wgpu::BindingResource::TextureView(bloom_image),
				},
				wgpu::BindGroupEntry {
					binding: 3,
					resource: wgpu::BindingResource::Sampler(sampler),
				},
				wgpu::BindGroupEntry {
					binding: 4,
					resource: wgpu::BindingResource::Buffer(parameters),
				},
			],
		});

	bind_group
}

fn set_all_bind_group(renderer: &Renderer, bloom_mat: &mut Material) {
	bloom_mat.bind_group.clear();
	// Prefilter bind group
	bloom_mat.bind_group.push(create_bloom_bind_group(
		renderer,
		bloom_mat,
		&bloom_mat.bind_groups_textures[0].mip_view[0],
		&renderer.hdr_texture.view,
		&renderer.hdr_texture.view,
		renderer.hdr_texture.sampler.as_ref().unwrap(),
		&bloom_mat.bind_groups_buffers[0],
	));

	// Downsample bind groups
	for i in 1..BLOOM_MIP_COUNT {
		// Ping
		bloom_mat.bind_group.push(create_bloom_bind_group(
			renderer,
			bloom_mat,
			&bloom_mat.bind_groups_textures[1].mip_view[i],
			&bloom_mat.bind_groups_textures[0].view,
			&renderer.hdr_texture.view,
			renderer.hdr_texture.sampler.as_ref().unwrap(),
			&bloom_mat.bind_groups_buffers[0],
		));

		// Pong
		bloom_mat.bind_group.push(create_bloom_bind_group(
			renderer,
			bloom_mat,
			&bloom_mat.bind_groups_textures[0].mip_view[i],
			&bloom_mat.bind_groups_textures[1].view,
			&renderer.hdr_texture.view,
			renderer.hdr_texture.sampler.as_ref().unwrap(),
			&bloom_mat.bind_groups_buffers[0],
		));
	}

	// First Upsample
	bloom_mat.bind_group.push(create_bloom_bind_group(
		renderer,
		bloom_mat,
		&bloom_mat.bind_groups_textures[2].mip_view[BLOOM_MIP_COUNT - 1],
		&bloom_mat.bind_groups_textures[0].view,
		&renderer.hdr_texture.view,
		renderer.hdr_texture.sampler.as_ref().unwrap(),
		&bloom_mat.bind_groups_buffers[0],
	));

	let mut o = true;
	//Upsample
	for i in (0..=BLOOM_MIP_COUNT - 2).rev() {
		if o {
			bloom_mat.bind_group.push(create_bloom_bind_group(
				renderer,
				bloom_mat,
				&bloom_mat.bind_groups_textures[1].mip_view[i],
				&bloom_mat.bind_groups_textures[0].view,
				&bloom_mat.bind_groups_textures[2].view,
				renderer.hdr_texture.sampler.as_ref().unwrap(),
				&bloom_mat.bind_groups_buffers[0],
			));
			o = false;
		} else {
			bloom_mat.bind_group.push(create_bloom_bind_group(
				renderer,
				bloom_mat,
				&bloom_mat.bind_groups_textures[2].mip_view[i],
				&bloom_mat.bind_groups_textures[0].view,
				&bloom_mat.bind_groups_textures[1].view,
				renderer.hdr_texture.sampler.as_ref().unwrap(),
				&bloom_mat.bind_groups_buffers[0],
			));
			o = true;
		}
	}
}

pub fn init_bloom(renderer: &mut Renderer) -> mesh::Mesh {
	let mut bloom_mesh = mesh::Mesh::new::<u32>(
		&renderer.context.device,
		Some("pbrMesh"),
		&vec![],
		None,
		None,
	);

	let mut bloom_mat = mesh::Material::new(BLOOM_MIP_COUNT * 2 + 2, 1);

	let bind_group_layout_entries = vec![
		wgpu::BindGroupLayoutEntry {
			binding: 0,
			visibility: wgpu::ShaderStages::COMPUTE,
			ty: wgpu::BindingType::StorageTexture {
				access: wgpu::StorageTextureAccess::WriteOnly,
				format: wgpu::TextureFormat::Rgba16Float,
				view_dimension: wgpu::TextureViewDimension::D2,
			},
			count: None,
		},
		wgpu::BindGroupLayoutEntry {
			binding: 1,
			visibility: wgpu::ShaderStages::COMPUTE,
			ty: wgpu::BindingType::Texture {
				sample_type: wgpu::TextureSampleType::Float { filterable: true },
				view_dimension: wgpu::TextureViewDimension::D2,
				multisampled: false,
			},
			count: None,
		},
		wgpu::BindGroupLayoutEntry {
			binding: 2,
			visibility: wgpu::ShaderStages::COMPUTE,
			ty: wgpu::BindingType::Texture {
				sample_type: wgpu::TextureSampleType::Float { filterable: true },
				view_dimension: wgpu::TextureViewDimension::D2,
				multisampled: false,
			},
			count: None,
		},
		wgpu::BindGroupLayoutEntry {
			binding: 3,
			visibility: wgpu::ShaderStages::COMPUTE,
			ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
			count: None,
		},
		wgpu::BindGroupLayoutEntry {
			binding: 4,
			visibility: wgpu::ShaderStages::COMPUTE,
			ty: wgpu::BindingType::Buffer {
				ty: wgpu::BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: std::num::NonZeroU64::new(
					std::mem::size_of::<BloomParam>() as u64
				),
			},
			count: None,
		},
	];

	bloom_mat.add_bind_group(
		&renderer.context.device,
		Some("bloom bind group layout"),
		bind_group_layout_entries,
		vec![],
	);

	bloom_mat.add_push_constant(wgpu::ShaderStages::COMPUTE, 0..4);

	bloom_mat.bind_groups_textures.push(texture::Texture::new(
		&renderer.context.device,
		Some("bloom downsampler image 0"),
		renderer.context.size.width / 2,
		renderer.context.size.height / 2,
		BLOOM_MIP_COUNT as u32,
		1,
		wgpu::TextureDimension::D2,
		wgpu::TextureFormat::Rgba16Float,
		wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
		wgpu::TextureAspect::All,
	));
	bloom_mat.bind_groups_textures.push(texture::Texture::new(
		&renderer.context.device,
		Some("bloom downsampler image 1"),
		renderer.context.size.width / 2,
		renderer.context.size.height / 2,
		BLOOM_MIP_COUNT as u32,
		1,
		wgpu::TextureDimension::D2,
		wgpu::TextureFormat::Rgba16Float,
		wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
		wgpu::TextureAspect::All,
	));
	bloom_mat.bind_groups_textures.push(texture::Texture::new(
		&renderer.context.device,
		Some("bloom upsampler image"),
		renderer.context.size.width / 2,
		renderer.context.size.height / 2,
		BLOOM_MIP_COUNT as u32,
		1,
		wgpu::TextureDimension::D2,
		wgpu::TextureFormat::Rgba16Float,
		wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
		wgpu::TextureAspect::All,
	));

	let bloom_threshold = 1.0f32;
	let bloom_knee = 0.2f32;

	bloom_mat
		.bind_groups_buffers
		.push(
			renderer
				.context
				.device
				.create_buffer_init(&wgpu::util::BufferInitDescriptor {
					label: Some("bloom parameters buffer"),
					contents: unsafe {
						vec![BloomParam {
							parameters: uv::Vec4::new(
								bloom_threshold,
								bloom_threshold - bloom_knee,
								bloom_knee * 2.0f32,
								0.25f32 / bloom_knee,
							),
							intensity: 1.0,
							combine_constant: 0.68,
						}]
						.align_to::<u8>()
						.1
					},
					usage: wgpu::BufferUsages::UNIFORM,
				}),
		);
	bloom_mat.set_compute_pipeline(
		&renderer.context.device,
		Some("bloom compute pipeline"),
		wgpu::include_wgsl!("../assets/shaders/bloom.wgsl"),
	);

	bloom_mesh.material = Some(bloom_mat);

	set_all_bind_group(renderer, bloom_mesh.material.as_mut().unwrap());

	bloom_mesh
}

fn get_mip_size(current_mip: usize, texture: &Texture) -> wgpu::Extent3d {
	let mut width = texture.size.width;
	let mut height = texture.size.height;
	for _ in 0..current_mip {
		width /= 2;
		height /= 2;
	}
	wgpu::Extent3d {
		width,
		height,
		depth_or_array_layers: 1,
	}
}

pub fn render_bloom(renderer: &mut Renderer, encoder: &mut wgpu::CommandEncoder) {
	const MODE_PREFILTER: u32 = 0;
	const MODE_DOWNSAMPLE: u32 = 1;
	const MODE_UPSAMPLE_FIRST: u32 = 2;
	const MODE_UPSAMPLE: u32 = 3;

	struct PushConstant {
		mode_lod: u32,
	}

	let mut bloom_mat =
		std::mem::take(&mut renderer.meshes.get_mut("bloom").unwrap().material).unwrap();

	if renderer.resized {
		set_all_bind_group(renderer, &mut bloom_mat);
	}

	let mut bind_group_index = 0usize;

	let mut pc = Vec::with_capacity(1);
	pc.push(PushConstant { mode_lod: 0 });
	let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
		label: Some("bloom Compute Pass"),
	});
	compute_pass.set_pipeline(bloom_mat.compute_pipeline.as_ref().unwrap());

	// * PreFilter
	pc[0].mode_lod = MODE_PREFILTER << 16 | 0;
	let pc_data = unsafe { pc.align_to::<u8>().1 };
	compute_pass.set_push_constants(0, pc_data);

	compute_pass.set_bind_group(0, &bloom_mat.bind_group[bind_group_index], &[]);
	bind_group_index += 1;
	let mip_size = get_mip_size(0, &bloom_mat.bind_groups_textures[0]);
	compute_pass.dispatch(mip_size.width / 8 + 1, mip_size.height / 4 + 1, 1);

	// * Downsample
	for i in 1..BLOOM_MIP_COUNT {
		let mip_size = get_mip_size(i, &bloom_mat.bind_groups_textures[0]);

		// * Ping
		pc[0].mode_lod = MODE_DOWNSAMPLE << 16 | ((i - 1) as u32);
		let pc_data = unsafe { pc.align_to::<u8>().1 };
		compute_pass.set_push_constants(0, pc_data);
		compute_pass.set_bind_group(0, &bloom_mat.bind_group[bind_group_index], &[]);
		bind_group_index += 1;
		compute_pass.dispatch(mip_size.width / 8 + 1, mip_size.height / 4 + 1, 1);

		// * Pong
		pc[0].mode_lod = MODE_DOWNSAMPLE << 16 | (i as u32);
		let pc_data = unsafe { pc.align_to::<u8>().1 };
		compute_pass.set_push_constants(0, pc_data);
		compute_pass.set_bind_group(0, &bloom_mat.bind_group[bind_group_index], &[]);
		bind_group_index += 1;
		compute_pass.dispatch(mip_size.width / 8 + 1, mip_size.height / 4 + 1, 1);
	}

	// * Frist Upsample
	pc[0].mode_lod = MODE_UPSAMPLE_FIRST << 16 | ((BLOOM_MIP_COUNT - 2) as u32);
	let pc_data = unsafe { pc.align_to::<u8>().1 };
	compute_pass.set_push_constants(0, pc_data);
	compute_pass.set_bind_group(0, &bloom_mat.bind_group[bind_group_index], &[]);
	bind_group_index += 1;
	let mip_size = get_mip_size(BLOOM_MIP_COUNT - 1, &bloom_mat.bind_groups_textures[2]);
	compute_pass.dispatch(mip_size.width / 8 + 1, mip_size.height / 4 + 1, 1);

	// * Upsample
	for i in (0..=BLOOM_MIP_COUNT - 2).rev() {
		let mip_size = get_mip_size(i, &bloom_mat.bind_groups_textures[2]);
		pc[0].mode_lod = MODE_UPSAMPLE << 16 | ((i) as u32);
		let pc_data = unsafe { pc.align_to::<u8>().1 };
		compute_pass.set_push_constants(0, pc_data);
		compute_pass.set_bind_group(0, &bloom_mat.bind_group[bind_group_index], &[]);
		bind_group_index += 1;
		compute_pass.dispatch(mip_size.width / 8 + 1, mip_size.height / 4 + 1, 1);
	}

	drop(compute_pass);
	renderer.meshes.get_mut("bloom").unwrap().material = Some(bloom_mat);
}
