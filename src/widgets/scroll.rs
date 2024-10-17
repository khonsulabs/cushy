//! A container that scrolls its contents on a virtual surface.
use std::time::{Duration, Instant};

use figures::units::{Lp, Px, UPx};
use figures::{FloatConversion, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size, Zero};
use intentional::Cast;
use kludgine::app::winit::event::{MouseScrollDelta, TouchPhase};
use kludgine::app::winit::window::CursorIcon;
use kludgine::shapes::Shape;
use kludgine::Color;

use crate::animation::{AnimationHandle, AnimationTarget, IntoAnimate, Spawn, ZeroToOne};
use crate::context::{AsEventContext, EventContext, LayoutContext};
use crate::styles::components::{EasingIn, EasingOut, LineHeight};
use crate::styles::Dimension;
use crate::value::{Destination, Dynamic, DynamicReader, IntoValue, Source, Value};
use crate::widget::{EventHandling, MakeWidget, Widget, WidgetRef, HANDLED, IGNORED};
use crate::window::DeviceId;
use crate::ConstraintLimit;

/// A widget that supports scrolling its contents.
#[derive(Debug)]
pub struct Scroll {
    contents: WidgetRef,
    content_size: Dynamic<Size<UPx>>,
    control_size: Dynamic<Size<UPx>>,
    /// The current scroll position.
    ///
    /// When a new value is assigned to this, this widget will scroll its
    /// contents. If a value is out of bounds of the maximum scroll, it will be
    /// clamped and this dynamic will be updated with clamped scroll.
    pub scroll: Dynamic<Point<UPx>>,
    enabled: Point<bool>,
    preserve_max_scroll: Value<bool>,
    max_scroll: Dynamic<Point<UPx>>,
    scrollbar_opacity: Dynamic<ZeroToOne>,
    scrollbar_opacity_animation: OpacityAnimationState,
    horizontal_bar: ScrollbarInfo,
    vertical_bar: ScrollbarInfo,
    bar_width: UPx,
    line_height: UPx,
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
            content_size: Dynamic::new(Size::default()),
            control_size: Dynamic::new(Size::default()),
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
            bar_width: UPx::default(),
            line_height: UPx::default(),
            drag: DragInfo::default(),
            preserve_max_scroll: Value::Constant(true),
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

    /// Sets whether the scroll view will stay scrolled to the maximum when a
    /// child is resized.
    ///
    /// When enabled, this setting allows the scroll view to remain scrolled to
    /// the bottom or to the right when its contents grow. The default value for
    /// this setting is `true`.
    #[must_use]
    pub fn preserve_max_scroll(mut self, preserve: impl IntoValue<bool>) -> Self {
        self.preserve_max_scroll = preserve.into_value();
        self
    }

    /// Returns a reader for the maximum scroll value.
    ///
    /// This represents the maximum amount that the scroll can be moved by.
    #[must_use]
    pub fn max_scroll(&self) -> DynamicReader<Point<UPx>> {
        self.max_scroll.create_reader()
    }

    /// Returns a reader for the size of the scrollable area.
    #[must_use]
    pub fn content_size(&self) -> DynamicReader<Size<UPx>> {
        self.content_size.create_reader()
    }

    /// Returns a reader for the size of this Scroll widget.
    #[must_use]
    pub fn control_size(&self) -> DynamicReader<Size<UPx>> {
        self.control_size.create_reader()
    }

    fn constrained_scroll(scroll: Point<UPx>, max_scroll: Point<UPx>) -> Point<UPx> {
        scroll.min(max_scroll)
    }

    fn constrain_scroll(&mut self) -> (Point<UPx>, Point<UPx>) {
        let scroll = self.scroll.get();
        let max_scroll = self.max_scroll.get();
        let clamped = Self::constrained_scroll(scroll, max_scroll);
        if clamped != scroll {
            self.scroll.set(clamped);
        }
        (clamped, max_scroll)
    }

