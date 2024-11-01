use std::borrow::Cow;

use cushy::widget::MakeWidget;
use cushy::widgets::Canvas;
use cushy::{RenderOperation, Run};
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

fn main() -> cushy::Result {
    let mut shader_op = None;
    Canvas::new(move |ctx| {
        if shader_op.is_none() {
            // Compile the shader now that we have access to wgpu
            let shader = ctx.gfx.inner_graphics().device().create_shader_module(
                wgpu::ShaderModuleDescriptor {
                    label: None,
                    source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(TRIANGLE_SHADER)),
                },
            );

            let pipeline_layout = ctx.gfx.inner_graphics().device().create_pipeline_layout(
                &wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                },
            );

            let pipeline = ctx.gfx.inner_graphics().device().create_render_pipeline(
                &wgpu::RenderPipelineDescriptor {
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
                    multisample: ctx.gfx.inner_graphics().multisample_state(),
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: "fs_main",
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(ctx.gfx.inner_graphics().texture_format().into())],
                    }),
                    multiview: None,
                    cache: None,
                },
            );

            // Create our rendering operation that uses the pipeline we created.
            shader_op = Some(RenderOperation::new(
                move |_origin, _opacity, ctx: &mut RenderingGraphics<'_, '_>| {
                    println!("Render to {_origin:?} clipped to {:?}", ctx.clip_rect());
                    ctx.pass_mut().set_pipeline(&pipeline);
                    ctx.pass_mut().draw(0..3, 0..1);
                },
            ));
        }

        // Draw our shader
        ctx.gfx.draw(shader_op.clone().expect("always initialized"));
    })
    .contain()
    .pad()
    .expand()
    .run()
}
