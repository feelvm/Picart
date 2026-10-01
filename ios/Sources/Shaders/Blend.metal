// Blend functions for multi-layer compositing (used by the Phase 3 composite
// fragment pass; not referenced by the Phase 1 single-layer pipeline yet).
// One pure function per mode so new modes don't require renderer rewrites —
// add a fn here and map it in BlendMode::shader_fn (editor-core/src/layer.rs).
//
// All functions take the *accumulated* destination and the *source* color in
// 0..1 and return the blended color; alpha composition happens in the caller.

#include <metal_stdlib>
using namespace metal;

static inline float3 blend_one(float3 dst, float3 src, uint mode) {
  switch (mode) {
    case 1: return dst * src;                                     // multiply
    case 2: return 1.0 - (1.0 - dst) * (1.0 - src);               // screen
    case 3: return mix(2.0*dst*src, 1.0-2.0*(1.0-dst)*(1.0-src), step(0.5, dst)); // overlay
    case 4: return (dst < 0.5)                                    // soft light
        ? (2.0*dst*src + dst*dst*(1.0-2.0*src))
        : (sqrt(dst)*(2.0*src-1.0) + 2.0*dst*(1.0-src));
    case 5: return (src < 0.5) ? (2.0*dst*src) : (1.0-2.0*(1.0-dst)*(1.0-src));   // hard light
    case 6: return min(dst, src);                                 // darken
    case 7: return max(dst, src);                                 // lighten
    case 8: return dst + src;                                     // add
    case 9: return abs(dst - src);                                // difference
    default: return src;                                          // normal
  }
}
