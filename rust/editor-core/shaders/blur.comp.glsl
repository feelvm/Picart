// Vulkan GLSL path: separable blur (shared with wgpu compute equivalent).
// Two passes (H+V). Radius is a real spatial radius: tap offsets scale with
// radius and the kernel is normalized, so output brightness is preserved at
// every radius. No recompile per slider tick.
//
// Note: the dynamic loop bound (ceil(radius)) is legal in Vulkan but can be
// slower on some tile-based mobile GPUs. If profiling shows it matters, add
// fixed-tap specializations (e.g. radius 2/4/8/16) and pick per preview scale.
#version 450
layout(local_size_x = 16, local_size_y = 16) in;
layout(binding = 0) uniform sampler2D src;
layout(binding = 1, rgba8) uniform writeonly image2D dst;
layout(push_constant) uniform PC { int horizontal; float radius; vec2 texel; } pc;

void main() {
  ivec2 xy = ivec2(gl_GlobalInvocationID.xy);
  vec2 uv = (vec2(xy) + 0.5) * pc.texel;
  vec4 c = texture(src, uv);
  float r = clamp(pc.radius, 0.0, 32.0);
  if (r < 0.5) {
    imageStore(dst, xy, c);
    return;
  }
  vec2 off = pc.horizontal == 1 ? vec2(pc.texel.x, 0.0) : vec2(0.0, pc.texel.y);
  float sigma = max(r * 0.5, 0.5);
  int ir = int(ceil(r));
  vec3 acc = c.rgb;
  float wsum = 1.0;
  for (int i = 1; i <= ir; i++) {
    float w = exp(-float(i * i) / (2.0 * sigma * sigma));
    acc += (texture(src, uv + off * float(i)).rgb + texture(src, uv - off * float(i)).rgb) * w;
    wsum += 2.0 * w;
  }
  imageStore(dst, xy, vec4(acc / wsum, c.a));
}
