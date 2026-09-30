use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Rolling perf counters feeding the developer overlay:
/// `FPS / Frame / GPU / CPU / GPU-mem / Textures / CacheHit / Passes`.
#[derive(Debug, Default)]
pub struct PerfMonitor {
    frames: u64,
    start: Option<Instant>,
    last_frame_ms: f32,
    last_gpu_ms: f32,
    last_cpu_ms: f32,
    gpu_mem_bytes: AtomicU64Wrapper,
    texture_count: u64,
    pass_count: u64,
    cache_hits: u64,
    cache_total: u64,
}

#[derive(Debug, Default)]
struct AtomicU64Wrapper(AtomicU64);
impl AtomicU64Wrapper {
    fn load(&self) -> u64 { self.0.load(Ordering::Relaxed) }
    fn store(&self, v: u64) { self.0.store(v, Ordering::Relaxed) }
}

#[derive(Debug, Clone, Copy)]
pub struct PerfSnapshot {
    pub fps: f32,
    pub frame_ms: f32,
    pub gpu_ms: f32,
    pub cpu_ms: f32,
    pub gpu_mem_mb: f32,
    pub textures: u64,
    pub cache_hit_pct: f32,
    pub passes: u64,
}

impl PerfMonitor {
    pub fn new() -> Self { Self::default() }

    pub fn begin_frame(&mut self) {
        if self.start.is_none() { self.start = Some(Instant::now()); }
    }
    pub fn end_frame(&mut self, gpu_ms: f32, cpu_ms: f32, passes: u64) {
        self.frames += 1;
        self.last_gpu_ms = gpu_ms;
        self.last_cpu_ms = cpu_ms;
        self.last_frame_ms = gpu_ms + cpu_ms;
        self.pass_count = passes;
    }
    pub fn set_gpu_mem(&mut self, bytes: u64, textures: u64) {
        self.gpu_mem_bytes.store(bytes);
        self.texture_count = textures;
    }
    pub fn record_cache(&mut self, hit: bool) {
        self.cache_total += 1;
        if hit { self.cache_hits += 1; }
    }
    pub fn snapshot(&self) -> PerfSnapshot {
        let elapsed = self.start.map(|s| s.elapsed().as_secs_f32()).unwrap_or(1.0).max(0.001);
        PerfSnapshot {
            fps: self.frames as f32 / elapsed,
            frame_ms: self.last_frame_ms,
            gpu_ms: self.last_gpu_ms,
            cpu_ms: self.last_cpu_ms,
            gpu_mem_mb: self.gpu_mem_bytes.load() as f32 / 1_048_576.0,
            textures: self.texture_count,
            cache_hit_pct: if self.cache_total == 0 { 100.0 } else { self.cache_hits as f32 * 100.0 / self.cache_total as f32 },
            passes: self.pass_count,
        }
    }
    pub fn overlay_text(&self) -> String {
        let s = self.snapshot();
        format!(
            "FPS: {:.1}\nFrame: {:.1} ms\nGPU: {:.1} ms\nCPU: {:.1} ms\nGPU Memory: {:.0} MB\nTextures: {}\nCache Hit: {:.0}%\nRender Passes: {}",
            s.fps, s.frame_ms, s.gpu_ms, s.cpu_ms, s.gpu_mem_mb, s.textures, s.cache_hit_pct, s.passes
        )
    }
}
