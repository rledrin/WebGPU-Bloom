extern crate ultraviolet as uv;

pub struct PerspectiveCamera {
	pub position: uv::Vec3,
	pub look_at: uv::Vec3,
	pub up: uv::Vec3,
	pub fov: f32,
	pub aspect_ratio: f32,
	pub near: f32,
	pub far: f32,
	pub view: uv::Mat4,
	pub proj: uv::Mat4,
	pub view_proj: uv::Mat4,
}

pub struct OrthographicCamera {
	pub position: uv::Vec3,
	pub look_at: uv::Vec3,
	pub up: uv::Vec3,
	pub near: f32,
	pub far: f32,
	pub top: f32,
	pub bottom: f32,
	pub left: f32,
	pub right: f32,
	pub view: uv::Mat4,
	pub proj: uv::Mat4,
	pub view_proj: uv::Mat4,
}

impl PerspectiveCamera {
	#![allow(unused)]
	pub fn new(
		position: uv::Vec3,
		look_at: uv::Vec3,
		fov: f32,
		aspect_ratio: f32,
		near: f32,
		far: f32,
	) -> Self {
		let view = uv::Mat4::look_at(position, look_at, uv::Vec3::unit_y());
		let proj = uv::projection::perspective_wgpu_dx(fov, aspect_ratio, near, far);

		PerspectiveCamera {
			position,
			look_at,
			up: uv::Vec3::unit_y(),
			fov,
			aspect_ratio,
			near,
			far,
			view,
			proj,
			view_proj: proj * view,
		}
	}

	pub fn recreate_matrices(&mut self) {
		let view = uv::Mat4::look_at(self.position, self.look_at, self.up);
		let proj =
			uv::projection::perspective_wgpu_dx(self.fov, self.aspect_ratio, self.near, self.far);

		self.view = view;
		self.proj = proj;
		self.view_proj = view * proj;
	}
}

impl OrthographicCamera {
	#![allow(unused)]
	pub fn new(
		position: uv::Vec3,
		look_at: uv::Vec3,
		top: f32,
		bottom: f32,
		left: f32,
		right: f32,
		near: f32,
		far: f32,
	) -> Self {
		let view = uv::Mat4::look_at(position, look_at, uv::Vec3::unit_y());
		let proj = uv::projection::orthographic_wgpu_dx(left, right, bottom, top, near, far);

		OrthographicCamera {
			position,
			look_at,
			up: uv::Vec3::unit_y(),
			top,
			bottom,
			left,
			right,
			near,
			far,
			view,
			proj,
			view_proj: view * proj,
		}
	}

	pub fn recreate_matrices(&mut self) {
		let view = uv::Mat4::look_at(self.position, self.look_at, self.up);
		let proj = uv::projection::orthographic_wgpu_dx(
			self.left,
			self.right,
			self.bottom,
			self.top,
			self.near,
			self.far,
		);

		self.view = view;
		self.proj = proj;
		self.view_proj = view * proj;
	}
}
