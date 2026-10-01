// Direct Metal path: composite + blend modes. One kernel per blend fn so
// new modes don't require renderer rewrites — add a fn + update BlendMode::shader_fn.
#include <metal_stdlib>
using namespace metal;

struct CompositeUniforms {
  float4x4 layerMat;
  float opacity;
  uint blendMode; // 0 normal … 9 difference
  float2 viewport;
};

float3 blend_one(float3 dst, float3 src, uint mode) {
  switch (mode) {
    case 1: return dst * src;                                  // multiply
    case 2: return 1.0 - (1.0 - dst) * (1.0 - src);            // screen
    case 3: return mix(2.0*dst*src, 1.0-2.0*(1.0-dst)*(1.0-src), step(0.5, dst)); // overlay
    case 4: return (dst < 0.5)
        ? (2.0*dst*src + dst*dst*(1.0-2.0*src))
        : (sqrt(dst)*(2.0*src-1.0) + 2.0*dst*(1.0-src)); // soft light approx
    case 5: return (src < 0.5) ? (2.0*dst*src) : (1.0-2.0*(1.0-dst)*(1.0-src));   // hard light
    case 6: return min(dst, src);                               // darken
    case 7: return max(dst, src);                               // lighten
    case 8: return dst + src;                                   // add
    case 9: return abs(dst - src);                              // difference
    default: return src;                                        // normal
  }
}

kernel void composite_blend(
    texture2d<float, access::read> src [[texture(0)]],
    texture2d<float, access::read_write> dst [[texture(1)]],
    constant CompositeUniforms& u [[buffer(0)]],
    uint2 gid [[thread_position_in_grid]]) {
  if (gid.x >= dst.get_width() || gid.y >= dst.get_height()) return;
  float4 s = src.read(gid);
  float4 d = dst.read(gid);
  float3 b = blend_one(d.rgb, s.rgb, u.blendMode);
  float a = s.a * u.opacity;
  dst.write(float4(mix(d.rgb, b, a), max(d.a, a)), gid);
}
