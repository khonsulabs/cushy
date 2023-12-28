//! A container that scrolls its contents on a virtual surface.
use std::time::{Duration, Instant};

use figures::units::{Lp, Px, UPx};
use figures::{FloatConversion, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size, Zero};
use intentional::Cast;
use kludgine::app::winit::event::{DeviceId, MouseScrollDelta, TouchPhase};
use kludgine::app::winit::window::CursorIcon;
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
    scrollbar_opacity_animation: OpacityAnimationState,
    horizontal_bar: ScrollbarInfo,
    vertical_bar: ScrollbarInfo,
    bar_width: Px,
    line_height: Px,
    drag: DragInfo,
}

#[derive(Debug)]
struct OpacityAnimationState {
    will_hide: bool,
    started_at: Instant,
    handle: AnimationHandle,
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
            scrollbar_opacity_animation: OpacityAnimationState {
                handle: AnimationHandle::new(),
                started_at: Instant::now(),
                will_hide: true,
            },
            horizontal_bar: ScrollbarInfo::default(),
            vertical_bar: ScrollbarInfo::default(),
            bar_width: Px::default(),
            line_height: Px::default(),
            drag: DragInfo::default(),
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
        let should_hide = self.drag.mouse_buttons_down == 0;
        if should_hide != self.scrollbar_opacity_animation.will_hide
            || self.scrollbar_opacity_animation.handle.is_complete()
            // Prevent respawning the same animation multiple times if we get a
            // lot of events.
            || self.scrollbar_opacity_animation.started_at.elapsed() > Duration::from_millis(500)
        {
            let current_opacity = self.scrollbar_opacity.get();
            let transition_time = *current_opacity.one_minus() / 4.;
            let animation = self
                .scrollbar_opacity
                .transition_to(ZeroToOne::ONE)
                .over(Duration::from_secs_f32(transition_time))
                .with_easing(context.get(&EasingIn));

            self.scrollbar_opacity_animation.will_hide = should_hide;
            self.scrollbar_opacity_animation.handle = if should_hide {
                animation
                    .and_then(Duration::from_secs(1))
                    .and_then(
                        self.scrollbar_opacity
                            .transition_to(ZeroToOne::ZERO)
                            .over(Duration::from_millis(300))
                            .with_easing(context.get(&EasingOut)),
                    )
                    .spawn()
            } else {
                animation.spawn()
            };
        }
    }

    fn hide_scrollbars(&mut self, context: &mut EventContext<'_, '_>) {
        if self.drag.mouse_buttons_down == 0 && !self.scrollbar_opacity_animation.will_hide {
            self.scrollbar_opacity_animation.will_hide = true;
            self.scrollbar_opacity_animation.handle = self
                .scrollbar_opacity
                .transition_to(ZeroToOne::ZERO)
                .over(Duration::from_millis(300))
                .with_easing(context.get(&EasingOut))
                .spawn();
        }
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
        self.hide_scrollbars(context);
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

        let max_extents = Size::new(
            if self.enabled.x {
                ConstraintLimit::SizeToFit(UPx::MAX)
            } else {
                available_space.width
            },
            if self.enabled.y {
                ConstraintLimit::SizeToFit(UPx::MAX)
            } else {
                available_space.height
            },
        );
        let managed = self.contents.mounted(&mut context.as_event_context());
        let new_content_size = context
            .for_other(&managed)
            .layout(max_extents)
            .into_signed();

        let layout_size = Size::new(
            if self.enabled.x {
                constrain_child(available_space.width, new_content_size.width)
            } else {
                new_content_size.width.into_unsigned()
            },
            if self.enabled.y {
                constrain_child(available_space.height, new_content_size.height)
            } else {
                new_content_size.height.into_unsigned()
            },
        );
        let control_size = layout_size.into_signed();

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
        // Set the current scroll, but prevent immediately triggering
        // invalidate.
        {
            let mut current_scroll = self.scroll.lock();
            current_scroll.prevent_notifications();
            *current_scroll = scroll;
        }
        context.invalidate_when_changed(&self.scroll);
        self.control_size = control_size;
        self.content_size = new_content_size;

        let region = Rect::new(
            scroll,
            self.content_size
                .min(Size::new(Px::MAX, Px::MAX) - scroll.max(Point::default())),
        );
        context.set_child_layout(&managed, region);

        layout_size
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
        let mut scroll = self.scroll.lock();
        let old_scroll = *scroll;
        let new_scroll =
            Self::constrained_scroll(*scroll + amount.cast::<Px>(), self.max_scroll.get());
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

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_, '_>,
    ) -> EventHandling {
        let relative_x = (self.control_size.width - location.x).max(Px::ZERO);
        let in_vertical_area = self.enabled.y && relative_x <= self.bar_width;

        let relative_y = (self.control_size.height - location.y).max(Px::ZERO);
        let in_horizontal_area = self.enabled.x && relative_y <= self.bar_width;

        if matches!(
            (in_horizontal_area, in_vertical_area),
            (true, true) | (false, false)
        ) {
            return IGNORED;
        }

        self.drag.start = location;
        self.drag.start_scroll = self.scroll.get();
        self.drag.horizontal = in_horizontal_area;
        self.drag.in_bar = if in_horizontal_area {
            let relative = location.x - self.horizontal_bar.offset;
            relative >= 0 && relative < self.horizontal_bar.size
        } else {
            let relative = location.y - self.vertical_bar.offset;
            relative >= 0 && relative < self.vertical_bar.size
        };

        // If we clicked in the open area, we need to jump to the new location
        // immediately.
        if !self.drag.in_bar {
            self.drag.update(
                location,
                &self.scroll,
                &self.horizontal_bar,
                &self.vertical_bar,
                self.max_scroll.get(),
                self.control_size,
            );
        }

        self.drag.mouse_buttons_down += 1;
        self.show_scrollbars(context);

        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        _context: &mut EventContext<'_, '_>,
    ) {
        self.drag.update(
            location,
            &self.scroll,
            &self.horizontal_bar,
            &self.vertical_bar,
            self.max_scroll.get(),
            self.control_size,
        );
    }

    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        _device_id: DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_, '_>,
    ) {
        self.drag.mouse_buttons_down -= 1;

        if self.drag.mouse_buttons_down == 0 {
            if location.map_or(false, |location| {
                Rect::from(self.control_size).contains(location)
            }) {
                self.scrollbar_opacity_animation.handle.clear();
                self.show_scrollbars(context);
            } else {
                self.hide_scrollbars(context);
            }
        }
    }

    fn summarize(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("Scroll")
            .field("enabled", &self.enabled)
            .field("contents", &self.contents)
            .finish()
    }
}

