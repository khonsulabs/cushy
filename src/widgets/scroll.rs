//! A container that scrolls its contents on a virtual surface.

use std::mem;
use std::time::Duration;

use figures::units::{Lp, Px, UPx};
use figures::{FloatConversion, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size, Zero};
use intentional::Cast;
use kempt::Set;
use kludgine::app::winit::event::{MouseScrollDelta, TouchPhase};
use kludgine::app::winit::window::CursorIcon;
use kludgine::shapes::Shape;
use kludgine::Color;

use crate::animation::{AnimationHandle, AnimationTarget, IntoAnimate, Spawn, ZeroToOne};
use crate::context::{AsEventContext, EventContext, LayoutContext};
use crate::styles::components::{EasingIn, EasingOut, LineHeight};
use crate::styles::Dimension;
use crate::value::{
    Destination, Dynamic, DynamicReader, IntoDynamic, IntoValue, MapEachCloned, Source, Value,
};
use crate::widget::{EventHandling, MakeWidget, Widget, WidgetId, WidgetRef, HANDLED, IGNORED};
use crate::window::DeviceId;
use crate::ConstraintLimit;

// TODO is this useful enough to make public?
#[derive(Debug)]
pub(crate) struct OwnedWidget<W>(OwnedWidgetState<W>);

#[derive(Debug)]
enum OwnedWidgetState<W> {
    Unmade(W),
    Making,
    Made(WidgetRef),
}

impl<W> OwnedWidget<W>
where
    W: Widget,
{
    pub const fn new(widget: W) -> Self {
        Self(OwnedWidgetState::Unmade(widget))
    }

    // pub fn make(&mut self) -> &WidgetInstance {
    //     self.make_if_needed().widget()
    // }

    pub fn make_if_needed(&mut self) -> &mut WidgetRef {
        if matches!(&self.0, OwnedWidgetState::Unmade(_)) {
            let OwnedWidgetState::Unmade(widget) =
                mem::replace(&mut self.0, OwnedWidgetState::Making)
            else {
                unreachable!("just matched")
            };

            self.0 = OwnedWidgetState::Made(WidgetRef::new(widget));
        }

        self.expect_made_mut()
    }

    pub fn expect_made(&self) -> &WidgetRef {
        let OwnedWidgetState::Made(widget) = &self.0 else {
            unreachable!("widget made")
        };
        widget
    }

    pub fn expect_made_mut(&mut self) -> &mut WidgetRef {
        let OwnedWidgetState::Made(widget) = &mut self.0 else {
            unreachable!("widget made")
        };
        widget
    }

    // pub fn expect_unmade(&self) -> &W {
    //     let OwnedWidgetState::Unmade(widget) = &self.0 else {
    //         unreachable!("widget unmade")
    //     };
    //     widget
    // }

    pub fn expect_unmade_mut(&mut self) -> &mut W {
        let OwnedWidgetState::Unmade(widget) = &mut self.0 else {
            unreachable!("widget unmade")
        };
        widget
    }
}

impl<T> Default for OwnedWidget<T>
where
    T: Widget + Default,
{
    fn default() -> Self {
        Self::new(T::default())
    }
}

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
    max_scroll: DynamicReader<Point<UPx>>,
    vertical_widget: OwnedWidget<ScrollBar>,
    horizontal_widget: OwnedWidget<ScrollBar>,
}

#[derive(Debug)]
struct OpacityAnimationState {
    hovering: Set<WidgetId>,
    is_hide: bool,
    will_hide: bool,
    handle: AnimationHandle,
}

