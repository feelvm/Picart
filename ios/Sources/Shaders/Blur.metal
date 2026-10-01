// Separable variable-radius gaussian blur (compute), for Phase 4.
// Two passes (H then V) ping-ponging between scratch textures. Radius is a
// real spatial radius: tap offsets scale with radius and the kernel is
// normalized, so brightness is preserved at every radius.
//
// Note: the dynamic loop bound (ceil(radius)) is legal in Metal but can be
// slower on some tile-based GPUs. If profiling shows it matters, add
// fixed-tap specializations (radius 2/4/8/16) and pick per preview scale.

#include <metal_stdlib>
using namespace metal;

struct BlurParams {
  int horizontal;  // 1 = horizontal pass, 0 = vertical
  float radius;    // px, clamped to MAX_RADIUS
  float2 texel;    // 1/width, 1/height
};

constant int MAX_RADIUS = 32;

kernel void blur_separable(
    texture2d<float, access::read> src [[texture(0)]],
    texture2d<float, access::write> dst [[texture(1)]],
    constant BlurParams &p [[buffer(0)]],
    uint2 gid [[thread_position_in_grid]]) {
  if (gid.x >= dst.get_width() || gid.y >= dst.get_height()) { return; }

  float4 c = src.read(gid);
  float r = clamp(p.radius, 0.0, float(MAX_RADIUS));
  if (r < 0.5) {
    dst.write(c, gid);
    return;
  }
  int2 s = (p.horizontal == 1) ? int2(1, 0) : int2(0, 1);
  int2 base = int2(gid);
  int2 lo = int2(0);
  int2 hi = int2(int(src.get_width()) - 1, int(src.get_height()) - 1);
  float sigma = max(r * 0.5, 0.5);
  int ir = (int)ceil(r);
  float3 acc = c.rgb;
  float wsum = 1.0;
  for (int i = 1; i <= ir; i++) {
    float w = exp(-float(i * i) / (2.0 * sigma * sigma));
    uint2 plus = uint2(clamp(base + s * i, lo, hi));
    uint2 minus = uint2(clamp(base - s * i, lo, hi));
    acc += (src.read(plus).rgb + src.read(minus).rgb) * w;
    wsum += 2.0 * w;
  }
  dst.write(float4(acc / wsum, c.a), gid);
}
