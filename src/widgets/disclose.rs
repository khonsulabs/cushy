//! A widget that hides/shows associated content.

use std::time::Duration;

use figures::units::{Lp, Px, UPx};
use figures::{Angle, IntoSigned, Point, Rect, Round, ScreenScale, Size, Zero};
use kludgine::app::winit::window::CursorIcon;
use kludgine::shapes::{PathBuilder, StrokeOptions};
use kludgine::{Color, DrawableExt};

use super::button::{ButtonActiveBackground, ButtonBackground, ButtonHoverBackground};
use crate::animation::{AnimationHandle, AnimationTarget, Spawn};
use crate::context::{EventContext, LayoutContext};
use crate::reactive::value::{Destination, Dynamic, IntoDynamic, IntoValue, Source, Value};
use crate::styles::components::{HighlightColor, IntrinsicPadding, LineHeight, OutlineColor};
use crate::styles::Dimension;
use crate::widget::{
    EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetInstance, WidgetLayout, WidgetRef,
    WidgetTag, HANDLED, IGNORED,
};
use crate::window::DeviceId;
use crate::ConstraintLimit;

/// A widget that hides and shows another widget.
pub struct Disclose {
    contents: WidgetInstance,
    label: Option<WidgetInstance>,
    collapsed: Value<bool>,
}

impl Disclose {
    /// Returns a new widget that allows hiding and showing `contents`.
    #[must_use]
    pub fn new(contents: impl MakeWidget) -> Self {
        Self {
            contents: contents.make_widget(),
            label: None,
            collapsed: Value::Constant(true),
        }
    }

    /// Sets `label` as a clickable label for this widget.
    #[must_use]
    pub fn labelled_by(mut self, label: impl MakeWidget) -> Self {
        self.label = Some(label.make_widget());
        self
    }

    /// Sets this widget's collapsed value.
    ///
    /// If a `Value::Constant` is provided, it is used as the initial collapse
    /// state. If a `Value::Dynamic` is provided, it will be updated when the
    /// contents are shown and hidden.
    #[must_use]
    pub fn collapsed(mut self, collapsed: impl IntoValue<bool>) -> Self {
        self.collapsed = collapsed.into_value();
        self
    }
}

impl MakeWidgetWithTag for Disclose {
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance {
        let collapsed = self.collapsed.into_dynamic();

        DiscloseIndicator::new(collapsed.clone(), self.label, self.contents).make_with_tag(tag)
    }
}

#[derive(Debug)]
struct DiscloseIndicator {
    label: Option<WidgetRef>,
    contents: WidgetRef,
    collapsed: Dynamic<bool>,
    hovering_indicator: bool,
    target_colors: Option<(Color, Color)>,
    color_animation: AnimationHandle,
    color: Dynamic<Color>,
    stroke_color: Dynamic<Color>,
    angle: Dynamic<Angle>,
    mouse_buttons_pressed: usize,
}

fn collapse_angle(collapsed: bool) -> Angle {
    if collapsed {
        Angle::degrees(0)
    } else {
        Angle::degrees(90)
    }
}

impl DiscloseIndicator {
    fn new(
        collapsed: Dynamic<bool>,
        label: Option<WidgetInstance>,
        contents: WidgetInstance,
    ) -> Self {
        let angle = Dynamic::new(collapse_angle(collapsed.get()));

        let mut _angle_animation = AnimationHandle::default();
        angle.set_source({
            let angle = angle.clone();
            collapsed.for_each(move |collapsed| {
                _angle_animation = angle
                    .transition_to(collapse_angle(*collapsed))
                    .over(Duration::from_millis(125))
                    .spawn();
            })
        });

        Self {
            contents: WidgetRef::new(contents.collapse_vertically(collapsed.clone())),
            collapsed,
            hovering_indicator: false,
            label: label.map(WidgetRef::new),
            target_colors: None,
            color: Dynamic::new(Color::CLEAR_WHITE),
            stroke_color: Dynamic::new(Color::CLEAR_WHITE),
            color_animation: AnimationHandle::default(),
            angle,
            mouse_buttons_pressed: 0,
        }
    }

    fn effective_colors(
        &mut self,
        context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>,
    ) -> (Color, Color) {
        let current_color = if context.active() {
            context.get(&ButtonActiveBackground)
        } else if self.hovering_indicator {
            context.get(&ButtonHoverBackground)
        } else {
            context.get(&ButtonBackground)
        };
        let stroke_color = if self.hovering_indicator {
            context.get(&OutlineColor)
        } else if context.focused(true) {
            context.get(&HighlightColor)
        } else {
            context.get(&OutlineColor).with_alpha(0)
        };
        let target_colors = (current_color, stroke_color);
        if self.target_colors.is_none() {
            self.target_colors = Some(target_colors);
            self.color.set(current_color);
            self.stroke_color.set(stroke_color);
        } else if self.target_colors != Some(target_colors) {
            self.target_colors = Some(target_colors);
            self.color_animation = (
                self.color.transition_to(current_color),
                self.stroke_color.transition_to(stroke_color),
            )
                .over(Duration::from_millis(125))
                .spawn();
        }

        (
            self.color.get_tracking_redraw(context),
            self.stroke_color.get_tracking_redraw(context),
        )
    }
}

impl Widget for DiscloseIndicator {
    fn unmounted(&mut self, context: &mut EventContext<'_>) {
        if let Some(label) = &mut self.label {
            label.unmount_in(context);
        }
        self.contents.unmount_in(context);
    }