impl Scroll {
    /// Returns a new scroll widget containing `contents`.
    fn construct(contents: impl MakeWidget, enabled: Point<bool>) -> Self {
        let scroll = Dynamic::<Point<UPx>>::default();
        let content_size = Dynamic::<Size<UPx>>::default();
        let x = scroll.map_each_cloned(|scroll| scroll.x);
        x.for_each_cloned({
            let scroll = scroll.clone();
            move |x| {
                if let Ok(mut scroll) = scroll.try_lock() {
                    if scroll.x != x {
                        scroll.x = x;
                    }
                }
            }
        })
        .persist();
        let horizontal = ScrollBar::new(content_size.map_each_cloned(|size| size.width), x, false);

        let y = scroll.map_each_cloned(|scroll| scroll.y);
        y.for_each_cloned({
            let scroll = scroll.clone();
            move |y| {
                if let Ok(mut scroll) = scroll.try_lock() {
                    if scroll.y != y {
                        scroll.y = y;
                    }
                }
            }
        })
        .persist();
        let mut vertical =
            ScrollBar::new(content_size.map_each_cloned(|size| size.height), y, true);
        vertical.synchronize_visibility_with(&horizontal);
        let max_scroll = (&horizontal.max_scroll(), &vertical.max_scroll())
            .map_each_cloned(|(x, y)| Point::new(x, y))
            .into_reader();

        Self {
            contents: WidgetRef::new(contents),
            enabled,
            content_size,
            control_size: Dynamic::new(Size::default()),
            scroll,
            max_scroll,
            horizontal_widget: OwnedWidget::new(horizontal),
            vertical_widget: OwnedWidget::new(vertical),
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
        let preserve = preserve.into_value();
        self.vertical_widget.expect_unmade_mut().preserve_max_scroll = preserve.clone();
        self.horizontal_widget
            .expect_unmade_mut()
            .preserve_max_scroll = preserve;
        self
    }

    /// Returns a reader for the maximum scroll value.
    ///
    /// This represents the maximum amount that the scroll can be moved by.
    #[must_use]
    pub const fn max_scroll(&self) -> &DynamicReader<Point<UPx>> {
        &self.max_scroll
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

    fn show_scrollbars(&mut self, context: &mut EventContext<'_>) {
        let mut horizontal = self.horizontal_widget.expect_made_mut().widget().lock();
        horizontal
            .downcast_mut::<ScrollBar>()
            .expect("a ScrollBar")
            .show(context);
    }

    fn hide_scrollbars(&mut self, context: &mut EventContext<'_>) {
        let mut horizontal = self.horizontal_widget.expect_made_mut().widget().lock();
        horizontal
            .downcast_mut::<ScrollBar>()
            .expect("a ScrollBar")
            .hide(context);
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
        let contents = self.contents.mounted(&mut context.as_event_context());
        context.for_other(&contents).redraw();
        if self.enabled.x {
            let horizontal = self
                .horizontal_widget
                .expect_made_mut()
                .mounted(&mut context.as_event_context());
            context.for_other(&horizontal).redraw();
        }
        if self.enabled.y {
            let vertical = self
                .vertical_widget
                .expect_made_mut()
                .mounted(&mut context.as_event_context());
            context.for_other(&vertical).redraw();
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
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
        let contents = self.contents.mounted(&mut context.as_event_context());
        let new_content_size = context.for_other(&contents).layout(max_extents);
        self.content_size.set(new_content_size);

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

        let horizontal = self
            .horizontal_widget
            .make_if_needed()
            .mounted(&mut context.as_event_context());
        let layout = context.for_other(&horizontal).layout(available_space);
        context.set_child_layout(
            &horizontal,
            Rect::new(
                Point::new(
                    Px::ZERO,
                    max_extents
                        .height
                        .fit_measured(new_control_size.height)
                        .saturating_sub(layout.height)
                        .into_signed(),
                ),
                layout.into_signed(),
            ),
        );
        let vertical = self
            .vertical_widget
            .make_if_needed()
            .mounted(&mut context.as_event_context());
        let layout = context.for_other(&vertical).layout(available_space);
        context.set_child_layout(
            &vertical,
            Rect::new(
                Point::new(
                    max_extents
                        .width
                        .fit_measured(new_control_size.width)
                        .saturating_sub(layout.width)
                        .into_signed(),
                    Px::ZERO,
                ),
                layout.into_signed(),
            ),
        );
        let scroll = self.scroll.get_tracking_invalidate(context);

        self.control_size.set(new_control_size);

        let region = Rect::new(
            -scroll.into_signed(),
            new_content_size
                .min(Size::new(UPx::MAX, UPx::MAX) - scroll.max(Point::default()))
                .into_signed(),
        );
        context.set_child_layout(&contents, region);

        new_control_size
    }

    fn mouse_wheel(
        &mut self,
        _device_id: DeviceId,
        delta: MouseScrollDelta,
        _phase: TouchPhase,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        let mut handled = false;
        {
            let mut vertical = self.vertical_widget.expect_made().widget().lock();
            handled |= vertical
                .downcast_mut::<ScrollBar>()
                .expect("a ScrollBar")
                .mouse_wheel(delta, context)
                .is_break();
            let mut horizontal = self.horizontal_widget.expect_made().widget().lock();
            handled |= horizontal
                .downcast_mut::<ScrollBar>()
                .expect("a ScrollBar")
                .mouse_wheel(delta, context)
                .is_break();
        }
        if handled {
            self.show_scrollbars(context);
            context.set_needs_redraw();

            HANDLED
        } else {
            IGNORED
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
    start: Px,
    start_scroll: UPx,
    in_bar: bool,
}

impl DragInfo {
    fn update(
        &self,
        location: Px,
        dynamic_scroll: &Dynamic<UPx>,
        info: &ScrollbarInfo,
        max_scroll: UPx,
        control_size: UPx,
    ) {
        let scroll = self.update_bar(
            location,
            self.start,
            max_scroll,
            self.start_scroll,
            info,
            control_size,
        );

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

/// A draggable bar that is used to scroll a region.
#[derive(Debug)]
pub struct ScrollBar {
    content_size: Dynamic<UPx>,
    last_content_size: UPx,
    scroll: Dynamic<UPx>,
    preserve_max_scroll: Value<bool>,
    max_scroll: Dynamic<UPx>,
    bar_width: UPx,
    control_size: UPx,
    line_height: UPx,
    vertical: bool,
    info: ScrollbarInfo,
    scrollbar_opacity: Dynamic<ZeroToOne>,
    scrollbar_opacity_animation: Dynamic<OpacityAnimationState>,
    drag: DragInfo,
}

impl ScrollBar {
    /// Creates a new scroll bar that updates `scroll_by` to scroll across
    /// `content_size`.
    ///
    /// If `vertical` this bar will draw a bar from the top of the widget to the
    /// bottom of the widget. Otherwise, the bar will be drawn from the left to
    /// the right of the widget.
    pub fn new(
        content_size: impl IntoDynamic<UPx>,
        scroll_by: impl IntoDynamic<UPx>,
        vertical: bool,
    ) -> Self {
        Self {
            content_size: content_size.into_dynamic(),
            scroll: scroll_by.into_dynamic(),
            preserve_max_scroll: Value::Constant(true),
            max_scroll: Dynamic::new(UPx::ZERO),
            bar_width: UPx::ZERO,
            line_height: UPx::ZERO,
            control_size: UPx::ZERO,
            vertical,
            info: ScrollbarInfo::default(),
            scrollbar_opacity: Dynamic::default(),
            scrollbar_opacity_animation: Dynamic::new(OpacityAnimationState {
                handle: AnimationHandle::new(),
                will_hide: true,
                is_hide: true,
                hovering: Set::new(),
            }),
            drag: DragInfo::default(),
            last_content_size: UPx::ZERO,
        }
    }

    /// Sets whether the scroll view will stay scrolled to the maximum when a
    /// child is resized.
    ///
    /// When enabled, this setting allows the scroll view to remain scrolled to
    /// the bottom or to the right when its contents grow. The default value for
    /// this setting is `true`.
    #[must_use]
    pub fn preserve_max_scroll(mut self, preserve_max_scroll: impl IntoValue<bool>) -> Self {
        self.preserve_max_scroll = preserve_max_scroll.into_value();
        self
    }

    /// Returns a reader for the maximum scroll value.
    ///
    /// This represents the maximum amount that the scroll can be moved by.
    #[must_use]
    pub fn max_scroll(&self) -> DynamicReader<UPx> {
        self.max_scroll.create_reader()
    }

    /// Applies the delta from a mouse wheel event.
    ///
    /// Returns whether the event resulted in any scroll.
    #[must_use]
    pub fn mouse_wheel(
        &mut self,
        delta: MouseScrollDelta,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        let amount = match delta {
            MouseScrollDelta::LineDelta(x, y) => Point::new(x, y) * self.line_height.into_float(),
            MouseScrollDelta::PixelDelta(px) => Point::new(px.x.cast(), px.y.cast()),
        };
        let amount = if self.vertical { amount.y } else { amount.x };

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
            self.show(context);
            HANDLED
        }
    }

    fn constrained_scroll(scroll: UPx, max_scroll: UPx) -> UPx {
        scroll.min(max_scroll)
    }

    /// Updates this scroll bar to synchronize its visibility with `other`.
    pub fn synchronize_visibility_with(&mut self, other: &ScrollBar) {
        self.scrollbar_opacity = other.scrollbar_opacity.clone();
        self.scrollbar_opacity_animation = other.scrollbar_opacity_animation.clone();
    }

    /// Shows this scroll bar, automatically hiding after a short delay.
    pub fn show(&mut self, context: &mut EventContext<'_>) {
        let mut animation_state = self.scrollbar_opacity_animation.lock();
        let should_hide = self.drag.mouse_buttons_down == 0 && animation_state.hovering.is_empty();
        if animation_state.is_hide
            || should_hide != animation_state.will_hide
            || animation_state.handle.is_complete()
        {
            let current_opacity = self.scrollbar_opacity.get();
            let transition_time = *current_opacity.one_minus() / 4.;
            let animation = self
                .scrollbar_opacity
                .transition_to(ZeroToOne::ONE)
                .over(Duration::from_secs_f32(transition_time))
                .with_easing(context.get(&EasingIn));

            animation_state.is_hide = false;
            animation_state.will_hide = should_hide;
            animation_state.handle = if should_hide {
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

    /// Hides the scroll bar, if it can be hidden.
    pub fn hide(&mut self, context: &mut EventContext<'_>) {
        let mut animation_state = self.scrollbar_opacity_animation.lock();
        if self.drag.mouse_buttons_down == 0
            && !animation_state.will_hide
            && animation_state.hovering.is_empty()
        {
            animation_state.is_hide = true;
            animation_state.will_hide = true;
            animation_state.handle = self
                .scrollbar_opacity
                .transition_to(ZeroToOne::ZERO)
                .over(Duration::from_millis(300))
                .with_easing(context.get(&EasingOut))
                .spawn();
        }
    }
}

impl Widget for ScrollBar {
    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>) {
        let scroll = self.scroll.get_tracking_redraw(context);
        let content_size = self.content_size.get_tracking_redraw(context);
        let control_size = context.gfx.region().size.into_unsigned();
        let scrolled_to_end = scroll == self.info.amount_hidden;

        self.control_size = if self.vertical {
            control_size.height
        } else {
            control_size.width
        };
        self.info = scrollbar_region(scroll, content_size, self.control_size);
        let mut constrained = Self::constrained_scroll(scroll, self.info.amount_hidden);

        // Preserve the current scroll if the widget has resized
        if scrolled_to_end
            && self.last_content_size != 0
            && self.last_content_size != content_size
            && self.preserve_max_scroll.get()
        {
            constrained = self.info.amount_hidden;
        }
        self.last_content_size = content_size;
        self.scroll.set(constrained);
        self.max_scroll.set(self.info.amount_hidden);

        let opacity = self.scrollbar_opacity.get_tracking_redraw(context);
        if context.enabled() && self.info.amount_hidden > 0 && opacity > 0. {
            let rect = if self.vertical {
                Rect::new(
                    Point::new(control_size.width - self.bar_width, self.info.offset),
                    Size::new(self.bar_width, self.info.size),
                )
            } else {
                Rect::new(
                    Point::new(UPx::ZERO, control_size.height - self.bar_width),
                    Size::new(self.info.size, self.bar_width),
                )
            };
            context.gfx.draw_shape(&Shape::filled_rect(
                rect.into_signed(), // See https://github.com/khonsulabs/cushy/issues/186
                Color::new_f32(1.0, 1.0, 1.0, *opacity),
            ));
        }
    }

    fn hit_test(&mut self, _location: Point<Px>, context: &mut EventContext<'_>) -> bool {
        self.max_scroll.get() > 0 && context.enabled()
    }

    fn hover(
        &mut self,
        _location: Point<Px>,
        context: &mut EventContext<'_>,
    ) -> Option<CursorIcon> {
        self.scrollbar_opacity_animation
            .lock()
            .hovering
            .insert(context.widget().id());
        self.show(context);

        None
    }

    fn unhover(&mut self, context: &mut EventContext<'_>) {
        self.scrollbar_opacity_animation
            .lock()
            .hovering
            .remove(&context.widget().id());
        self.hide(context);
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

        if self.vertical {
            Size::new(self.bar_width, available_space.height.max())
        } else {
            Size::new(available_space.width.max(), self.bar_width)
        }
    }

    fn mouse_wheel(
        &mut self,
        _device_id: DeviceId,
        delta: MouseScrollDelta,
        _phase: TouchPhase,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        self.mouse_wheel(delta, context)
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        if self.max_scroll.get().is_zero() || !context.enabled() {
            return IGNORED;
        }
        self.drag.start = if self.vertical {
            location.y
        } else {
            location.x
        };
        self.drag.start_scroll = self.scroll.get();
        let relative = self.drag.start - self.info.offset.into_signed();
        self.drag.in_bar = relative >= 0 && relative < self.info.size;

        // If we clicked in the open area, we need to jump to the new location
        // immediately.
        if !self.drag.in_bar {
            self.drag.update(
                self.drag.start,
                &self.scroll,
                &self.info,
                self.max_scroll.get(),
                self.control_size,
            );
        }

        self.drag.mouse_buttons_down += 1;
        self.show(context);

        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        _context: &mut EventContext<'_>,
    ) {
        let offset = if self.vertical {
            location.y
        } else {
            location.x
        };
        self.drag.update(
            offset,
            &self.scroll,
            &self.info,
            self.max_scroll.get(),
            self.control_size,
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
                let offset = if self.vertical {
                    location.y
                } else {
                    location.x
                };
                offset >= 0 && offset < self.control_size
            }) {
                self.scrollbar_opacity_animation.lock().handle.clear();
                self.show(context);
            } else {
                self.hide(context);
            }
        }
    }
}
