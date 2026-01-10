#import bevy_sprite::mesh2d_vertex_output::VertexOutput;
#import bevy_sprite::mesh2d_view_bindings::globals;

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var base_color_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var base_color_sampler: sampler;

//mat2 rot(in float a){float c = cos(a), s = sin(a);return mat2(c,s,-s,c);}
fn rot(a: f32) -> mat2x2<f32> {
	let c = cos(a);
	let s = sin(a);
	return mat2x2(c, s, -s, c);
}

//const mat3 m3 = mat3(0.33338, 0.56034, -0.71817, -0.87887, 0.32651, -0.15323, 0.15162, 0.69596, 0.61339)*1.93;
const m3: mat3x3<f32> = mat3x3(
	0.33338, 0.56034, -0.71817,
	-0.87887, 0.32651, -0.15323,
	0.15162, 0.69596, 0.61339,
) * 1.93;

//float mag2(vec2 p){return dot(p,p);}
fn mag2(p: vec2<f32>) -> f32 {
	return dot(p, p);
}

//float linstep(in float mn, in float mx, in float x){ return clamp((x - mn)/(mx - mn), 0., 1.); }
fn linstep(mn: f32, mx: f32, x: f32) -> f32 {
	return clamp((x - mn)/(mx - mn), 0., 1.);
}

//float prm1 = 0.;
//vec2 bsMo = vec2(0);

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
		//let t = globals.time;

		//var texture_color = textureSample(
		//		base_color_texture, 
		//		base_color_sampler,
		//		in.uv
		//		//clamp(fract(in.uv * 10.0), vec2(0.0), vec2(1.0))
		//).xyz;

		//return vec4(texture_color, 1.0);
		return vec4(vec3(0.125), 1.0);
}
