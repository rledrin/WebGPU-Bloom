// Vertex Shader

struct Vertexinput {
	[[location(0)]] position: vec3<f32>;
	[[location(1)]] normal: vec3<f32>;
	[[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
	[[builtin(position)]] clip_position: vec4<f32>;
	[[location(0)]] normal: vec3<f32>;
	[[location(1)]] uv: vec2<f32>;
	[[location(2)]] world_pos: vec3<f32>;
};

struct Matrices {
	vp: mat4x4<f32>;
	model: mat4x4<f32>;
};

[[group(0), binding(0)]]
var<uniform> matrix: Matrices;

[[stage(vertex)]]
fn vs_main(in: Vertexinput) -> VertexOutput {
	var out: VertexOutput;
	let world_pos = matrix.model * vec4<f32>(in.position, 1.0);
	let screen_pos = matrix.vp * world_pos;

	out.uv = in.uv;
	out.normal = (matrix.model * vec4<f32>(in.normal, 1.0)).xyz;
	out.world_pos = world_pos.xyz;
	out.clip_position = screen_pos;
	return out;
}

// Fragment Shader

struct PbrParam {
	cam_pos: vec3<f32>;
	metallic: f32;
	albedo: vec3<f32>;
	roughness: f32;
	emissive_color: vec3<f32>;
	ao: f32;
	light_position: vec3<f32>;
	emissive_intensity: f32;
	light_color: vec3<f32>;
};

[[group(0), binding(1)]]
var<uniform> param: PbrParam;

let PI: f32 = 3.14159265359;

fn DistributionGGX(N: vec3<f32>, H: vec3<f32>, roughness: f32) -> f32 {
	let a = roughness*roughness;
	let a2 = a*a;
	let NdotH = max(dot(N, H), 0.0);
	let NdotH2 = NdotH*NdotH;

	let nom = a2;
	var denom: f32 = (NdotH2 * (a2 - 1.0) + 1.0);
	denom = PI * denom * denom;

	return nom / denom;
}
// ----------------------------------------------------------------------------
fn GeometrySchlickGGX(NdotV: f32, roughness: f32) -> f32 {
	let r = (roughness + 1.0);
	let k = (r*r) / 8.0;

	let nom = NdotV;
	let denom = NdotV * (1.0 - k) + k;

	return nom / denom;
}
// ----------------------------------------------------------------------------
fn GeometrySmith(N: vec3<f32>, V: vec3<f32>, L: vec3<f32>, roughness: f32) -> f32 {
	let NdotV = max(dot(N, V), 0.0);
	let NdotL = max(dot(N, L), 0.0);
	let ggx2 = GeometrySchlickGGX(NdotV, roughness);
	let ggx1 = GeometrySchlickGGX(NdotL, roughness);

	return ggx1 * ggx2;
}
// ----------------------------------------------------------------------------
fn fresnelSchlick(cosTheta: f32, F0: vec3<f32>) -> vec3<f32> {
	return F0 + (1.0 - F0) * pow(clamp(1.0 - cosTheta, 0.0, 1.0), 5.0);
}
// ----------------------------------------------------------------------------


[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
	let N = normalize(in.normal);
	let V = normalize(param.cam_pos - in.world_pos);

	var F0: vec3<f32> = vec3<f32>(0.04); 
	F0 = mix(F0, param.albedo, param.metallic);

	// reflectance equation
	var Lo: vec3<f32> = vec3<f32>(0.0);
	// calculate per-light radiance
	let L = normalize(param.light_position - in.world_pos);
	let H = normalize(V + L);
	let distance = length(param.light_position - in.world_pos);
	let attenuation = 1.0 / (distance * distance);
	let radiance = param.light_color * attenuation;

	// cook-torrance brdf
	let NDF = DistributionGGX(N, H, param.roughness);
	let G = GeometrySmith(N, V, L, param.roughness);
	let F = fresnelSchlick(max(dot(H, V), 0.0), F0);

	let kS = F;
	var kD: vec3<f32> = vec3<f32>(1.0) - kS;
	kD = kD * (1.0 - param.metallic);

	let numerator = NDF * G * F;
	let denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
	let specular = numerator / denominator;

	// add to outgoing radiance Lo
	let NdotL = max(dot(N, L), 0.0);
	Lo = Lo + ((kD * param.albedo / PI + specular) * radiance * NdotL);


	let ambient = vec3<f32>(0.03) * param.albedo * param.ao;
	let color = ambient + Lo;


	return vec4<f32>((color + (param.emissive_color * param.emissive_intensity)).xyz, 1.0);
}
