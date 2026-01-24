// Clouds based on simplex noise
//
// Tranlated by hand from " 3d simplex noise" by nikat
// from: https://www.shadertoy.com/view/XsX3zB

struct VertexOutput {
	@builtin(position) position: vec4<f32>,
	@location(2) uv: vec2<f32>,
}

//struct BackgroundMaterial {
//  params0: vec4<f32>,
//  params1: vec4<f32>,
//}

@group(2) @binding(0) var<uniform> params0: vec4<f32>;
@group(2) @binding(1) var<uniform> params1: vec4<f32>;

fn random3(c: vec3<f32>) -> vec3<f32> {
	let j = 4096.0*sin(dot(c, vec3(17.0, 59.4, 15.0)));
	let i = fract(512.0 * j);
	let r = vec3(
		i,
		i * .125,
		i * .125 * .125,
	);

	return r - 0.5;
}

const F3: f32 = 0.3333333;
const G3: f32 = 0.1666667;

fn simplex3d(p: vec3<f32>) -> f32 {
	let s = floor(p + dot(p, vec3(F3)));
	let x = p - s + dot(s, vec3(G3));

	let e = step(vec3(0.0), x - x.yzx);
	let i1 = e * (1.0 - e.zxy);
	let i2 = 1.0 - e.zxy * (1.0 - e);

	let x1 = x - i1 + G3;
	let x2 = x - i2 + 2.0 * G3;
	let x3 = x - 1.0 + 3.0 * G3;

	var w = vec4(
		dot(x, x),
		dot(x1, x1),
		dot(x2, x2),
		dot(x3, x3),
	);

	// huh?
	w = max(0.6 - w, vec4(0.0));

	var d = vec4(
		dot(random3(s), x),
		dot(random3(s + i1), x1),
		dot(random3(s + i2), x2),
		dot(random3(s + 1.0), x3),
	);

	w *= w;
	w *= w;
	d *= w;

	return dot(d, vec4(52.0));
}

const rot1: mat3x3<f32> = mat3x3(-0.37, 0.36, 0.85,-0.14,-0.93, 0.34,0.92, 0.01,0.4);
const rot2: mat3x3<f32> = mat3x3(-0.55,-0.39, 0.74, 0.33,-0.91,-0.24,0.77, 0.12,0.63);
const rot3: mat3x3<f32> = mat3x3(-0.71, 0.52,-0.47,-0.08,-0.72,-0.68,-0.7,-0.45,0.56);

fn simplex3d_fractal(m: vec3<f32>) -> f32 {
	return 
		0.5333333 * simplex3d(m * rot1)
	+ 0.2666667 * simplex3d(2.0 * m * rot2)
	+ 0.1333333 * simplex3d(4.0 * m * rot3)
	+ 0.0666667 * simplex3d(8.0 * m); 
}

@fragment
fn fragment(
	mesh: VertexOutput,
) -> @location(0) vec4<f32> {	
	let speed = params0.x;
	let scale = params0.yz;
  let time = params0.w;

	let drift_gain = params1.x;
	let color_gain = params1.y;
	let color_offset = params1.z;
	let alpha = params1.w;

	let drift = vec2(1.0, 0.1) * time * speed * drift_gain;

  let p = mesh.uv * scale + vec2(100.0);
	let p3 = vec3(p + drift, time * speed);

	var value: f32 = simplex3d_fractal(p3 * 8.0 + 8.0);

	value = 0.5 * value + 0.5;
	value *= 0.5 * smoothstep(0.0, 0.005, abs(0.6 - p.x));

	var color = vec3(0.7, 0.7, 0.9) * value;
	color = color_gain * color + color_offset;

	return vec4(color, alpha);
}
