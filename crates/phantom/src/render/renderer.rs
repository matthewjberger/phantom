pub struct Renderer<'window> {
    pub gpu: crate::render::gpu::Gpu<'window>,
    view: Option<crate::render::view::WorldRender>,
    depth_texture_view: wgpu::TextureView,
    postprocess_pipeline: crate::render::postprocess::PostprocessingPipeline,
    gui_renderer: egui_wgpu::Renderer,
}

impl<'window> Renderer<'window> {
    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'window>>,
        width: u32,
        height: u32,
    ) -> Self {
        let gpu = crate::render::gpu::Gpu::new_async(window, width, height).await;
        let depth_texture_view = gpu.create_depth_texture(width, height);
        let postprocess_pipeline =
            crate::render::postprocess::PostprocessingPipeline::new(&gpu, width, height);
        let gui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            Some(wgpu::TextureFormat::Depth32Float),
            1,
        );
        Self {
            gpu,
            view: None,
            depth_texture_view,
            postprocess_pipeline,
            gui_renderer,
        }
    }

    pub fn load_world(&mut self, asset: &crate::world::World) {
        let _ = std::mem::replace(
            &mut self.view,
            Some(crate::render::view::WorldRender::new(&self.gpu, asset)),
        );
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.postprocess_pipeline =
            crate::render::postprocess::PostprocessingPipeline::new(&self.gpu, width, height);
        self.depth_texture_view = self.gpu.create_depth_texture(width, height);
    }

    pub fn render_frame(
        &mut self,
        context: &mut crate::app::Context,
        textures_delta: &egui::epaint::textures::TexturesDelta,
        paint_jobs: Vec<egui::ClippedPrimitive>,
        screen_descriptor: ScreenDescriptor,
    ) {
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        for (id, image_delta) in &textures_delta.set {
            self.gui_renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
        }

        for id in &textures_delta.free {
            self.gui_renderer.free_texture(id);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: screen_descriptor.size_in_pixels,
            pixels_per_point: screen_descriptor.pixels_per_point,
        };
        self.gui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let surface_texture = self
            .gpu
            .surface
            .get_current_texture()
            .expect("Failed to get surface texture!");

        let surface_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: wgpu::Label::default(),
                    aspect: wgpu::TextureAspect::default(),
                    format: Some(self.gpu.surface_format),
                    dimension: None,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                });

        encoder.insert_debug_marker("Render scene");

        // This scope around the crate::render_pass prevents the
        // crate::render_pass from holding a borrow to the encoder,
        // which would prevent calling `.finish()` in
        // preparation for queue submission.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.postprocess_pipeline.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.19,
                            g: 0.24,
                            b: 0.42,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            if let Some(view) = self.view.as_mut() {
                view.render(&mut render_pass, &self.gpu, context);
            }
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("PostProcess::render_to_texture"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &surface_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            render_pass.set_pipeline(&self.postprocess_pipeline.pipeline);
            render_pass.set_bind_group(0, &self.postprocess_pipeline.bind_group, &[]);
            render_pass.draw(0..3, 0..1);

            self.gui_renderer
                .render(&mut render_pass, &paint_jobs, &screen_descriptor);
        }

        self.gpu.queue.submit(std::iter::once(encoder.finish()));

        surface_texture.present();
    }
}

#[allow(dead_code)]
pub fn map_sampler(sampler: &crate::world::Sampler) -> wgpu::SamplerDescriptor<'static> {
    let min_filter = match sampler.min_filter {
        crate::world::MinFilter::Linear
        | crate::world::MinFilter::LinearMipmapLinear
        | crate::world::MinFilter::LinearMipmapNearest => wgpu::FilterMode::Linear,
        crate::world::MinFilter::Nearest
        | crate::world::MinFilter::NearestMipmapLinear
        | crate::world::MinFilter::NearestMipmapNearest => wgpu::FilterMode::Nearest,
    };

    let mipmap_filter = match sampler.min_filter {
        crate::world::MinFilter::Linear
        | crate::world::MinFilter::LinearMipmapLinear
        | crate::world::MinFilter::LinearMipmapNearest => wgpu::FilterMode::Linear,
        crate::world::MinFilter::Nearest
        | crate::world::MinFilter::NearestMipmapLinear
        | crate::world::MinFilter::NearestMipmapNearest => wgpu::FilterMode::Nearest,
    };

    let mag_filter = match sampler.mag_filter {
        crate::world::MagFilter::Linear => wgpu::FilterMode::Linear,
        crate::world::MagFilter::Nearest => wgpu::FilterMode::Nearest,
    };

    let address_mode_u = match sampler.wrap_s {
        crate::world::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        crate::world::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        crate::world::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
    };

    let address_mode_v = match sampler.wrap_t {
        crate::world::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        crate::world::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        crate::world::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
    };

    let address_mode_w = wgpu::AddressMode::Repeat;

    wgpu::SamplerDescriptor {
        address_mode_u,
        address_mode_v,
        address_mode_w,
        mag_filter,
        min_filter,
        mipmap_filter,
        ..Default::default()
    }
}

/// Information about the screen used for crate::rendering.
pub struct ScreenDescriptor {
    /// Size of the window in physical pixels.
    pub size_in_pixels: [u32; 2],

    /// HiDPI scale factor (pixels per point).
    pub pixels_per_point: f32,
}

impl ScreenDescriptor {
    /// size in "logical" points
    pub fn screen_size_in_points(&self) -> [f32; 2] {
        [
            self.size_in_pixels[0] as f32 / self.pixels_per_point,
            self.size_in_pixels[1] as f32 / self.pixels_per_point,
        ]
    }
}
