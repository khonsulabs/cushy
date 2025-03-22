//! A widget that indicates a value.

use std::fmt::Debug;
use std::time::Duration;

use figures::units::{Px, UPx};
use figures::{IntoSigned, IntoUnsigned, Point, Rect, Round, ScreenScale, Size, Zero};
use kludgine::app::winit::window::CursorIcon;
use kludgine::Color;

use crate::animation::{AnimationHandle, AnimationTarget, LinearInterpolate, Spawn, ZeroToOne};
use crate::context::{EventContext, GraphicsContext, LayoutContext, WidgetContext};
use crate::reactive::value::{Destination, Dynamic, Source};
use crate::styles::components::{
    AutoFocusableControls, Easing, IntrinsicPadding, WidgetAccentColor,
};
use crate::styles::ColorExt;
use crate::widget::{
    Baseline, EventHandling, MakeWidget, Widget, WidgetLayout, WidgetRef, HANDLED, IGNORED,
};
use crate::window::WindowLocal;
use crate::ConstraintLimit;

/// A type that defines how an [`Indicator`] behaves and is drawn.
pub trait IndicatorBehavior: Send + Debug + 'static {
    /// The type that contains all the colors needed to draw this indicator.
    ///
    /// These colors are transitioned using animations depending on how the user
    /// is interacting with the indicator.
    type Colors: LinearInterpolate + PartialEq + Debug + Send + Sync + Copy + 'static;

    /// Returns the colors desired for the current state of the indicator.
    fn desired_colors(
        &mut self,
        context: &mut WidgetContext<'_>,
        state: IndicatorState,
    ) -> Self::Colors;
    /// Updates the indicator's state from the indicator being activated.
    fn activate(&mut self);
    /// Returns true if the indicator will display empty if the indicator is
    /// activated.
    fn will_be_empty_if_activated(&self) -> bool;
    /// Returns true if the indicator is not currently filled in.
    fn empty(&self) -> bool;
    /// Render the indicator in `region` given the current state and colors.
    ///
    /// - `is_active` is true if the widget is currently being activated by the
    ///   user.
    /// - `colors` is the currently interpolated colors to draw.
    /// - `selected_color` is the color that a selected indicator should be
    ///   drawn using.
    /// - `region` is the region the indicator should be drawn inside
    /// - `context` is the context to draw to.
    fn render(
        &mut self,
        is_active: bool,
        colors: &Self::Colors,
        selected_color: Color,
        region: Rect<Px>,
        context: &mut GraphicsContext<'_, '_, '_, '_>,
    );
    /// Returns the size of this indicator.
    fn size(&self, context: &mut GraphicsContext<'_, '_, '_, '_>) -> WidgetLayout;
}

/// The current state of an [`Indicator`] widget.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub struct IndicatorState {
    /// If true, the mouse is currently above the widget.
    pub hovered: bool,
    /// If true, the user is currently activating the widget.
    pub active: bool,
    /// If true, the indicator has keyboard focus.
    pub focused: bool,
    /// If true, the indicator is enabled.
    pub enabled: bool,
}

#[derive(Debug)]
struct WindowLocalState<Colors> {
    active_colors: Option<Dynamic<Colors>>,
    target_colors: Option<Colors>,
    color_animation: AnimationHandle,
    checkbox_region: Rect<Px>,
    label_region: Rect<Px>,
    focused: bool,
    hovered: bool,
    mouse_buttons_pressed: usize,
    size: Size<UPx>,
}

impl<Colors> Default for WindowLocalState<Colors> {
    fn default() -> Self {
        Self {
            active_colors: None,
            target_colors: None,
            color_animation: AnimationHandle::new(),
            checkbox_region: Rect::ZERO,
            label_region: Rect::ZERO,
            focused: false,
            hovered: false,
            mouse_buttons_pressed: 0,
            size: Size::ZERO,
        }
    }
}

