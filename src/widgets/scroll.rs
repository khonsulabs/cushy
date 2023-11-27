//! A container that scrolls its contents on a virtual surface.
use std::time::Duration;

use intentional::Cast;
use kludgine::app::winit::event::{DeviceId, MouseScrollDelta, TouchPhase};
use kludgine::app::winit::window::CursorIcon;
use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::{
    FloatConversion, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size,
};
use kludgine::shapes::Shape;
use kludgine::Color;

use crate::animation::{AnimationHandle, AnimationTarget, IntoAnimate, Spawn, ZeroToOne};
use crate::context::{AsEventContext, EventContext, LayoutContext};
use crate::styles::components::{EasingIn, EasingOut, LineHeight};
use crate::styles::Dimension;
use crate::value::Dynamic;
use crate::widget::{EventHandling, MakeWidget, Widget, WidgetRef, HANDLED, IGNORED};
use crate::ConstraintLimit;

/// A widget that supports scrolling its contents.
#[derive(Debug)]
pub struct Scroll {
    contents: WidgetRef,
    content_size: Size<Px>,
    control_size: Size<Px>,
    scroll: Dynamic<Point<Px>>,
    enabled: Point<bool>,
    max_scroll: Dynamic<Point<Px>>,
    scrollbar_opacity: Dynamic<ZeroToOne>,
    scrollbar_opacity_animation: AnimationHandle,
    horizontal_bar: ScrollbarInfo,
    vertical_bar: ScrollbarInfo,
    bar_width: Px,
    line_height: Px,
}

impl Scroll {
    /// Returns a new scroll widget containing `contents`.
    fn construct(contents: impl MakeWidget, enabled: Point<bool>) -> Self {
        Self {
            contents: WidgetRef::new(contents),
            enabled,
            content_size: Size::default(),
            control_size: Size::default(),
            scroll: Dynamic::new(Point::default()),
            max_scroll: Dynamic::new(Point::default()),
            scrollbar_opacity: Dynamic::default(),
            scrollbar_opacity_animation: AnimationHandle::new(),
            horizontal_bar: ScrollbarInfo::default(),
            vertical_bar: ScrollbarInfo::default(),
            bar_width: Px::default(),
            line_height: Px::default(),
        }
    }

    /// Returns a new scroll widget containing `contents` that allows scrolling
    /// vertically or horizontally.
    pub fn new(contents: impl MakeWidget) -> Self {
        Self::construct(contents, Point::new(true, true))
    }

    /// Returns a new scroll widget that allows scrolling `contents`
    /// horizontally.
    pub fn horizontal(contents: impl MakeWidget) -> Self {
        Self::construct(contents, Point::new(true, false))
    }

    /// Returns a new scroll widget that allows scrolling `contents` vertically.
    pub fn vertical(contents: impl MakeWidget) -> Self {
        Self::construct(contents, Point::new(false, true))
    }

    fn constrained_scroll(scroll: Point<Px>, max_scroll: Point<Px>) -> Point<Px> {
        scroll.max(max_scroll).min(Point::default())
    }

    fn constrain_scroll(&mut self) -> (Point<Px>, Point<Px>) {
        let scroll = self.scroll.get();
        let max_scroll = self.max_scroll.get();
        let clamped = Self::constrained_scroll(scroll, max_scroll);
        if clamped != scroll {
            self.scroll.set(clamped);
        }
        (clamped, max_scroll)
    }

    fn show_scrollbars(&mut self, context: &mut EventContext<'_, '_>) {
        self.scrollbar_opacity_animation = self
            .scrollbar_opacity
            .transition_to(ZeroToOne::ONE)
            .over(Duration::from_millis(300))
            .with_easing(context.get(&EasingIn))
            .and_then(Duration::from_secs(1))
            .and_then(
                self.scrollbar_opacity
                    .transition_to(ZeroToOne::ZERO)
                    .over(Duration::from_millis(300))
                    .with_easing(context.get(&EasingOut)),
            )
            .spawn();
    }
}

impl Widget for Scroll {
    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn hover(
        &mut self,
        _location: Point<Px>,
        context: &mut EventContext<'_, '_>,
    ) -> Option<CursorIcon> {
        self.show_scrollbars(context);

        None
    }

    fn unhover(&mut self, context: &mut EventContext<'_, '_>) {
        self.scrollbar_opacity_animation = self
            .scrollbar_opacity
            .transition_to(ZeroToOne::ZERO)
            .over(Duration::from_millis(300))
            .with_easing(context.get(&EasingOut))
            .spawn();
    }

    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_, '_>) {
        context.redraw_when_changed(&self.scrollbar_opacity);

        let managed = self.contents.mounted(&mut context.as_event_context());
        context.for_other(&managed).redraw();

        let size = context.gfx.region().size;

        if self.horizontal_bar.amount_hidden > 0 {
            context.gfx.draw_shape(&Shape::filled_rect(
                Rect::new(
                    Point::new(self.horizontal_bar.offset, size.height - self.bar_width),
                    Size::new(self.horizontal_bar.size, self.bar_width),
                ),
                Color::new_f32(1.0, 1.0, 1.0, *self.scrollbar_opacity.get()),
            ));
        }

