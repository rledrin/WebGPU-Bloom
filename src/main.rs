mod bloom;
mod context;
mod renderer;

use epi::App;
use winit::{
	event::VirtualKeyCode,
	event_loop::{ControlFlow, EventLoop},
	window::WindowBuilder,
};

use context::Context;
use renderer::Renderer;

fn main() {
	let event_loop = EventLoop::with_user_event();
	// let event_loop = EventLoop::new();
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

	// Display the demo application that ships with egui.
	// let mut demo_app = egui_demo_lib::WrapApp::default();
	// let mut demo_app = egui_demo_lib::WrapApp::;

	let start_time = std::time::Instant::now();
	let mut previous_frame_time = None;
	let repaint_signal = std::sync::Arc::new(renderer::gui::ExampleRepaintSignal(
		std::sync::Mutex::new(event_loop.create_proxy()),
	));

	let mut slider_val = 0.0f32;
	let mut color = [0.0f32; 3];

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

			let egui_start = std::time::Instant::now();
			renderer.gui.platform.begin_frame();

			// let app_output = epi::backend::AppOutput::default();
			// let mut frame = epi::Frame::new(epi::backend::FrameData {
			// 	info: epi::IntegrationInfo {
			// 		name: "egui_example",
			// 		web_info: None,
			// 		cpu_usage: previous_frame_time,
			// 		native_pixels_per_point: Some(renderer.context.window.scale_factor() as _),
			// 		prefer_dark_mode: None,
			// 	},
			// 	output: app_output,
			// 	repaint_signal: repaint_signal.clone(),
			// });
			// // Draw the demo application.
			// demo_app.update(&renderer.gui.platform.context(), &mut frame);

			egui::Window::new("Parameters")
				.resizable(false)
				// .auto_sized()
				.show(&renderer.gui.platform.context(), |ui| {
					ui.add(
						egui::Slider::new(&mut slider_val, 0.0..=1.0)
							.step_by(0.001)
							.smart_aim(false)
							.text("Roughness"),
					);
					ui.add(
						egui::Slider::new(&mut slider_val, 0.0..=1.0)
							.step_by(0.001)
							.smart_aim(false)
							.text("Metallic"),
					);
					ui.add(
						egui::Slider::new(&mut slider_val, 0.0..=1.0)
							.step_by(0.001)
							.smart_aim(false)
							.text("Ao"),
					);
					ui.horizontal(|ui| {
						ui.color_edit_button_rgb(&mut color);
						ui.label("Albedo");
					});
					ui.horizontal(|ui| {
						ui.color_edit_button_rgb(&mut color);
						ui.label("Emissive");
					});
					ui.add(
						egui::Slider::new(&mut slider_val, 0.0..=1.0)
							.step_by(0.001)
							.smart_aim(false)
							.text("Emissive intensity"),
					);
					ui.add(
						egui::Slider::new(&mut slider_val, 0.0..=1.0)
							.step_by(0.001)
							.smart_aim(false)
							.text("Bloom intensity"),
					);
					ui.add(
						egui::Slider::new(&mut slider_val, 0.0..=1.0)
							.step_by(0.001)
							.smart_aim(false)
							.text("Bloom threshold"),
					);
				})
				.unwrap();

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

			let frame_time = (std::time::Instant::now() - egui_start).as_secs_f64() as f32;
			previous_frame_time = Some(frame_time);

			// // query the change in mouse this update
			// let mouse_diff = input.mouse_diff();
			// if mouse_diff != (0.0, 0.0) {
			// 	println!("The mouse diff is: {:?}", mouse_diff);
			// 	println!("The mouse position is: {:?}", input.mouse());
			// }
		}
	});
}