impl<Colors> WindowLocalState<Colors>
where
    Colors: LinearInterpolate + PartialEq + Copy + Send + Sync + 'static,
{
    fn update_colors<B>(
        &mut self,
        context: &mut WidgetContext<'_>,
        immediate: bool,
        behavior: &mut B,
    ) where
        B: IndicatorBehavior<Colors = Colors>,
    {
        let desired_colors = behavior.desired_colors(
            context,
            IndicatorState {
                hovered: self.hovered,
                active: self.hovered && self.mouse_buttons_pressed > 0,
                focused: self.focused,
                enabled: context.enabled(),
            },
        );

        if let Some(active_colors) = &self.active_colors {
            if self.target_colors.as_ref() != Some(&desired_colors) {
                if immediate {
                    active_colors.set(desired_colors);
                    self.color_animation.clear();
                } else {
                    self.color_animation = active_colors
                        .transition_to(desired_colors)
                        .over(Duration::from_millis(150))
                        .with_easing(context.get(&Easing))
                        .spawn();
                }
            }
        } else {
            self.active_colors = Some(Dynamic::new(desired_colors));
        }
        self.target_colors = Some(desired_colors);
    }

    fn hit_test(&self, location: Point<Px>) -> bool {
        self.checkbox_region.contains(location)
            || self.label_region.contains(location)
            || (location.x > self.checkbox_region.size.width
                && location.x < self.label_region.origin.x
                && location.y >= self.checkbox_region.origin.y
                && location.y <= self.checkbox_region.origin.y + self.checkbox_region.size.height)
    }
}

/// A widget that indicates a value.
///
/// This base widget type is used to implement the
/// [`Checkbox`](crate::widgets::Checkbox) and [`Radio`](crate::widgets::Radio)
/// widgets.
#[derive(Debug)]
pub struct Indicator<T>
where
    T: IndicatorBehavior,
{
    behavior: T,
    label: Option<WidgetRef>,
    focusable: bool,
    per_window: WindowLocal<WindowLocalState<T::Colors>>,
}

impl<T> Indicator<T>
where
    T: IndicatorBehavior,
{
    /// Returns a new indicator widget driven by `behavior`.
    pub fn new(behavior: T) -> Self {
        Self {
            behavior,
            label: None,
            focusable: true,
            per_window: WindowLocal::default(),
        }
    }

    /// Displays `label` next to this indicator. When unhandled clicks are
    /// received in the label's area, the indicator will be toggled.
    #[must_use]
    pub fn labelled_by(mut self, label: impl MakeWidget) -> Self {
        self.label = Some(WidgetRef::new(label));
        self
    }

    /// Sets whether this widget should receive keyboard focus.
    #[must_use]
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self
    }

    fn update_colors(&mut self, context: &mut WidgetContext<'_>, immediate: bool) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.update_colors(context, immediate, &mut self.behavior);
    }

    fn clicked(&mut self, context: &WidgetContext<'_>) {
        if context.enabled() {
            self.behavior.activate();
        }
    }
}