        if self.vertical_bar.amount_hidden > 0 {
            context.gfx.draw_shape(&Shape::filled_rect(
                Rect::new(
                    Point::new(size.width - self.bar_width, self.vertical_bar.offset),
                    Size::new(self.bar_width, self.vertical_bar.size),
                ),
                Color::new_f32(1.0, 1.0, 1.0, *self.scrollbar_opacity.get()),
            ));
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        self.bar_width = context
            .get(&ScrollBarThickness)
            .into_px(context.gfx.scale());
        self.line_height = context.get(&LineHeight).into_px(context.gfx.scale());

        let (mut scroll, current_max_scroll) = self.constrain_scroll();

        let control_size =
            Size::new(available_space.width.max(), available_space.height.max()).into_signed();
        let max_extents = Size::new(
            if self.enabled.x {
                ConstraintLimit::SizeToFit((control_size.width).into_unsigned())
            } else {
                available_space.width
            },
            if self.enabled.y {
                ConstraintLimit::SizeToFit((control_size.height).into_unsigned())
            } else {
                available_space.height
            },
        );
        let managed = self.contents.mounted(&mut context.as_event_context());
        let new_content_size = context
            .for_other(&managed)
            .layout(max_extents)
            .into_signed();

        self.horizontal_bar =
            scrollbar_region(scroll.x, new_content_size.width, control_size.width);
        let max_scroll_x = if self.enabled.x {
            -self.horizontal_bar.amount_hidden
        } else {
            Px::ZERO
        };

        self.vertical_bar =
            scrollbar_region(scroll.y, new_content_size.height, control_size.height);
        let max_scroll_y = if self.enabled.y {
            -self.vertical_bar.amount_hidden
        } else {
            Px::ZERO
        };
        let new_max_scroll = Point::new(max_scroll_x, max_scroll_y);
        if current_max_scroll != new_max_scroll {
            self.max_scroll.set(new_max_scroll);
            scroll = scroll.max(new_max_scroll);
        }

        // Preserve the current scroll if the widget has resized
        if self.content_size.width != new_content_size.width
            || self.control_size.width != control_size.width
        {
            self.content_size.width = new_content_size.width;
            let scroll_pct = scroll.x.into_float() / current_max_scroll.x.into_float();
            scroll.x = max_scroll_x * scroll_pct;
        }

        if self.content_size.height != new_content_size.height
            || self.control_size.height != control_size.height
        {
            self.content_size.height = new_content_size.height;
            let scroll_pct = scroll.y.into_float() / current_max_scroll.y.into_float();
            scroll.y = max_scroll_y * scroll_pct;
        }
        self.scroll.set(scroll);
        self.control_size = control_size;
        self.content_size = new_content_size;

        let region = Rect::new(
            scroll,
            self.content_size
                .min(Size::new(Px::MAX, Px::MAX) - scroll.max(Point::default())),
        );
        context.set_child_layout(&managed, region);

        Size::new(
            if self.enabled.x {
                constrain_child(available_space.width, self.content_size.width)
            } else {
                self.content_size.width.into_unsigned()
            },
            if self.enabled.y {
                constrain_child(available_space.height, self.content_size.height)
            } else {
                self.content_size.height.into_unsigned()
            },
        )
    }

    fn mouse_wheel(
        &mut self,
        _device_id: DeviceId,
        delta: MouseScrollDelta,
        _phase: TouchPhase,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        let amount = match delta {
            MouseScrollDelta::LineDelta(x, y) => Point::new(x, y) * self.line_height.into_float(),
            MouseScrollDelta::PixelDelta(px) => Point::new(px.x.cast(), px.y.cast()),
        };
        context.invalidate_when_changed(&self.scroll);
        let mut scroll = self.scroll.lock();
        let old_scroll = *scroll;
        let new_scroll = Self::constrained_scroll(*scroll + amount.cast(), self.max_scroll.get());
        if old_scroll == new_scroll {
            IGNORED
        } else {
            *scroll = new_scroll;
            drop(scroll);

            self.show_scrollbars(context);
            context.set_needs_redraw();

            HANDLED
        }
    }
}

fn constrain_child(constraint: ConstraintLimit, measured: Px) -> UPx {
    let measured = measured.into_unsigned();
    match constraint {
        ConstraintLimit::Fill(size) => size.min(measured),
        ConstraintLimit::SizeToFit(_) => measured,
    }
}

#[derive(Debug, Default)]
struct ScrollbarInfo {
    offset: Px,
    amount_hidden: Px,
    size: Px,
}

fn scrollbar_region(scroll: Px, content_size: Px, control_size: Px) -> ScrollbarInfo {
    if content_size > control_size {
        let amount_hidden = content_size - control_size;
        let ratio_visible = control_size.into_float() / content_size.into_float();
        let bar_size = control_size * ratio_visible;
        let remaining_area = control_size - bar_size;
        let amount_scrolled = -scroll.into_float() / amount_hidden.into_float();
        let bar_offset = remaining_area * amount_scrolled;
        ScrollbarInfo {
            offset: bar_offset,
            amount_hidden,
            size: bar_size,
        }
    } else {
        ScrollbarInfo::default()
    }
}

define_components! {
    Scroll {
        /// The thickness that scrollbars are drawn with.
        ScrollBarThickness(Dimension, "size", Dimension::Lp(Lp::points(7)))
    }
}
