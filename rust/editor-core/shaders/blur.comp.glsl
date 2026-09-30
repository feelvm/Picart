// Vulkan GLSL path: separable blur (shared with wgpu compute equivalent).
// Two passes (H+V), radius via specialization constant — no recompile per slider tick.
#version 450
layout(local_size_x = 16, local_size_y = 16) in;
layout(binding = 0) uniform sampler2D src;
layout(binding = 1, rgba8) uniform writeonly image2D dst;
layout(push_constant) uniform PC { int horizontal; float radius; vec2 texel; } pc;

void main() {
  ivec2 xy = ivec2(gl_GlobalInvocationID.xy);
  vec2 uv = (vec2(xy) + 0.5) * pc.texel;
  vec3 acc = texture(src, uv).rgb * 0.227027;
  vec2 off = pc.horizontal == 1 ? vec2(pc.texel.x, 0.0) : vec2(0.0, pc.texel.y);
  float w[4] = float[4](0.1945946, 0.1216216, 0.054054, 0.016216);
  for (int i = 1; i <= 4; i++) {
    float k = w[i-1] * clamp(pc.radius / 8.0, 0.0, 1.0);
    acc += texture(src, uv + off * float(i)).rgb * k;
    acc += texture(src, uv - off * float(i)).rgb * k;
  }
  imageStore(dst, xy, vec4(acc, 1.0));
}