impl<T> Widget for Indicator<T>
where
    T: IndicatorBehavior,
{
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        let window_local = self.per_window.entry(context).or_default();
        let is_active = window_local.mouse_buttons_pressed > 0 && window_local.hovered;
        window_local.update_colors(context, false, &mut self.behavior);
        let colors = window_local
            .active_colors
            .as_ref()
            .expect("always present after update_colors")
            .get_tracking_redraw(context);
        let mut selected_color = context.get(&WidgetAccentColor);
        if window_local.mouse_buttons_pressed > 0 {
            selected_color = selected_color.darken_by(ZeroToOne::new(0.8));
        }

        self.behavior.render(
            is_active,
            &colors,
            selected_color,
            window_local.checkbox_region,
            context,
        );

        if let Some(label) = &mut self.label {
            let label = label.mounted(context);
            context.for_other(&label).redraw();
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WidgetLayout {
        let window_local = self.per_window.entry(context).or_default();
        let indicator_layout = self.behavior.size(context);
        window_local.size = indicator_layout.size.ceil();
        window_local.checkbox_region.size = window_local.size.into_signed();

        let (mut full_size, baseline) = if let Some(label) = &mut self.label {
            let padding = context
                .get(&IntrinsicPadding)
                .into_px(context.gfx.scale())
                .ceil();
            let x_offset = window_local.checkbox_region.size.width + padding;
            let remaining_space = Size::new(
                available_space.width - x_offset.into_unsigned(),
                available_space.height,
            );
            let mounted = label.mounted(context);
            let label_layout = context.for_other(&mounted).layout(Size::new(
                remaining_space.width,
                ConstraintLimit::SizeToFit(remaining_space.height.max()),
            ));
            let indicator_baseline = indicator_layout
                .baseline
                .unwrap_or(indicator_layout.size.height);
            let height = indicator_layout.size.height.max(label_layout.size.height);
            let offset = match *label_layout.baseline {
                Some(baseline) if baseline < indicator_baseline => {
                    indicator_baseline.saturating_sub(baseline)
                }

                _ => UPx::ZERO,
            };

            window_local.label_region = Rect::new(
                Point::new(x_offset, offset.into_signed()),
                label_layout.size.into_signed(),
            );
            context.set_child_layout(&mounted, window_local.label_region);

            (
                Size::new(label_layout.size.width + x_offset.into_unsigned(), height),
                label_layout.baseline.map(|baseline| baseline + offset),
            )
        } else {
            (window_local.size.into_unsigned(), Baseline::NONE)
        };

        match (*baseline, *indicator_layout.baseline) {
            (Some(label_baseline), Some(indicator_baseline)) => {
                window_local.checkbox_region.origin.y =
                    (label_baseline.saturating_sub(indicator_baseline)).into_signed();
            }
            (Some(label_baseline), None) => {
                window_local.checkbox_region.origin.y =
                    (label_baseline.saturating_sub(window_local.size.height)).into_signed();
            }
            _ => {
                window_local.checkbox_region.origin.y =
                    (full_size.height.into_signed() - window_local.checkbox_region.size.height) / 2;
            }
        }

        full_size.height = window_local
            .label_region
            .extent()
            .y
            .max(window_local.checkbox_region.extent().y)
            .into_unsigned();

        WidgetLayout {
            size: full_size,
            baseline: baseline.max(indicator_layout.baseline),
        }
    }

    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> bool {
        let window_local = self.per_window.entry(context).or_default();
        window_local.hit_test(location)
    }

    fn mouse_down(
        &mut self,
        _location: Point<Px>,
        _device_id: crate::window::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        if context.enabled() {
            let window_local = self.per_window.entry(context).or_default();
            window_local.mouse_buttons_pressed += 1;

            context.set_needs_redraw();

            HANDLED
        } else {
            IGNORED
        }
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: crate::window::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_>,
    ) {
        let window_local = self.per_window.entry(context).or_default();
        let hovered = window_local.hit_test(location);
        if hovered != window_local.hovered {
            window_local.hovered = hovered;
            context.set_needs_redraw();
        }
    }

    fn mouse_up(
        &mut self,
        _location: Option<Point<Px>>,
        _device_id: crate::window::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut EventContext<'_>,
    ) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.mouse_buttons_pressed -= 1;
        let hovered = window_local.hovered;
        if window_local.mouse_buttons_pressed == 0 {
            self.clicked(context);
        }
        if self.focusable && hovered {
            context.focus();
        }
        context.set_needs_redraw();
    }

    fn accept_focus(&mut self, context: &mut EventContext<'_>) -> bool {
        self.focusable && context.enabled() && context.get(&AutoFocusableControls).is_all()
    }

    fn focus(&mut self, context: &mut EventContext<'_>) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.focused = true;
        context.set_needs_redraw();
    }

    fn blur(&mut self, context: &mut EventContext<'_>) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.focused = false;
        context.set_needs_redraw();
    }

    fn hover(
        &mut self,
        _location: Point<Px>,
        context: &mut EventContext<'_>,
    ) -> Option<CursorIcon> {
        if context.enabled() {
            let window_local = self.per_window.entry(context).or_default();
            window_local.hovered = true;
            context.set_needs_redraw();
            Some(CursorIcon::Pointer)
        } else {
            Some(CursorIcon::NotAllowed)
        }
    }

    fn unhover(&mut self, context: &mut EventContext<'_>) {
        let window_local = self.per_window.entry(context).or_default();
        window_local.hovered = false;
        context.set_needs_redraw();
    }

    fn activate(&mut self, context: &mut EventContext<'_>) {
        let window_local = self.per_window.entry(context).or_default();
        // If we have no buttons pressed, the event should fire on activate not
        // on deactivate.
        if window_local.mouse_buttons_pressed == 0 {
            self.clicked(context);
        }
        self.update_colors(context, true);
    }
}
