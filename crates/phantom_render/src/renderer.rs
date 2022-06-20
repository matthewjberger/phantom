use crate::wgpu::WgpuRenderer;
use phantom_dependencies::{anyhow::Result, raw_window_handle::HasRawWindowHandle};

pub enum Backend {
    Wgpu,
}

pub trait Renderer {
    fn resize(&mut self, dimensions: [u32; 2]);
    fn render(&mut self) -> Result<()>;
}

pub fn create_render_backend(
    backend: &Backend,
    window_handle: &impl HasRawWindowHandle,
    dimensions: &[u32; 2],
) -> Result<Box<dyn Renderer>> {
    match backend {
        Backend::Wgpu => {
            let backend = WgpuRenderer::new(window_handle, dimensions)?;
            Ok(Box::new(backend) as Box<dyn Renderer>)
        }
    }
}
