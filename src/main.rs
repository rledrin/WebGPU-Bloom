extern crate ultraviolet as uv;

mod bloom;
mod context;
mod renderer;

use winit::{
	event::VirtualKeyCode,
	event_loop::{ControlFlow, EventLoop},
	window::WindowBuilder,
};

use context::Context;
use renderer::Renderer;

fn main() {
	let event_loop = EventLoop::new();
	let window = WindowBuilder::new()
		.with_title("wgpu bloom")
		.with_inner_size(winit::dpi::LogicalSize::new(
			f64::from(1080),
			f64::from(720),
		))
		.build(&event_loop)
		.unwrap();
	let mut input = winit_input_helper::WinitInputHelper::new();

	let context = pollster::block_on(Context::new(
		window,
		Some(
			wgpu::Features::PUSH_CONSTANTS
				| wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
		),
		Some(wgpu::Limits {
			max_push_constant_size: 128,
			..Default::default()
		}),
	));
	let mut renderer = Renderer::new(context);

	let mut pbr_param = bloom::PbrParam {
		cam_pos: renderer.camera.position,
		metallic: 0.0,
		albedo: uv::Vec3::new(1.0, 0.0, 0.0),
		roughness: 0.2,
		emissive_color: uv::Vec3::new(0.0, 0.0, 0.0),
		ao: 0.01,
		light_position: uv::Vec3::new(-4.0, 5.0, -5.0),
		emissive_intensity: 0.0,
		light_color: uv::Vec3::new(25.0, 25.0, 25.0),
	};
	let mut bloom_threshold = 1.0f32;
	let mut bloom_knee = 0.2f32;
	let mut bloom_param = bloom::BloomParam {
		parameters: uv::Vec4::new(
			bloom_threshold,
			bloom_threshold - bloom_knee,
			bloom_knee * 2.0f32,
			0.25f32 / bloom_knee,
		), // (x) threshold, (y) threshold - knee, (z) knee * 2, (w) 0.25 / knee
		combine_constant: 0.68,
	};
	let mut bloom_intensity = 1.0f32;

	let start_time = std::time::Instant::now();

	event_loop.run(move |event, _, control_flow| {
		renderer
			.gui
			.platform
			.update_time(start_time.elapsed().as_secs_f64());
		renderer.gui.platform.handle_event(&event);

		if input.update(&event) {
			if input.key_released(VirtualKeyCode::Escape) || input.quit() {
				*control_flow = ControlFlow::Exit;
				return;
			}
			if let Some(physical_size) = input.window_resized() {
				renderer.resize(physical_size);
			}

			renderer.gui.platform.begin_frame();

			let (pbr, final_composite, bloom) = renderer::gui::create_gui(
				&renderer.gui.platform.context(),
				&mut pbr_param,
				&mut bloom_threshold,
				&mut bloom_knee,
				&mut bloom_param,
				&mut bloom_intensity,
			);
			if pbr {
				renderer
					.meshes
					.get_mut("pbr")
					.unwrap()
					.material
					.as_mut()
					.unwrap()
					.copy_to_buffer(
						&renderer.context.device,
						&renderer.context.queue,
						1,
						0,
						vec![pbr_param.clone()],
					)
			}
			if bloom {
				renderer
					.meshes
					.get_mut("bloom")
					.unwrap()
					.material
					.as_mut()
					.unwrap()
					.copy_to_buffer(
						&renderer.context.device,
						&renderer.context.queue,
						0,
						0,
						vec![bloom_param.clone()],
					)
			}
			if final_composite {
				renderer.final_buffer.copy_to_buffer(
					&renderer.context.device,
					&renderer.context.queue,
					0,
					vec![bloom_intensity],
				)
			}
			// println!(
			// 	"(pbr_param: {}, final_composite: {}, bloom_param: {})",
			// 	pbr_param, final_composite, bloom_param
			// );

			match renderer.render(true) {
				Ok(_) => {}
				// Reconfigure the surface if lost
				Err(wgpu::SurfaceError::Lost) => renderer.resize(renderer.context.size),
				// The system is out of memory, we should probably quit
				Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
				// All other errors (Outdated, Timeout) should be resolved by the next frame
				Err(e) => eprintln!("{:?}", e),
			}
			renderer.resized = false;

			// // query the change in mouse this update
			// let mouse_diff = input.mouse_diff();
			// if mouse_diff != (0.0, 0.0) {
			// 	println!("The mouse diff is: {:?}", mouse_diff);
			// 	println!("The mouse position is: {:?}", input.mouse());
			// }
		}
	});
}
