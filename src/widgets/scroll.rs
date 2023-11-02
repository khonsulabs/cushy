//! A container that scrolls its contents on a virtual surface.
use std::borrow::Cow;
use std::time::Duration;

use kludgine::app::winit::event::{DeviceId, MouseScrollDelta, TouchPhase};
use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::utils::lossy_f64_to_f32;
use kludgine::figures::{
    FloatConversion, IntoSigned, IntoUnsigned, Point, Rect, ScreenScale, Size,
};
use kludgine::shapes::Shape;
use kludgine::Color;

use crate::animation::{AnimationHandle, AnimationTarget, IntoAnimate, Spawn, ZeroToOne};
use crate::context::{AsEventContext, EventContext};
use crate::styles::{
    ComponentDefinition, ComponentGroup, ComponentName, Dimension, NamedComponent,
};
use crate::value::Dynamic;
use crate::widget::{EventHandling, MakeWidget, ManagedWidget, Widget, WidgetInstance, HANDLED};
use crate::{ConstraintLimit, Name};

#[derive(Debug)]
enum ChildWidget {
    Instance(WidgetInstance),
    Managed(ManagedWidget),
    Mounting,
}

impl ChildWidget {
    pub fn managed(&mut self, context: &mut EventContext<'_, '_>) -> ManagedWidget {
        if matches!(self, ChildWidget::Instance(_)) {
            let ChildWidget::Instance(instance) = std::mem::replace(self, ChildWidget::Mounting)
            else {
                unreachable!("just matched")
            };
            *self = ChildWidget::Managed(context.push_child(instance));
        }
        let ChildWidget::Managed(managed) = self else {
            unreachable!("always converted")
        };
        managed.clone()
    }
}

/// A widget that supports scrolling its contents.
#[derive(Debug)]
pub struct Scroll {
    contents: ChildWidget,
    content_size: Size<Px>,
    scroll: Point<Px>,
    max_scroll: Point<Px>,
    scrollbar_opacity: Dynamic<ZeroToOne>,
    scrollbar_opacity_animation: AnimationHandle,
}

impl Scroll {
    /// Returns a new scroll widget containing `contents`.
    pub fn new(contents: impl MakeWidget) -> Self {
        Self {
            contents: ChildWidget::Instance(contents.make_widget()),
            content_size: Size::default(),
            scroll: Point::default(),
            max_scroll: Point::default(),
            scrollbar_opacity: Dynamic::default(),
            scrollbar_opacity_animation: AnimationHandle::new(),
        }
    }

    fn constrain_scroll(&mut self) {
        self.scroll = self.scroll.max(self.max_scroll).min(Point::default());
    }
}

impl Widget for Scroll {
    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) -> bool {
        true
    }

    fn hover(&mut self, _location: Point<Px>, _context: &mut EventContext<'_, '_>) {
        self.scrollbar_opacity_animation = self
            .scrollbar_opacity
            .transition_to(ZeroToOne::ONE)
            .over(Duration::from_millis(300))
            .and_then(Duration::from_secs(1))
            .and_then(
                self.scrollbar_opacity
                    .transition_to(ZeroToOne::ZERO)
                    .over(Duration::from_millis(300)),
            )
            .spawn();
    }

    fn unhover(&mut self, _context: &mut EventContext<'_, '_>) {
        self.scrollbar_opacity_animation = self
            .scrollbar_opacity
            .transition_to(ZeroToOne::ZERO)
            .over(Duration::from_millis(300))
            .spawn();
    }

    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_, '_>) {
        context.redraw_when_changed(&self.scrollbar_opacity);
        self.constrain_scroll();
        let Some(visible_rect) = context.graphics.visible_rect() else {
            return;
        };
        let visible_bottom_right = visible_rect.into_signed().extent();
        let styles = context.query_styles(&[&ScrollBarThickness]);
        let bar_width = styles
            .get_or_default(&ScrollBarThickness)
            .into_px(context.graphics.scale());

        let max_extents = Size::new(
            ConstraintLimit::ClippedAfter(UPx::MAX - self.scroll.x.into_unsigned()),
            ConstraintLimit::ClippedAfter(UPx::MAX - self.scroll.y.into_unsigned()),
        );
        let managed = self.contents.managed(&mut context.as_event_context());
        self.content_size = context
            .for_other(&managed)
            .measure(max_extents)
            .into_signed();
        let control_size = context.graphics.region().size;

        let region = Rect::new(
            self.scroll,
            self.content_size
                .min(Size::new(Px::MAX, Px::MAX) - self.scroll.max(Point::default())),
        );
        context.for_child(&managed, region).redraw();

        let horizontal_bar =
            scrollbar_region(self.scroll.x, self.content_size.width, control_size.width);
        self.max_scroll.x = -horizontal_bar.amount_hidden;

        if horizontal_bar.size > 0 {
            context.graphics.draw_shape(
                &Shape::filled_rect(
                    Rect::new(
                        Point::new(horizontal_bar.offset, control_size.height - bar_width),
                        Size::new(horizontal_bar.size, bar_width),
                    ),
                    Color::new_f32(1.0, 1.0, 1.0, *self.scrollbar_opacity.get()),
                ),
                Point::default(),
                None,
                None,
            );
        }

        let vertical_bar =
            scrollbar_region(self.scroll.y, self.content_size.height, control_size.height);
        self.max_scroll.y = -vertical_bar.amount_hidden;

        if vertical_bar.size > 0 {
            context.graphics.draw_shape(
                &Shape::filled_rect(
                    Rect::new(
                        Point::new(visible_bottom_right.x - bar_width, vertical_bar.offset),
                        Size::new(bar_width, vertical_bar.size),
                    ),
                    Color::new_f32(1.0, 1.0, 1.0, *self.scrollbar_opacity.get()),
                ),
                Point::default(),
                None,
                None,
            );
        }
    }

    fn measure(
        &mut self,
        available_space: Size<crate::ConstraintLimit>,
        _context: &mut crate::context::GraphicsContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
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
            /* TODO query line height */
            MouseScrollDelta::LineDelta(x, y) => Point::new(x, y) * 16.0,
            MouseScrollDelta::PixelDelta(px) => {
                Point::new(lossy_f64_to_f32(px.x), lossy_f64_to_f32(px.y))
            }
        };

        self.scroll += amount.cast();
        context.set_needs_redraw();

        // TODO make this only returned handled if we actually scrolled.
        HANDLED
    }
}

#[derive(Default)]
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
