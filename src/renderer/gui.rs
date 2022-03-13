use egui_wgpu_backend::RenderPass;
use egui_winit_platform::{Platform, PlatformDescriptor};

use crate::bloom;

pub struct Gui {
	pub platform: Platform,
	pub render_pass: RenderPass,
}

impl Gui {
	pub fn new(
		window: &winit::window::Window,
		device: &wgpu::Device,
		surface_format: wgpu::TextureFormat,
		msaa_samples: u32,
	) -> Self {
		let size = window.inner_size();

		let platform = Platform::new(PlatformDescriptor {
			physical_width: size.width,
			physical_height: size.height,
			scale_factor: window.scale_factor(),
			font_definitions: egui::FontDefinitions::default(),
			style: Default::default(),
		});

		let render_pass = RenderPass::new(device, surface_format, msaa_samples);

		Gui {
			platform,
			render_pass,
		}
	}
}

pub fn create_gui(
	ctx: &egui::Context,
	pbr_param: &mut bloom::PbrParam,
	bloom_threshold: &mut f32,
	bloom_knee: &mut f32,
	bloom_param: &mut bloom::BloomParam,
	bloom_intensity: &mut f32,
) -> (bool, bool, bool) {
	egui::Window::new("Parameters")
		.resizable(false)
		.auto_sized()
		.show(ctx, |ui| {
			let mut pbr_param_ret = false;
			let mut final_composite_ret = false;
			let mut bloom_param_ret = false;
			let mut albedo = [0.0f32; 3];
			let mut emissive_color = [0.0f32; 3];

			albedo[0] = pbr_param.albedo.x;
			albedo[1] = pbr_param.albedo.y;
			albedo[2] = pbr_param.albedo.z;

			emissive_color[0] = pbr_param.emissive_color.x;
			emissive_color[1] = pbr_param.emissive_color.y;
			emissive_color[2] = pbr_param.emissive_color.z;

			pbr_param_ret |= ui
				.add(
					egui::Slider::new(&mut pbr_param.roughness, 0.0..=1.0)
						.step_by(0.001)
						.smart_aim(false)
						.text("Roughness"),
				)
				.changed();
			pbr_param_ret |= ui
				.add(
					egui::Slider::new(&mut pbr_param.metallic, 0.0..=1.0)
						.step_by(0.001)
						.smart_aim(false)
						.text("Metallic"),
				)
				.changed();
			pbr_param_ret |= ui
				.add(
					egui::Slider::new(&mut pbr_param.ao, 0.0..=1.0)
						.step_by(0.001)
						.smart_aim(false)
						.text("Ao"),
				)
				.changed();
			pbr_param_ret |= ui
				.horizontal(|ui| {
					let ret = ui.color_edit_button_rgb(&mut albedo);
					ui.label("Albedo");
					ret.changed()
				})
				.inner;
			pbr_param_ret |= ui
				.horizontal(|ui| {
					let ret = ui.color_edit_button_rgb(&mut emissive_color);
					ui.label("Emissive");
					ret.changed()
				})
				.inner;
			pbr_param_ret |= ui
				.add(
					egui::Slider::new(&mut pbr_param.emissive_intensity, 0.0..=20.0)
						.step_by(0.001)
						.smart_aim(false)
						.text("Emissive intensity"),
				)
				.changed();
			final_composite_ret |= ui
				.add(
					egui::Slider::new(bloom_intensity, 1.0..=100.0)
						.step_by(0.001)
						.smart_aim(false)
						.text("Bloom intensity"),
				)
				.changed();
			bloom_param_ret |= ui
				.add(
					egui::Slider::new(bloom_threshold, 0.0..=50.0)
						.step_by(0.001)
						.smart_aim(false)
						.text("Bloom threshold"),
				)
				.changed();
			bloom_param_ret |= ui
				.add(
					egui::Slider::new(bloom_knee, 0.01..=50.0)
						.step_by(0.001)
						.smart_aim(false)
						.text("Bloom knee"),
				)
				.changed();

			pbr_param.albedo.x = albedo[0];
			pbr_param.albedo.y = albedo[1];
			pbr_param.albedo.z = albedo[2];

			pbr_param.emissive_color.x = emissive_color[0];
			pbr_param.emissive_color.y = emissive_color[1];
			pbr_param.emissive_color.z = emissive_color[2];

			if bloom_param_ret {
				bloom_param.parameters = uv::Vec4::new(
					*bloom_threshold,
					*bloom_threshold - *bloom_knee,
					*bloom_knee * 2.0f32,
					0.25f32 / *bloom_knee,
				); // (x) threshold, (y) threshold - knee, (z) knee * 2, (w) 0.25 / knee
			}

			(pbr_param_ret, final_composite_ret, bloom_param_ret)
		})
		.unwrap()
		.inner
		.unwrap()
}
