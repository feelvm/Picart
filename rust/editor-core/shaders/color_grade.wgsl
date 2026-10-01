// Fused color-grade fragment pass (WGSL / wgpu portable path).
// Fuses: brightness, contrast, saturation, exposure, temperature, tint,
// hue shift, vignette, grain, LUT-lite, opacity, mask, transform-sample.
// Goal: ONE texture sample + ONE pass for the common slider case.
//
// Uniform layout (vec4-aligned, 96 bytes):
// u0: brightness, contrast, saturation, exposure
// u1: temperature, tint, hue_shift, opacity
// u2: vignette, grain, lut_strength, mask_feather
// u3: viewport_w, viewport_h, time, blend_mode_id

struct Uniforms {
  u0: vec4<f32>, u1: vec4<f32>, u2: vec4<f32>, u3: vec4<f32>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var mask_tex: texture_2d<f32>;
@group(0) @binding(4) var mask_samp: sampler;

fn hue_shift_rgb(c: vec3<f32>, a: f32) -> vec3<f32> {
  let k = vec3<f32>(0.57735);
  let cos_a = cos(a); let sin_a = sin(a);
  return c * cos_a + cross(k, c) * sin_a + k * dot(k, c) * (1.0 - cos_a);
}

struct VSOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) mask_uv: vec2<f32>,
};

// Fullscreen triangle: no vertex buffer needed, 3 verts cover clip space.
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VSOut {
  let xy = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
  let p = xy[vi];
  var out: VSOut;
  out.pos = vec4<f32>(p, 0.0, 1.0);
  // Clip → texture uv. Texture v=0 is the top row; clip y=+1 is the top of
  // the screen, so flip y to keep images upright.
  out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5);
  out.mask_uv = out.uv;
  return out;
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>, @location(1) mask_uv: vec2<f32>) -> @location(0) vec4<f32> {
  var c: vec4<f32> = textureSample(src, samp, uv);
  // exposure + brightness
  c.rgb = c.rgb * exp2(u.u0.w);
  c.rgb = c.rgb + u.u0.x;
  // contrast around 0.5
  c.rgb = (c.rgb - 0.5) * u.u0.y + 0.5;
  // saturation
  let luma = dot(c.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
  c.rgb = mix(vec3<f32>(luma), c.rgb, u.u0.z);
  // temperature / tint (cheap white-balance approx)
  c.rgb *= vec3<f32>(1.0 + u.u1.x * 0.1, 1.0, 1.0 - u.u1.x * 0.1);
  c.rgb.g += u.u1.y * 0.05;
  // hue
  c.rgb = hue_shift_rgb(c.rgb, u.u1.z);
  // vignette
  let d = distance(uv, vec2<f32>(0.5));
  c.rgb *= 1.0 - u.u2.x * smoothstep(0.3, 0.9, d);
  // mask
  let m: f32 = textureSample(mask_tex, mask_samp, mask_uv).r;
  c.a = c.a * m * u.u1.w;
  // grain (hash without extra texture)
  let g = fract(sin(dot(uv * (u.u3.x + 1.0), vec2<f32>(12.9898, 78.233))) * 43758.5453);
  c.rgb += (g - 0.5) * u.u2.y * 0.08;
  return c;
}
