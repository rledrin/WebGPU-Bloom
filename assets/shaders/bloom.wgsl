// Compute Shader

let BLOOM_MIP_COUNT: i32 = 7;

let MODE_PREFILTER: u32 = 0u;
let MODE_DOWNSAMPLE: u32 = 1u;
let MODE_UPSAMPLE_FIRST: u32 = 2u;
let MODE_UPSAMPLE: u32 = 3u;

let EPSILON: f32 = 1.0e-4;

struct bloom_param {
	parameters: vec4<f32>; // (x) threshold, (y) threshold - knee, (z) knee * 2, (w) 0.25 / knee
	combine_constant: f32;
};

[[group(0), binding(0)]] var output_texture: texture_storage_2d<rgba16float, write>;
[[group(0), binding(1)]] var input_texture: texture_2d<f32>;
[[group(0), binding(2)]] var bloom_texture: texture_2d<f32>;
[[group(0), binding(3)]] var samp: sampler;
[[group(0), binding(4)]] var<uniform> param: bloom_param;

struct PushConstants {
	mode_lod: u32;
};
var<push_constant> pc: PushConstants;



// Quadratic color thresholding
// curve = (threshold - knee, knee * 2, 0.25 / knee)
fn QuadraticThreshold(color: vec4<f32>, threshold: f32, curve: vec3<f32>) -> vec4<f32>
{
	// Maximum pixel brightness
	let brightness = max(max(color.r, color.g), color.b);
	// Quadratic curve
	var rq: f32 = clamp(brightness - curve.x, 0.0, curve.y);
	rq = curve.z * (rq * rq);
	let ret_color = color * max(rq, brightness - threshold) / max(brightness, EPSILON);
	return ret_color;
}

fn Prefilter(color: vec4<f32>, uv: vec2<f32>) -> vec4<f32>
{
	let clamp_value = 20.0;
	var color: vec4<f32> = min(vec4<f32>(clamp_value), color);
	color = QuadraticThreshold(color, param.parameters.x, param.parameters.yzw);
	return color;
}

fn DownsampleBox13(tex: texture_2d<f32>, lod: f32, uv: vec2<f32>, texel_size: vec2<f32>) -> vec3<f32>
{
	// Center
	let A = textureSampleLevel(tex, samp, uv, lod).rgb;

	let texel_size = texel_size * 0.5; // Sample from center of texels

	// Inner box
	let B = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(-1.0, -1.0), lod).rgb;
	let C = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(-1.0, 1.0), lod).rgb;
	let D = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(1.0, 1.0), lod).rgb;
	let E = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(1.0, -1.0), lod).rgb;

	// Outer box
	let F = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(-2.0, -2.0), lod).rgb;
	let G = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(-2.0, 0.0), lod).rgb;
	let H = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(0.0, 2.0), lod).rgb;
	let I = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(2.0, 2.0), lod).rgb;
	let J = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(2.0, 2.0), lod).rgb;
	let K = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(2.0, 0.0), lod).rgb;
	let L = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(-2.0, -2.0), lod).rgb;
	let M = textureSampleLevel(tex, samp, uv + texel_size * vec2<f32>(0.0, -2.0), lod).rgb;

	// Weights
	var result: vec3<f32> = vec3<f32>(0.0);
	// Inner box
	result = result + (B + C + D + E) * 0.5;
	// Bottom-left box
	result = result + (F + G + A + M) * 0.125;
	// Top-left box
	result = result + (G + H + I + A) * 0.125;
	// Top-right box
	result = result + (A + I + J + K) * 0.125;
	// Bottom-right box
	result = result + (M + A + K + L) * 0.125;

	// 4 samples each
	result = result * 0.25;

	return result;
}