#[derive(Default, Debug)]
struct DragInfo {
    mouse_buttons_down: usize,
    start: Point<Px>,
    start_scroll: Point<Px>,
    horizontal: bool,
    in_bar: bool,
}

impl DragInfo {
    fn update(
        &self,
        location: Point<Px>,
        dynamic_scroll: &Dynamic<Point<Px>>,
        horizontal_bar: &ScrollbarInfo,
        vertical_bar: &ScrollbarInfo,
        max_scroll: Point<Px>,
        control_size: Size<Px>,
    ) {
        let mut scroll = dynamic_scroll.get();
        if self.horizontal {
            scroll.x = self.update_bar(
                location.x,
                self.start.x,
                max_scroll.x,
                self.start_scroll.x,
                horizontal_bar,
                control_size.width,
            );
        } else {
            scroll.y = self.update_bar(
                location.y,
                self.start.y,
                max_scroll.y,
                self.start_scroll.y,
                vertical_bar,
                control_size.height,
            );
        }
        dynamic_scroll.set(scroll);
    }

    fn update_bar(
        &self,
        location: Px,
        start: Px,
        max_scroll: Px,
        start_scroll: Px,
        bar: &ScrollbarInfo,
        control_size: Px,
    ) -> Px {
        if self.in_bar {
            let dy = location - start;
            if dy == 0 {
                start_scroll
            } else {
                (start_scroll
                    - Px::from(
                        dy.into_float() / (control_size - bar.size).into_float()
                            * bar.amount_hidden.into_float(),
                    ))
                .clamp(max_scroll, Px::ZERO)
            }
        } else {
            max_scroll
                * ((location - bar.size / 2).max(Px::ZERO).into_float()
                    / (control_size - bar.size).into_float())
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
