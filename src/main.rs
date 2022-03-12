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
		&window,
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

	event_loop.run(move |event, _, control_flow| {
		if input.update(&event) {
			if input.key_released(VirtualKeyCode::Escape) || input.quit() {
				*control_flow = ControlFlow::Exit;
				return;
			}
			if let Some(physical_size) = input.window_resized() {
				renderer.resize(physical_size);
			}

			match renderer.render() {
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
