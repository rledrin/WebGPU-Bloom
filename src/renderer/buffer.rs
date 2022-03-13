use wgpu::util::DeviceExt;

pub struct Buffer {
	pub buffer: wgpu::Buffer,
	pub size: std::num::NonZeroU64,
	pub usage: wgpu::BufferUsages,
}

impl Buffer {
	#![allow(unused)]
	pub fn new<T>(
		device: &wgpu::Device,
		label: Option<&str>,
		contents: Vec<T>,
		usage: wgpu::BufferUsages,
	) -> Self {
		let contents = unsafe { contents.align_to::<u8>().1 };
		let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label,
			contents,
			usage,
		});
		Buffer {
			buffer,
			size: std::num::NonZeroU64::new(contents.len() as u64).unwrap(),
			usage,
		}
	}

	pub fn new_empty(
		device: &wgpu::Device,
		label: Option<&str>,
		size: u64,
		usage: wgpu::BufferUsages,
		mapped_at_creation: bool,
	) -> Self {
		let buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label,
			size,
			usage,
			mapped_at_creation,
		});
		Buffer {
			buffer,
			size: std::num::NonZeroU64::new(size).unwrap(),
			usage,
		}
	}

	pub fn copy_to_buffer<T>(
		&mut self,
		device: &wgpu::Device,
		queue: &wgpu::Queue,
		offset: u64,
		data_vec: Vec<T>,
	) {
		let data = unsafe { data_vec.align_to::<u8>().1 };
		let copy_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("copy buffer"),
			contents: data,
			usage: wgpu::BufferUsages::COPY_SRC,
		});
		let mut command_encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("copy Encoder"),
		});
		command_encoder.copy_buffer_to_buffer(
			&copy_buffer,
			0,
			&self.buffer,
			offset,
			data.len() as u64,
		);
		queue.submit(std::iter::once(command_encoder.finish()));
	}
}
