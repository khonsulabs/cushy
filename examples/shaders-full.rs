//! The `shaders.rs` example demonstrates a minimal shader setup. However, its
//! rendering is clipped to the control bounds rather than the triangle being
//! resized to fit within the widget. This example shows how to collect the
//! needed information from Cushy/Kludgine and use it to position, scale, and
//! honor opacity in the shader correctly. Clipping is automatically handled by
//! Kludgine.
//!
//! Information can be passed from Rust to wgpu through Uniforms, Push
//! Constants, or the individual drawing calls. This example shows how to
//! utilize:
//!
//! - uniforms to communicate the window size, which remains static throughout a
//!   Cushy user interface render
//! - Push constants to communicate the viewport and opacity of the individual
//!   drawing operation
//!
//! This approach allows this render operation to share a single set of
//! resources across multiple drawing operations.
use std::borrow::Cow;
use std::mem;
use std::num::NonZeroU64;

use bytemuck::{Pod, Zeroable};
use cushy::animation::ZeroToOne;
use cushy::figures::units::Px;
use cushy::figures::Rect;
use cushy::kludgine::{wgpu, RenderingGraphics};
use cushy::styles::components::Opacity;
use cushy::value::Dynamic;
use cushy::widget::MakeWidget;
use cushy::widgets::slider::Slidable;
use cushy::widgets::Canvas;
use cushy::{RenderOperation, Run};
use kludgine::wgpu::util::DeviceExt;

#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq)]
#[repr(C)]
struct PushConstants {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    opacity: f32,
}

#[derive(Debug, Clone, Copy, Pod, Zeroable, PartialEq, Default)]
#[repr(C)]
struct Uniforms {
    width: f32,
    height: f32,
}

static TRIANGLE_SHADER: &str = r#"
    struct PushConstants {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        opacity: f32,
    }
    var<push_constant> push_constants: PushConstants;

    struct Uniforms {
        width: f32,
        height: f32,
    }
    @group(0) @binding(0)
    var<uniform> uniforms: Uniforms;

    fn ortho2d(left: f32, top: f32, right: f32, bottom: f32, near: f32, far: f32) -> mat4x4f {
        let tx = -((right + left) / (right - left));
        let ty = -((top + bottom) / (top - bottom));
        let tz = -((far + near) / (far - near));
        return mat4x4(
            vec4f(2. / (right - left), 0., 0., 0.), 
            vec4f(0., 2. / (top - bottom), 0., 0.), 
            vec4f(0., 0., -2. / (far - near), 0.), 
            vec4f(tx, ty, tz, 1.)
        );
    }

    @vertex
    fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
        let projection = ortho2d(
            0., 
            0., 
            uniforms.width, 
            uniforms.height,
            -1.0,
            1.0
        );
        let half_size = vec2f(push_constants.width, push_constants.height) / 2.;
        let center = vec2f(push_constants.x, push_constants.y) + half_size;
        let unit_point = vec2f(f32(i32(in_vertex_index) - 1), f32(1 - i32(in_vertex_index & 1u) * 2));
        let vertex = center + unit_point * half_size;
        return projection * vec4<f32>(vertex, 0.0, 1.0);
    }

    @fragment
    fn fs_main() -> @location(0) vec4<f32> {
        return vec4<f32>(1.0, 0.0, 0.0, push_constants.opacity);
    }
"#;

pub struct TriangleShader {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    stored_uniforms: Uniforms,
    bindings: wgpu::BindGroup,
}

impl RenderOperation for TriangleShader {
    type DrawInfo = ();
    type Prepared = ();

    fn new(graphics: &mut kludgine::Graphics<'_>) -> Self {
        let shader = graphics
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(TRIANGLE_SHADER)),
            });

        let bind_group_layout =
            graphics
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: NonZeroU64::new(mem::size_of::<Uniforms>() as u64),
                        },
                        count: None,
                    }],
                });

        let current_uniforms = Uniforms::default();
        let uniforms = graphics
            .device()
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::bytes_of(&current_uniforms),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let pipeline_layout =
            graphics
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[wgpu::PushConstantRange {
                        stages: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        range: 0..mem::size_of::<PushConstants>() as u32,
                    }],
                });

        let pipeline = graphics
            .device()
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: graphics.multisample_state(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),

                    targets: &[Some(wgpu::ColorTargetState {
                        format: graphics.texture_format(),
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),

                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            });

        let bindings = graphics
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &bind_group_layout,

                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                        buffer: &uniforms,
                        offset: 0,
                        size: NonZeroU64::new(mem::size_of::<Uniforms>() as u64),
                    }),
                }],
            });

        Self {
            pipeline,
            uniforms,
            stored_uniforms: current_uniforms,
            bindings,
        }
    }

    fn prepare(
        &mut self,
        _context: Self::DrawInfo,
        _region: Rect<Px>,
        _opacity: ZeroToOne,
        graphics: &mut kludgine::Graphics<'_>,
    ) -> Self::Prepared {
        let uniforms = Uniforms {
            width: graphics.kludgine().size().width.into(),
            height: graphics.kludgine().size().height.into(),
        };
        if self.stored_uniforms != uniforms {
            self.stored_uniforms = uniforms;
            graphics.queue().write_buffer(
                &self.uniforms,
                0,
                bytemuck::bytes_of(&self.stored_uniforms),
            );
        }
    }

    fn render(
        &self,
        _prepared: &Self::Prepared,
        region: Rect<Px>,
        opacity: ZeroToOne,
        graphics: &mut RenderingGraphics<'_, '_>,
    ) {
        println!(
            "Render to {region:?} clipped to {:?} with {opacity:?}",
            graphics.clip_rect()
        );
        graphics.pass_mut().set_pipeline(&self.pipeline);
        graphics.pass_mut().set_bind_group(0, &self.bindings, &[]);
        graphics.pass_mut().set_push_constants(
            wgpu::ShaderStages::VERTEX_FRAGMENT,
            0,
            bytemuck::bytes_of(&PushConstants {
                x: region.origin.x.into(),
                y: region.origin.y.into(),
                width: region.size.width.into(),
                height: region.size.height.into(),
                opacity: *opacity,
            }),
        );
        graphics.pass_mut().draw(0..3, 0..1);
    }
}

fn main() -> cushy::Result {
    let opacity = Dynamic::new(ZeroToOne::ONE);
    opacity
        .clone()
        .slider()
        .and(
            Canvas::new(move |ctx| {
                ctx.gfx.draw::<TriangleShader>();
            })
            .with(&Opacity, opacity)
            .contain(),
        )
        .into_rows()
        .pad()
        .expand()
        .run()
}