fn UpsampleTent9(tex: texture_2d<f32>, lod: f32, uv: vec2<f32>, texel_size: vec2<f32>, radius: f32) -> vec3<f32>
{
	let offset = texel_size.xyxy * vec4<f32>(1.0, 1.0, -1.0, 0.0) * radius;

	// Center
	var result: vec3<f32> = textureSampleLevel(tex, samp, uv, lod).rgb * 4.0;

	result = result + textureSampleLevel(tex, samp, uv - offset.xy, lod).rgb;
	result = result + textureSampleLevel(tex, samp, uv - offset.wy, lod).rgb * 2.0;
	result = result + textureSampleLevel(tex, samp, uv - offset.zy, lod).rgb;

	result = result + textureSampleLevel(tex, samp, uv + offset.zw, lod).rgb * 2.0;
	result = result + textureSampleLevel(tex, samp, uv + offset.xw, lod).rgb * 2.0;

	result = result + textureSampleLevel(tex, samp, uv + offset.zy, lod).rgb;
	result = result + textureSampleLevel(tex, samp, uv + offset.wy, lod).rgb * 2.0;
	result = result + textureSampleLevel(tex, samp, uv + offset.xy, lod).rgb;

	return result * (1.0 / 16.0);
}

fn combine(existing_color: vec3<f32>, color_to_add: vec3<f32>, combine_constant: f32) -> vec3<f32>
{
	let existing_color = existing_color + (-color_to_add);
	let blended_color = (combine_constant * existing_color) + color_to_add;
	return blended_color;
}


[[stage(compute), workgroup_size(8, 4, 1)]]
fn cs_main([[builtin(global_invocation_id)]] global_invocation_id: vec3<u32>)
{
	let mode_lod = pc.mode_lod;
	let mode = pc.mode_lod >> 16u;
	let lod = pc.mode_lod & 65535u;

	let out_text = output_texture;
	let in_text = input_texture;
	let bl_text = bloom_texture;

	let imgSize = textureDimensions(out_text);

	if (global_invocation_id.x <= u32(imgSize.x) && global_invocation_id.y <= u32(imgSize.y)) {

		// float combine_constant = 0.68;

		var texCoords: vec2<f32> = vec2<f32>(f32(global_invocation_id.x) / f32(imgSize.x), f32(global_invocation_id.y) / f32(imgSize.y));
		texCoords = texCoords + (1.0 / vec2<f32>(imgSize)) * 0.5;

		let texSize = vec2<f32>(textureDimensions(in_text, i32(lod)));
		var color: vec4<f32> = vec4<f32>(1.0);

		if (mode == MODE_PREFILTER)
		{
			color = vec4<f32>(DownsampleBox13(in_text, f32(lod), texCoords, 1.0 / texSize), 1.0);
			color = Prefilter(color, texCoords);
		}
		else if (mode == MODE_DOWNSAMPLE)
		{
			color = vec4<f32>(DownsampleBox13(in_text, f32(lod), texCoords, 1.0 / texSize), 1.0);
		}
		else if (mode == MODE_UPSAMPLE_FIRST)
		{
			let bloomTexSize = textureDimensions(in_text, i32(lod) + 1);
			let sampleScale = 1.0;
			let upsampledTexture = UpsampleTent9(in_text, f32(lod) + 1.0, texCoords, 1.0 / vec2<f32>(bloomTexSize), sampleScale);

			let existing = textureSampleLevel(in_text, samp, texCoords, f32(lod)).rgb;
			color = vec4<f32>(combine(existing, upsampledTexture, param.combine_constant), 1.0);
		}
		else if (mode == MODE_UPSAMPLE)
		{
			let bloomTexSize = textureDimensions(bl_text, i32(lod) + 1);
			let sampleScale = 1.0;
			let upsampledTexture = UpsampleTent9(bl_text, f32(lod) + 1.0, texCoords, 1.0 / vec2<f32>(bloomTexSize), sampleScale);

			let existing = textureSampleLevel(in_text, samp, texCoords, f32(lod)).rgb;
			color = vec4<f32>(combine(existing, upsampledTexture, param.combine_constant), 1.0);
		}
		textureStore(out_text, vec2<i32>(global_invocation_id.xy), color);
	}
}

