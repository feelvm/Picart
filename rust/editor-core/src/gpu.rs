//! GPU abstraction. wgpu is the portable path; `MetalBackend` / `VulkanBackend`
//! are direct integrations used when profiling shows a bandwidth or
//! pass-count win. Renderer code only depends on this trait.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind { Wgpu, Metal, Vulkan }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureHandle(pub u64);

#[derive(Debug, Clone)]
pub struct RenderCommand {
    pub pass_index: u32,
    pub pipeline: String,
    pub uniforms_16b: Vec<u8>,
    pub target: TextureHandle,
}

pub trait GpuBackend {
    fn kind(&self) -> BackendKind;
    /// Upload decoded tile; must not read back on hot path.
    fn upload_texture(&mut self, key: &str, rgba: &[u8], w: u32, h: u32) -> TextureHandle;
    fn submit(&mut self, cmds: &[RenderCommand]);
    fn gpu_memory_bytes(&self) -> u64;
    fn texture_count(&self) -> u64;
}

/// Null backend for host tests / UI-less logic tests.
#[derive(Debug, Default)]
pub struct NullBackend { mem: u64, textures: u64 }

impl GpuBackend for NullBackend {
    fn kind(&self) -> BackendKind { BackendKind::Wgpu }
    fn upload_texture(&mut self, _k: &str, rgba: &[u8], _w: u32, _h: u32) -> TextureHandle {
        self.mem += rgba.len() as u64;
        self.textures += 1;
        TextureHandle(self.textures)
    }
    fn submit(&mut self, _c: &[RenderCommand]) {}
    fn gpu_memory_bytes(&self) -> u64 { self.mem }
    fn texture_count(&self) -> u64 { self.textures }
}