    fn redraw(&mut self, context: &mut crate::context::GraphicsContext<'_, '_, '_, '_>) {
        let angle = self.angle.get_tracking_redraw(context);
        let (color, stroke_color) = self.effective_colors(context);
        let size = context
            .get(&IndicatorSize)
            .into_px(context.gfx.scale())
            .round();
        let stroke_options =
            StrokeOptions::px_wide(Lp::points(1).into_px(context.gfx.scale()).round())
                .colored(stroke_color);

        let radius = ((size - stroke_options.line_width) / 2).round();
        let pt1 = Point::new(radius, Px::ZERO).rotate_by(Angle::degrees(0));
        let pt2 = Point::new(radius, Px::ZERO).rotate_by(Angle::degrees(120));
        let pt3 = Point::new(radius, Px::ZERO).rotate_by(Angle::degrees(240));

        let path = PathBuilder::new(pt1).line_to(pt2).line_to(pt3).close();

        let indicator_layout_height = if let Some(label) = &mut self.label {
            let label = label.mounted(context);
            context.for_other(&label).redraw();
            label
                .last_layout()
                .unwrap_or_default()
                .size
                .height
                .max(size)
        } else {
            size
        };

        let center = (Point::new(size, indicator_layout_height) / 2).round();
        context
            .gfx
            .draw_shape(path.fill(color).translate_by(center).rotate_by(angle));

        context.gfx.draw_shape(
            path.stroke(stroke_options)
                .translate_by(center)
                .rotate_by(angle),
        );

        let contents = self.contents.mounted(context);
        context.for_other(&contents).redraw();
    }

    fn layout(
        &mut self,
        mut available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WidgetLayout {
        let indicator_size = context
            .get(&IndicatorSize)
            .into_upx(context.gfx.scale())
            .round();
        let padding = context
            .get(&IntrinsicPadding)
            .into_upx(context.gfx.scale())
            .round();

        let content_inset = indicator_size + padding;
        available_space.width -= content_inset;

        let label_layout = if let Some(label) = &mut self.label {
            let label = label.mounted(context);
            let label_layout = context.for_other(&label).layout(available_space);
            let label_vertical_offset = if label_layout.size.height < indicator_size {
                (indicator_size - label_layout.size.height).round()
            } else {
                UPx::ZERO
            };
            context.set_child_layout(
                &label,
                Rect::new(
                    Point::new(content_inset, label_vertical_offset),
                    label_layout.size,
                )
                .into_signed(),
            );
            WidgetLayout {
                size: Size::new(
                    label_layout.size.width,
                    label_layout.size.height.max(indicator_size),
                ),
                baseline: label_layout.baseline,
            }
        } else {
            WidgetLayout::ZERO
        };

        let content_vertical_offset = if label_layout.size.height > 0 {
            label_layout.size.height + padding
        } else {
            label_layout.size.height
        };

        available_space.height -= content_vertical_offset;

        let contents = self.contents.mounted(context);
        let content_layout = context.for_other(&contents).layout(available_space);
        let content_rect = Rect::new(
            Point::new(content_inset, content_vertical_offset),
            content_layout.size,
        );
        context.set_child_layout(&contents, content_rect.into_signed());
        WidgetLayout {
            size: Size::new(
                content_inset + content_rect.size.width.max(label_layout.size.width),
                indicator_size.max(content_rect.origin.y + content_rect.size.height),
            ),
            baseline: label_layout.baseline,
        }
    }

    fn accept_focus(&mut self, _context: &mut EventContext<'_>) -> bool {
        true
    }

    fn focus(&mut self, context: &mut EventContext<'_>) {
        context.set_needs_redraw();
    }

    fn blur(&mut self, context: &mut EventContext<'_>) {
        context.set_needs_redraw();
    }

    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> bool {
        let size = context
            .get(&IndicatorSize)
            .into_px(context.kludgine.scale())
            .round();
        if let Some(label) = &mut self.label {
            let layout = label.mounted(context).last_layout().unwrap_or_default();

            location.y < size.max(layout.size.height)
        } else {
            location.x < size && location.y < size
        }
    }

    fn hover(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> Option<CursorIcon> {
        let hovering = self.hit_test(location, context);
        if self.hovering_indicator != hovering {
            context.set_needs_redraw();
            self.hovering_indicator = hovering;
        }

        hovering.then_some(CursorIcon::Pointer)
    }

    fn unhover(&mut self, context: &mut EventContext<'_>) {
        if self.hovering_indicator {
            self.hovering_indicator = false;
            context.set_needs_redraw();
        }
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        if self.hit_test(location, context) {
            self.mouse_buttons_pressed += 1;
            self.activate(context);
            context.focus();
            HANDLED
        } else {
            IGNORED
        }
    }

    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        _device_id: DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_>,
    ) {
        self.mouse_buttons_pressed -= 1;
        if self.mouse_buttons_pressed == 0 {
            self.deactivate(context);
            self.collapsed.toggle();
        }
        let hovering = location.is_some_and(|location| self.hit_test(location, context));
        if hovering != self.hovering_indicator {
            self.hovering_indicator = hovering;
            context.set_needs_redraw();
        }
    }

    fn activate(&mut self, _context: &mut EventContext<'_>) {
        if self.mouse_buttons_pressed == 0 {
            self.collapsed.toggle();
        }
    }
}

define_components! {
    Disclose {
        /// The size to render a [`Disclose`] indicator.
        IndicatorSize(Dimension, "size", @LineHeight)
    }
}
