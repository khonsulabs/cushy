//! A container that scrolls its contents on a virtual surface.
use std::borrow::Cow;
use std::time::Duration;

use intentional::Cast;
use kludgine::app::winit::event::{DeviceId, MouseScrollDelta, TouchPhase};
use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::{
    FloatConversion, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size,
};
use kludgine::shapes::Shape;
use kludgine::Color;

use crate::animation::{AnimationHandle, AnimationTarget, IntoAnimate, Spawn, ZeroToOne};
use crate::context::{AsEventContext, EventContext, LayoutContext};
use crate::styles::components::{EasingIn, EasingOut, LineHeight};
use crate::styles::{
    ComponentDefinition, ComponentGroup, ComponentName, Dimension, NamedComponent,
};
use crate::value::Dynamic;
use crate::widget::{EventHandling, MakeWidget, Widget, WidgetRef, HANDLED, IGNORED};
use crate::{ConstraintLimit, Name};

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
        let styles = context.query_styles(&[&EasingIn, &EasingOut]);
        self.scrollbar_opacity_animation = self
            .scrollbar_opacity
            .transition_to(ZeroToOne::ONE)
            .over(Duration::from_millis(300))
            .with_easing(styles.get_or_default(&EasingIn))
            .and_then(Duration::from_secs(1))
            .and_then(
                self.scrollbar_opacity
                    .transition_to(ZeroToOne::ZERO)
                    .over(Duration::from_millis(300))
                    .with_easing(styles.get_or_default(&EasingOut)),
            )
            .spawn();
    }
}

impl Widget for Scroll {
    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn hover(&mut self, _location: Point<Px>, context: &mut EventContext<'_, '_>) {
        self.show_scrollbars(context);
    }

    fn unhover(&mut self, context: &mut EventContext<'_, '_>) {
        self.scrollbar_opacity_animation = self
            .scrollbar_opacity
            .transition_to(ZeroToOne::ZERO)
            .over(Duration::from_millis(300))
            .with_easing(context.query_style(&EasingOut))
            .spawn();
    }

    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_, '_>) {
        context.redraw_when_changed(&self.scrollbar_opacity);
        let Some(visible_rect) = context.graphics.visible_rect() else {
            return;
        };
        let visible_bottom_right = visible_rect.into_signed().extent();

        let managed = self.contents.mounted(&mut context.as_event_context());
        context.for_other(&managed).redraw();

        if self.horizontal_bar.amount_hidden > 0 {
            context.graphics.draw_shape(
                &Shape::filled_rect(
                    Rect::new(
                        Point::new(
                            self.horizontal_bar.offset,
                            self.control_size.height - self.bar_width,
                        ),
                        Size::new(self.horizontal_bar.size, self.bar_width),
                    ),
                    Color::new_f32(1.0, 1.0, 1.0, *self.scrollbar_opacity.get()),
                ),
                Point::default(),
                None,
                None,
            );
        }

        if self.vertical_bar.amount_hidden > 0 {
            context.graphics.draw_shape(
                &Shape::filled_rect(
                    Rect::new(
                        Point::new(
                            visible_bottom_right.x - self.bar_width,
                            self.vertical_bar.offset,
                        ),
                        Size::new(self.bar_width, self.vertical_bar.size),
                    ),
                    Color::new_f32(1.0, 1.0, 1.0, *self.scrollbar_opacity.get()),
                ),
                Point::default(),
                None,
                None,
            );
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let styles = context.query_styles(&[&ScrollBarThickness, &LineHeight]);
        self.bar_width = styles
            .get_or_default(&ScrollBarThickness)
            .into_px(context.graphics.scale());
        self.line_height = styles
            .get_or_default(&LineHeight)
            .into_px(context.graphics.scale());

        let (mut scroll, current_max_scroll) = self.constrain_scroll();

        let control_size =
            Size::<UPx>::new(available_space.width.max(), available_space.height.max())
                .into_signed();
        let max_extents = Size::new(
            if self.enabled.x {
                ConstraintLimit::ClippedAfter(UPx::MAX - scroll.x.into_unsigned())
            } else {
                ConstraintLimit::Known(control_size.width.into_unsigned())
            },
            if self.enabled.y {
                ConstraintLimit::ClippedAfter(UPx::MAX - scroll.y.into_unsigned())
            } else {
                ConstraintLimit::Known(control_size.height.into_unsigned())
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
            Px(0)
        };

        self.vertical_bar =
            scrollbar_region(scroll.y, new_content_size.height, control_size.height);
        let max_scroll_y = if self.enabled.y {
            -self.vertical_bar.amount_hidden
        } else {
            Px(0)
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
        self.scroll.update(scroll);
        self.control_size = control_size;
        self.content_size = new_content_size;

        let region = Rect::new(
            scroll,
            self.content_size
                .min(Size::new(Px::MAX, Px::MAX) - scroll.max(Point::default())),
        );
        context.set_child_layout(&managed, region);

        Size::new(available_space.width.max(), available_space.height.max())
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

/// The thickness that scrollbars are drawn with.
pub struct ScrollBarThickness;

impl ComponentDefinition for ScrollBarThickness {
    type ComponentType = Dimension;

    fn default_value(&self) -> Self::ComponentType {
        Dimension::Lp(Lp::points(7))
    }
}

impl NamedComponent for ScrollBarThickness {
    fn name(&self) -> Cow<'_, ComponentName> {
        Cow::Owned(ComponentName::named::<Scroll>("text_size"))
    }
}

impl ComponentGroup for Scroll {
    fn name() -> Name {
        Name::new("Scroll")
    }
}