    fn show_scrollbars(&mut self, context: &mut EventContext<'_>) {
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

    fn hide_scrollbars(&mut self, context: &mut EventContext<'_>) {
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
    fn unmounted(&mut self, context: &mut EventContext<'_>) {
        self.contents.unmount_in(context);
    }

    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_>) -> bool {
        true
    }

    fn hover(
        &mut self,
        _location: Point<Px>,
        context: &mut EventContext<'_>,
    ) -> Option<CursorIcon> {
        self.show_scrollbars(context);

        None
    }

    fn unhover(&mut self, context: &mut EventContext<'_>) {
        self.hide_scrollbars(context);
    }

    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>) {
        context.redraw_when_changed(&self.scrollbar_opacity);

        let managed = self.contents.mounted(&mut context.as_event_context());
        context.for_other(&managed).redraw();

        let size = context.gfx.region().size.into_unsigned();

        if self.horizontal_bar.amount_hidden > 0 {
            context.gfx.draw_shape(&Shape::filled_rect(
                Rect::new(
                    Point::new(self.horizontal_bar.offset, size.height - self.bar_width),
                    Size::new(self.horizontal_bar.size, self.bar_width),
                )
                .into_signed(), // See https://github.com/khonsulabs/cushy/issues/186
                Color::new_f32(1.0, 1.0, 1.0, *self.scrollbar_opacity.get()),
            ));
        }

        if self.vertical_bar.amount_hidden > 0 {
            context.gfx.draw_shape(&Shape::filled_rect(
                Rect::new(
                    Point::new(size.width - self.bar_width, self.vertical_bar.offset),
                    Size::new(self.bar_width, self.vertical_bar.size),
                )
                .into_signed(), // See https://github.com/khonsulabs/cushy/issues/186
                Color::new_f32(1.0, 1.0, 1.0, *self.scrollbar_opacity.get()),
            ));
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        self.bar_width = context
            .get(&ScrollBarThickness)
            .into_upx(context.gfx.scale());
        self.line_height = context.get(&LineHeight).into_upx(context.gfx.scale());

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
        let new_content_size = context.for_other(&managed).layout(max_extents);

        let new_control_size = Size::new(
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

        self.horizontal_bar =
            scrollbar_region(scroll.x, new_content_size.width, new_control_size.width);
        let max_scroll_x = if self.enabled.x {
            self.horizontal_bar.amount_hidden
        } else {
            UPx::ZERO
        };

        self.vertical_bar =
            scrollbar_region(scroll.y, new_content_size.height, new_control_size.height);
        let max_scroll_y = if self.enabled.y {
            self.vertical_bar.amount_hidden
        } else {
            UPx::ZERO
        };
        let new_max_scroll = Point::new(max_scroll_x, max_scroll_y);
        if current_max_scroll != new_max_scroll {
            self.max_scroll.set(new_max_scroll);
            scroll = scroll.max(new_max_scroll);
        }

        // This is not tracked on purpose - it's only ever changed in layout
        let content_size = self.content_size.get();
        let control_size = self.control_size.get();

        // Preserve the current scroll if the widget has resized
        if content_size != Size::ZERO && content_size != new_content_size {
            if (content_size.width != new_content_size.width
                || control_size.width != new_control_size.width)
                && scroll.x == current_max_scroll.x
                && self.preserve_max_scroll.get()
            {
                scroll.x = max_scroll_x;
            }

            if (content_size.height != new_content_size.height
                || control_size.height != new_control_size.height)
                && scroll.y == current_max_scroll.y
                && self.preserve_max_scroll.get()
            {
                scroll.y = max_scroll_y;
            }
        }

        // Set the current scroll, but prevent immediately triggering
        // invalidate.
        {
            let mut current_scroll = self.scroll.lock();
            current_scroll.prevent_notifications();
            *current_scroll = scroll;
        }
        context.invalidate_when_changed(&self.scroll);
        self.control_size.set(new_control_size);
        self.content_size.set(new_content_size);

        let region = Rect::new(
            -scroll.into_signed(),
            new_content_size
                .min(Size::new(UPx::MAX, UPx::MAX) - scroll.max(Point::default()))
                .into_signed(),
        );
        context.set_child_layout(&managed, region);

        new_control_size
    }

    fn mouse_wheel(
        &mut self,
        _device_id: DeviceId,
        delta: MouseScrollDelta,
        _phase: TouchPhase,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        let amount = match delta {
            MouseScrollDelta::LineDelta(x, y) => Point::new(x, y) * self.line_height.into_float(),
            MouseScrollDelta::PixelDelta(px) => Point::new(px.x.cast(), px.y.cast()),
        };
        let mut scroll = self.scroll.lock();
        let old_scroll = *scroll;
        let new_scroll = Self::constrained_scroll(
            (scroll.into_signed() - amount.cast::<Px>()).into_unsigned(),
            self.max_scroll.get(),
        );
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
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        let control_size = self.control_size.get();

        let relative_x = (control_size.width.into_signed() - location.x).into_unsigned();
        let in_vertical_area = self.enabled.y && relative_x <= self.bar_width;

        let relative_y = (control_size.height.into_signed() - location.y).into_unsigned();
        let in_horizontal_area = self.enabled.x && relative_y <= self.bar_width;

        if matches!(
            (in_horizontal_area, in_vertical_area),
            (true, true) | (false, false)
        ) {
            return IGNORED;
        }

        self.drag.start = location.into_signed();
        self.drag.start_scroll = self.scroll.get();
        self.drag.horizontal = in_horizontal_area;
        self.drag.in_bar = if in_horizontal_area {
            let relative = location.x - self.horizontal_bar.offset.into_signed();
            relative >= 0 && relative < self.horizontal_bar.size
        } else {
            let relative = location.y - self.vertical_bar.offset.into_signed();
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
                control_size,
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
        _context: &mut EventContext<'_>,
    ) {
        self.drag.update(
            location,
            &self.scroll,
            &self.horizontal_bar,
            &self.vertical_bar,
            self.max_scroll.get(),
            self.control_size.get(),
        );
    }

    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        _device_id: DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_>,
    ) {
        self.drag.mouse_buttons_down -= 1;

        if self.drag.mouse_buttons_down == 0 {
            if location.map_or(false, |location| {
                Rect::from(self.control_size.get())
                    .into_signed()
                    .contains(location)
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
    start_scroll: Point<UPx>,
    horizontal: bool,
    in_bar: bool,
}

impl DragInfo {
    fn update(
        &self,
        location: Point<Px>,
        dynamic_scroll: &Dynamic<Point<UPx>>,
        horizontal_bar: &ScrollbarInfo,
        vertical_bar: &ScrollbarInfo,
        max_scroll: Point<UPx>,
        control_size: Size<UPx>,
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
        max_scroll: UPx,
        start_scroll: UPx,
        bar: &ScrollbarInfo,
        control_size: UPx,
    ) -> UPx {
        if self.in_bar {
            let dy = location - start;
            if dy == 0 {
                start_scroll
            } else {
                (start_scroll.into_signed()
                    + Px::from(
                        dy.into_float() / (control_size - bar.size).into_float()
                            * bar.amount_hidden.into_float(),
                    ))
                .into_unsigned()
                .min(max_scroll)
            }
        } else {
            max_scroll
                * ((location - bar.size.into_signed() / 2)
                    .max(Px::ZERO)
                    .into_float()
                    / (control_size - bar.size).into_float())
        }
    }
}

fn constrain_child(constraint: ConstraintLimit, measured: UPx) -> UPx {
    match constraint {
        ConstraintLimit::Fill(size) => size.min(measured),
        ConstraintLimit::SizeToFit(_) => measured,
    }
}

#[derive(Debug, Default)]
struct ScrollbarInfo {
    offset: UPx,
    amount_hidden: UPx,
    size: UPx,
}

fn scrollbar_region(scroll: UPx, content_size: UPx, control_size: UPx) -> ScrollbarInfo {
    if content_size > control_size {
        let amount_hidden = content_size - control_size;
        let ratio_visible = control_size.into_float() / content_size.into_float();
        let bar_size = control_size * ratio_visible;
        let remaining_area = control_size - bar_size;
        let amount_scrolled = scroll.into_float() / amount_hidden.into_float();
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
