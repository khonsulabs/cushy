use std::borrow::Cow;

use cushy::widget::MakeWidget;
use cushy::widgets::Canvas;
use cushy::{Run, SimpleRenderOperation};
use kludgine::{wgpu, RenderingGraphics};

static TRIANGLE_SHADER: &str = r#"
    @vertex
    fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
        let x = f32(i32(in_vertex_index) - 1);
        let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
        return vec4<f32>(x, y, 0.0, 1.0);
    }

    @fragment
    fn fs_main() -> @location(0) vec4<f32> {
        return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    }
"#;

pub struct TriangleShader {
    pipeline: wgpu::RenderPipeline,
}

impl SimpleRenderOperation for TriangleShader {
    fn new(graphics: &mut kludgine::Graphics<'_>) -> Self {
        let shader = graphics
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(TRIANGLE_SHADER)),
            });

        let pipeline_layout =
            graphics
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let pipeline = graphics
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: graphics.multisample_state(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(graphics.texture_format().into())],
                }),
                multiview: None,
                cache: None,
            });

        Self { pipeline }
    }

    fn render(
        &self,
        origin: figures::Point<figures::units::Px>,
        _opacity: f32,
        graphics: &mut RenderingGraphics<'_, '_>,
    ) {
        println!("Render to {origin:?} clipped to {:?}", graphics.clip_rect());
        graphics.pass_mut().set_pipeline(&self.pipeline);
        graphics.pass_mut().draw(0..3, 0..1);
    }
}

fn main() -> cushy::Result {
    Canvas::new(move |ctx| {
        ctx.gfx.draw::<TriangleShader>();
    })
    .contain()
    .pad()
    .expand()
    .run()
}
