// Fused color-grade pass (the only pipeline needed for Phase 1/2).
// Fuses: exposure, brightness, contrast, saturation, temperature, tint,
// hue shift, vignette, grain, mask, opacity, transform sampling.
// ONE texture sample + ONE pass for the common slider case.
//
// The Rust core packs uniforms (`editor-core/src/uniforms.rs`):
//  0.. 4  params0  brightness, contrast, saturation, exposure
//  4.. 8  params1  temperature, tint, hue_shift, opacity
//  8..12  params2  vignette, grain, lut_strength, mask_feather
// 12..16  params3  viewport_w, viewport_h, time_s, blend_mode
// 16..28  m0, m1, m2  screen-uv → layer-uv affine matrix rows (w unused)
//
// uv convention: (0,0) = top-left of screen and of the layer texture.

#include <metal_stdlib>
using namespace metal;

struct ColorUniforms {
  float4 params0;
  float4 params1;
  float4 params2;
  float4 params3;
  float4 m0;
  float4 m1;
  float4 m2;
};

struct VSOut {
  float4 pos [[position]];
  float2 uv;
};

// Fullscreen triangle: 3 vertices cover clip space; no vertex buffer needed.
vertex VSOut vs_fullscreen(uint vid [[vertex_id]]) {
  float2 p[3] = { float2(-1.0, -1.0), float2(3.0, -1.0), float2(-1.0, 3.0) };
  VSOut o;
  o.pos = float4(p[vid], 0.0, 1.0);
  // Clip → uv. Texture v=0 is the top row; clip y=+1 is screen top, so flip.
  o.uv = float2((p[vid].x + 1.0) * 0.5, (1.0 - p[vid].y) * 0.5);
  return o;
}

float3 hue_shift_rgb(float3 c, float a) {
  float3 k = float3(0.57735);
  float cos_a = cos(a);
  float sin_a = sin(a);
  return c * cos_a + cross(k, c) * sin_a + k * dot(k, c) * (1.0 - cos_a);
}

fragment float4 fs_color_grade(
    VSOut in [[stage_in]],
    texture2d<float> src [[texture(0)]],
    texture2d<float> mask [[texture(1)]],
    constant ColorUniforms &u [[buffer(0)]]) {
  constexpr sampler s(filter::linear, address::clamp_to_edge);

  // screen uv → layer uv via the packed affine rows (explicit row dot products;
  // avoids MSL column/row constructor ambiguity).
  float2 luv = float2(
      u.m0.x * in.uv.x + u.m0.y * in.uv.y + u.m0.z,
      u.m1.x * in.uv.x + u.m1.y * in.uv.y + u.m1.z);

  float4 c = src.sample(s, luv);
  // exposure + brightness
  c.rgb = c.rgb * exp2(u.params0.w);
  c.rgb = c.rgb + u.params0.x;
  // contrast around 0.5
  c.rgb = (c.rgb - 0.5) * u.params0.y + 0.5;
  // saturation (BT.709 luma)
  float luma = dot(c.rgb, float3(0.2126, 0.7152, 0.0722));
  c.rgb = mix(float3(luma), c.rgb, u.params0.z);
  // temperature / tint (cheap white-balance approx)
  c.rgb *= float3(1.0 + u.params1.x * 0.1, 1.0, 1.0 - u.params1.x * 0.1);
  c.rgb.g += u.params1.y * 0.05;
  // hue
  c.rgb = hue_shift_rgb(c.rgb, u.params1.z);
  // vignette (screen-space)
  float d = distance(in.uv, float2(0.5));
  c.rgb *= 1.0 - u.params2.x * smoothstep(0.3, 0.9, d);
  // mask × opacity
  float m = mask.sample(s, luv).r;
  c.a = c.a * m * u.params1.w;
  // grain (hash without extra texture)
  float g = fract(sin(dot(in.uv * (u.params3.x + 1.0), float2(12.9898, 78.233))) * 43758.5453);
  c.rgb += (g - 0.5) * u.params2.y * 0.08;
  return c;
}
