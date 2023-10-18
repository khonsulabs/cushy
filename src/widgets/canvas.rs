use std::fmt::Debug;
use std::panic::UnwindSafe;
use std::time::{Duration, Instant};

use kludgine::figures::units::UPx;
use kludgine::figures::Size;

use crate::context::Context;
use crate::graphics::Graphics;
use crate::widget::Widget;

#[must_use]
pub struct Canvas {
    render: Box<dyn RenderFunction>,
    target_frame_duration: Option<Duration>,
    last_frame_time: Option<Instant>,
}

impl Canvas {
    pub fn new<F>(render: F) -> Self
    where
        F: for<'clip, 'gfx, 'pass, 'context, 'window> FnMut(
                &mut Graphics<'clip, 'gfx, 'pass>,
                &mut Context<'context, 'window>,
            ) + Send
            + UnwindSafe
            + 'static,
    {
        Self {
            render: Box::new(render),
            target_frame_duration: None,
            last_frame_time: None,
        }
    }

    pub fn target_fps(mut self, fps: u16) -> Self {
        const ONE_SECOND_NS: u64 = 1_000_000_000;
        let frame_duration = ONE_SECOND_NS / u64::from(fps);
        self.target_frame_duration = Some(Duration::from_nanos(frame_duration));
        self
    }
}

impl Widget for Canvas {
    fn redraw(&mut self, graphics: &mut Graphics<'_, '_, '_>, context: &mut Context<'_, '_>) {
        self.render.render(graphics, context);

        if let Some(target_frame_duration) = self.target_frame_duration {
            let now = Instant::now();
            let max_target = now + target_frame_duration;
            let next_frame_target = self.last_frame_time.map_or(max_target, |last_frame_time| {
                max_target.max(last_frame_time + target_frame_duration)
            });
            context.redraw_at(next_frame_target);
        }
    }

    fn measure(
        &mut self,
        available_space: Size<crate::ConstraintLimit>,
        _graphics: &mut Graphics<'_, '_, '_>,
        _context: &mut Context<'_, '_>,
    ) -> Size<UPx> {
        Size::new(available_space.width.max(), available_space.height.max())
    }
}

impl Debug for Canvas {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Canvas").finish_non_exhaustive()
    }
}

trait RenderFunction: Send + UnwindSafe + 'static {
    fn render(&mut self, graphics: &mut Graphics<'_, '_, '_>, context: &mut Context<'_, '_>);
}

impl<F> RenderFunction for F
where
    F: for<'clip, 'gfx, 'pass, 'context, 'window> FnMut(
            &mut Graphics<'clip, 'gfx, 'pass>,
            &mut Context<'context, 'window>,
        ) + Send
        + UnwindSafe
        + 'static,
{
    fn render(&mut self, graphics: &mut Graphics<'_, '_, '_>, window: &mut Context<'_, '_>) {
        self(graphics, window);
    }
}
