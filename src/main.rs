mod bloom;
mod context;
mod renderer;

use winit::{
	event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
	event_loop::{ControlFlow, EventLoop},
	window::WindowBuilder,
};

use context::Context;
use renderer::Renderer;

fn main() {
	let event_loop = EventLoop::new();
	let window = WindowBuilder::new()
		.with_title("window title")
		.with_inner_size(winit::dpi::LogicalSize::new(
			f64::from(1080),
			f64::from(720),
		))
		.build(&event_loop)
		.unwrap();
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
		match event {
			Event::WindowEvent {
				ref event,
				window_id,
			} if window_id == window.id() => {
				if !renderer.input(event) {
					match event {
						WindowEvent::CloseRequested
						| WindowEvent::KeyboardInput {
							input:
								KeyboardInput {
									state: ElementState::Pressed,
									virtual_keycode: Some(VirtualKeyCode::Escape),
									..
								},
							..
						} => *control_flow = ControlFlow::Exit,
						WindowEvent::Resized(physical_size) => {
							renderer.resize(*physical_size);
						}
						WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
							// new_inner_size is &mut so w have to dereference it twice
							renderer.resize(**new_inner_size);
						}
						_ => {}
					}
				}
			}
			Event::RedrawRequested(window_id) if window_id == window.id() => {
				// renderer.update();
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
			}
			Event::MainEventsCleared => {
				// RedrawRequested will only trigger once, unless we manually
				// request it.
				window.request_redraw();
			}
			_ => {}
		}
	});
}
