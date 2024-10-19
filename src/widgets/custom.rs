use std::fmt::Debug;

use figures::units::Px;
use figures::{Point, Size};
use kludgine::app::winit::event::{Ime, MouseButton, MouseScrollDelta, TouchPhase};
use kludgine::app::winit::window::CursorIcon;
use kludgine::Color;

use crate::context::{EventContext, GraphicsContext, LayoutContext, WidgetContext};
use crate::styles::VisualOrder;
use crate::value::{IntoValue, Value};
use crate::widget::{EventHandling, MakeWidget, WidgetRef, WrappedLayout, WrapperWidget, IGNORED};
use crate::widgets::Space;
use crate::window::{DeviceId, KeyEvent};
use crate::ConstraintLimit;

/// A callback-based custom widget.
///
/// This type can be used to create inline widgets without defining a new type
/// and implementing [`Widget`](crate::widget::Widget)/[`WrapperWidget`] for it.
#[must_use]
pub struct Custom {
    child: WidgetRef,
    redraw_foreground: Option<Box<dyn RedrawFunc>>,
    redraw_background: Option<Box<dyn RedrawFunc>>,
    mounted: Option<Box<dyn EventFunc>>,
    unmounted: Option<Box<dyn EventFunc>>,
    background: Option<Value<Color>>,
    unhover: Option<Box<dyn EventFunc>>,
    focus: Option<Box<dyn EventFunc>>,
    blur: Option<Box<dyn EventFunc>>,
    activate: Option<Box<dyn EventFunc>>,
    deactivate: Option<Box<dyn EventFunc>>,
    accept_focus: Option<Box<dyn EventFunc<bool>>>,
    adjust_child: Option<Box<dyn AdjustChildConstraintsFunc>>,
    position_child: Option<Box<dyn PositionChildFunc>>,
    hit_test: Option<Box<dyn OneParamEventFunc<Point<Px>, bool>>>,
    hover: Option<Box<dyn OneParamEventFunc<Point<Px>, Option<CursorIcon>>>>,
    mouse_down:
        Option<Box<dyn ThreeParamEventFunc<Point<Px>, DeviceId, MouseButton, EventHandling>>>,
    mouse_drag: Option<Box<dyn ThreeParamEventFunc<Point<Px>, DeviceId, MouseButton>>>,
    mouse_up: Option<Box<MouseUpFunc>>,
    ime: Option<Box<dyn OneParamEventFunc<Ime, EventHandling>>>,
    keyboard_input: Option<Box<dyn ThreeParamEventFunc<DeviceId, KeyEvent, bool, EventHandling>>>,
    mouse_wheel:
        Option<Box<dyn ThreeParamEventFunc<DeviceId, MouseScrollDelta, TouchPhase, EventHandling>>>,
    allow_blur: Option<Box<dyn EventFunc<bool>>>,
    advance_focus: Option<Box<dyn OneParamEventFunc<VisualOrder, EventHandling>>>,
}

impl Debug for Custom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Custom")
            .field("child", &self.child)
            .finish_non_exhaustive()
    }
}

impl Default for Custom {
    fn default() -> Self {
        Self::empty()
    }
}

impl Custom {
    /// Returns a custom widget that has no child.
    pub fn empty() -> Self {
        Self::new(Space::clear())
    }

    /// Returns a custom widget that contains `child`.
    pub fn new(child: impl MakeWidget) -> Self {
        Self {
            child: WidgetRef::new(child),
            redraw_background: None,
            redraw_foreground: None,
            background: None,
            mounted: None,
            unmounted: None,
            unhover: None,
            focus: None,
            blur: None,
            activate: None,
            deactivate: None,
            accept_focus: None,
            adjust_child: None,
            position_child: None,
            hit_test: None,
            hover: None,
            mouse_down: None,
            mouse_drag: None,
            mouse_up: None,
            ime: None,
            keyboard_input: None,
            mouse_wheel: None,
            allow_blur: None,
            advance_focus: None,
        }
    }

    /// Sets the background color of this widget to `color` and returns self.
    ///
    /// If the color is set to a non-transparent value, it will be filled before
    /// any of the redraw callbacks are invoked.
    ///
    /// This value coresponds to [`WrapperWidget::background_color`].
    pub fn background_color(mut self, color: impl IntoValue<Color>) -> Self {
        self.background = Some(color.into_value());
        self
    }

    /// Sets `redraw` as the callback to invoke when redrawing this control.
    ///
    /// If this control contains a child, its redraw function will be invoked
    /// after `redraw` is invoked. Use [`Self::on_redraw_after_child()`] to draw
    /// after the child widget.
    ///
    /// This callback corresponds to [`WrapperWidget::redraw_background`].
    pub fn on_redraw<Redraw>(mut self, redraw: Redraw) -> Self
    where
        Redraw: Send
            + 'static
            + for<'context, 'clip, 'gfx, 'pass> FnMut(
                &mut GraphicsContext<'context, 'clip, 'gfx, 'pass>,
            ),
    {
        self.redraw_background = Some(Box::new(redraw));
        self
    }

    /// Sets `redraw` as the callback to invoke when redrawing this control,
    /// after the child control has been redrawn.
    ///
    /// If this control contains a child, its redraw function will be invoked
    /// before `redraw` is invoked. Use [`Self::on_redraw()`] to draw before the
    /// child widget.
    ///
    /// `redraw` will be invoked regardless of whether a child is present.
    ///
    /// This callback corresponds to [`WrapperWidget::redraw_foreground`].
    pub fn on_redraw_after_child<Redraw>(mut self, redraw: Redraw) -> Self
    where
        Redraw: Send
            + 'static
            + for<'context, 'clip, 'gfx, 'pass> FnMut(
                &mut GraphicsContext<'context, 'clip, 'gfx, 'pass>,
            ),
    {
        self.redraw_foreground = Some(Box::new(redraw));
        self
    }

    /// Sets `mounted` to be invoked when this widget is mounted into a parent.
    ///
    /// This callback corresponds to [`WrapperWidget::mounted`].
    pub fn on_mounted<Mounted>(mut self, mounted: Mounted) -> Self
    where
        Mounted: Send + 'static + for<'context> FnMut(&mut EventContext<'context>),
    {
        self.mounted = Some(Box::new(mounted));
        self
    }

    /// Sets `unmounted` to be invoked when this widget is unmounted from its
    /// parent.
    ///
    /// This callback corresponds to [`WrapperWidget::unmounted`].
    pub fn on_unmounted<Mounted>(mut self, mounted: Mounted) -> Self
    where
        Mounted: Send + 'static + for<'context> FnMut(&mut EventContext<'context>),
    {
        self.unmounted = Some(Box::new(mounted));
        self
    }

    /// Invokes `unhovered` when the mouse cursor leaves the widget's boundary.
    ///
    /// This callback corresponds to [`WrapperWidget::unhover`].
    pub fn on_unhover<Unhover>(mut self, unhovered: Unhover) -> Self
    where
        Unhover: Send + 'static + for<'context> FnMut(&mut EventContext<'context>),
    {
        self.unhover = Some(Box::new(unhovered));
        self
    }

    /// Invokes `focus` when the widget receives input focus.
    ///
    /// This callback corresponds to [`WrapperWidget::focus`].
    pub fn on_focus<Focused>(mut self, focus: Focused) -> Self
    where
        Focused: Send + 'static + for<'context> FnMut(&mut EventContext<'context>),
    {
        self.focus = Some(Box::new(focus));
        self
    }

    /// Invokes `blur` when the widget loses input focus.
    ///
    /// This callback corresponds to [`WrapperWidget::blur`].
    pub fn on_blur<Blur>(mut self, blur: Blur) -> Self
    where
        Blur: Send + 'static + for<'context> FnMut(&mut EventContext<'context>),
    {
        self.blur = Some(Box::new(blur));
        self
    }

    /// Invokes `activated` when this widget becomes the active widget.
    ///
    /// This callback corresponds to [`WrapperWidget::activate`].
    pub fn on_activate<Activated>(mut self, activated: Activated) -> Self
    where
        Activated: Send + 'static + for<'context> FnMut(&mut EventContext<'context>),
    {
        self.activate = Some(Box::new(activated));
        self
    }

    /// Invokes `deactivated` when this widget no longer is the active widget.
    ///
    /// This callback corresponds to [`WrapperWidget::deactivate`].
    pub fn on_deactivate<Deactivated>(mut self, deactivated: Deactivated) -> Self
    where
        Deactivated: Send + 'static + for<'context> FnMut(&mut EventContext<'context>),
    {
        self.deactivate = Some(Box::new(deactivated));
        self
    }

    /// Invokes `accept` when this widget is set to receive input focus. If this
    /// function returns true, this widget will become the focused widget.
    ///
    /// This callback corresponds to [`WrapperWidget::accept_focus`].
    pub fn on_accept_focus<AcceptFocus>(mut self, accept: AcceptFocus) -> Self
    where
        AcceptFocus: Send + 'static + for<'context> FnMut(&mut EventContext<'context>) -> bool,
    {
        self.accept_focus = Some(Box::new(accept));
        self
    }

    /// Invokes `allow_blur` when this widget is about to lose focus. If
    /// `allow_blur` returns false, focus will be disallowed from leaving this
    /// widget.
    ///
    /// This callback corresponds to [`WrapperWidget::allow_blur`].
    pub fn on_allow_blur<AllowBlur>(mut self, allow_blur: AllowBlur) -> Self
    where
        AllowBlur: Send + 'static + for<'context> FnMut(&mut EventContext<'context>) -> bool,
    {
        self.allow_blur = Some(Box::new(allow_blur));
        self
    }

    /// Invokes `advance_focus` when this widget has focus and focus is
    /// requested to advance to another widget. Returning
    /// [`HANDLED`](crate::widget::HANDLED) will signal to Cushy that the focus
    /// has moved and the request should not be processed any further.
    ///
    /// This callback corresponds to [`WrapperWidget::advance_focus`].
    pub fn on_advance_focus<AdvanceFocus>(mut self, advance_focus: AdvanceFocus) -> Self
    where
        AdvanceFocus: Send
            + 'static
            + for<'context> FnMut(VisualOrder, &mut EventContext<'context>) -> EventHandling,
    {
        self.advance_focus = Some(Box::new(advance_focus));
        self
    }

    /// Invokes `adjust_child_constraints` before measuring the child widget.
    /// The returned constraints will be passed along to the child in its layout
    /// function.
    ///
    /// This callback corresponds to [`WrapperWidget::adjust_child_constraints`].
    pub fn on_adjust_child_constraints<AdjustChildConstraints>(
        mut self,
        adjust_child_constraints: AdjustChildConstraints,
    ) -> Self
    where
        AdjustChildConstraints: Send
            + 'static
            + for<'context, 'clip, 'gfx, 'pass> FnMut(
                Size<ConstraintLimit>,
                &mut LayoutContext<'context, 'clip, 'gfx, 'pass>,
            ) -> Size<ConstraintLimit>,
    {
        self.adjust_child = Some(Box::new(adjust_child_constraints));
        self
    }

    /// Invokes `position_child` to determine the position of a measured child.
    ///
    /// This callback corresponds to [`WrapperWidget::position_child`].
    pub fn on_position_child<PositionChild>(mut self, position_child: PositionChild) -> Self
    where
        PositionChild: Send
            + 'static
            + for<'context, 'clip, 'gfx, 'pass> FnMut(
                Size<Px>,
                Size<ConstraintLimit>,
                &mut LayoutContext<'context, 'clip, 'gfx, 'pass>,
            ) -> WrappedLayout,
    {
        self.position_child = Some(Box::new(position_child));
        self
    }

    /// Invokes `hit_test` when determining if a location should be considered
    /// interacting with this widget.
    ///
    /// This callback corresponds to [`WrapperWidget::hit_test`].
    pub fn on_hit_test<HitTest>(mut self, hit_test: HitTest) -> Self
    where
        HitTest:
            Send + 'static + for<'context> FnMut(Point<Px>, &mut EventContext<'context>) -> bool,
    {
        self.hit_test = Some(Box::new(hit_test));
        self
    }

    /// Invokes `hover` when a mouse cursor is above this widget.
    ///
    /// This callback corresponds to [`WrapperWidget::hover`].
    pub fn on_hover<Hover>(mut self, hover: Hover) -> Self
    where
        Hover: Send
            + 'static
            + for<'context> FnMut(Point<Px>, &mut EventContext<'context>) -> Option<CursorIcon>,
    {
        self.hover = Some(Box::new(hover));
        self
    }

    /// Invokes `mouse_down` when a mouse button is pushed on a location where
    /// [`Self::on_hit_test`] returned true.
    ///
    /// Returning [`HANDLED`](crate::widget::HANDLED) will set this widget as
    /// the handler for the [`DeviceId`] and [`MouseButton`]. Future mouse
    /// events for the same device and button will be sent to this widget's
    /// [`Self::on_mouse_drag`] and [`Self::on_mouse_up`] callbacks.
    ///
    /// This callback corresponds to [`WrapperWidget::mouse_down`].
    pub fn on_mouse_down<MouseDown>(mut self, mouse_down: MouseDown) -> Self
    where
        MouseDown: Send
            + 'static
            + for<'context> FnMut(
                Point<Px>,
                DeviceId,
                MouseButton,
                &mut EventContext<'context>,
            ) -> EventHandling,
    {
        self.mouse_down = Some(Box::new(mouse_down));
        self
    }

    /// Invokes `mouse_drag` when the mouse cursor moves while a tracked button
    /// is presed.
    ///
    /// This callback corresponds to [`WrapperWidget::mouse_drag`].
    pub fn on_mouse_drag<MouseDrag>(mut self, mouse_drag: MouseDrag) -> Self
    where
        MouseDrag: Send
            + 'static
            + for<'context> FnMut(Point<Px>, DeviceId, MouseButton, &mut EventContext<'context>),
    {
        self.mouse_drag = Some(Box::new(mouse_drag));
        self
    }

    /// Invokes `mouse_up` when a tracked mouse button is released.
    ///
    /// This callback corresponds to [`WrapperWidget::mouse_up`].
    pub fn on_mouse_up<MouseUp>(mut self, mouse_up: MouseUp) -> Self
    where
        MouseUp: Send
            + 'static
            + for<'context> FnMut(
                Option<Point<Px>>,
                DeviceId,
                MouseButton,
                &mut EventContext<'context>,
            ),
    {
        self.mouse_up = Some(Box::new(mouse_up));
        self
    }

    /// Invokes `ime` when an input manager event occurs.
    ///
    /// This callback corresponds to [`WrapperWidget::ime`].
    pub fn on_ime<OnIme>(mut self, ime: OnIme) -> Self
    where
        OnIme:
            Send + 'static + for<'context> FnMut(Ime, &mut EventContext<'context>) -> EventHandling,
    {
        self.ime = Some(Box::new(ime));
        self
    }

    /// Invokes `keyboard_input` when a keyboard event occurs.
    ///
    /// This callback corresponds to [`WrapperWidget::keyboard_input`].
    pub fn on_keyboard_input<KeyboardInput>(mut self, keyboard_input: KeyboardInput) -> Self
    where
        KeyboardInput: Send
            + 'static
            + for<'context> FnMut(
                DeviceId,
                KeyEvent,
                bool,
                &mut EventContext<'context>,
            ) -> EventHandling,
    {
        self.keyboard_input = Some(Box::new(keyboard_input));
        self
    }

    /// Invokes `mouse_wheel` when a mouse wheel event occurs.
    ///
    /// This callback corresponds to [`WrapperWidget::mouse_wheel`].
    pub fn mouse_wheel<MouseWheel>(mut self, mouse_wheel: MouseWheel) -> Self
    where
        MouseWheel: Send
            + 'static
            + for<'context> FnMut(
                DeviceId,
                MouseScrollDelta,
                TouchPhase,
                &mut EventContext<'context>,
            ) -> EventHandling,
    {
        self.mouse_wheel = Some(Box::new(mouse_wheel));
        self
    }
}

impl WrapperWidget for Custom {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn redraw_background(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        if let Some(redraw) = &mut self.redraw_background {
            redraw.invoke(context);
        }
    }

    fn redraw_foreground(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        if let Some(redraw) = &mut self.redraw_foreground {
            redraw.invoke(context);
        }
    }

    fn adjust_child_constraints(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<ConstraintLimit> {
        if let Some(adjust_child) = &mut self.adjust_child {
            adjust_child.invoke(available_space, context)
        } else {
            available_space
        }
    }

    fn position_child(
        &mut self,
        size: Size<Px>,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WrappedLayout {
        if let Some(position_child) = &mut self.position_child {
            position_child.invoke(size, available_space, context)
        } else {
            Size::new(
                available_space.width.fit_measured(size.width),
                available_space.height.fit_measured(size.height),
            )
            .into()
        }
    }

    fn background_color(&mut self, context: &WidgetContext<'_>) -> Option<Color> {
        self.background
            .as_ref()
            .map(|bg| bg.get_tracking_redraw(context))
    }

    fn mounted(&mut self, context: &mut EventContext<'_>) {
        if let Some(mounted) = &mut self.mounted {
            mounted.invoke(context);
        }
    }

    fn unmounted(&mut self, context: &mut EventContext<'_>) {
        if let Some(unmounted) = &mut self.unmounted {
            unmounted.invoke(context);
        } else {
            self.child_mut().unmount_in(context);
        }
    }

    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> bool {
        if let Some(hit_test) = &mut self.hit_test {
            hit_test.invoke(location, context)
        } else {
            false
        }
    }

    fn hover(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> Option<CursorIcon> {
        let hover = self.hover.as_mut()?;
        hover.invoke(location, context)
    }

    fn unhover(&mut self, context: &mut EventContext<'_>) {
        if let Some(unhover) = &mut self.unhover {
            unhover.invoke(context);
        }
    }

    fn accept_focus(&mut self, context: &mut EventContext<'_>) -> bool {
        if let Some(accept_focus) = &mut self.accept_focus {
            accept_focus.invoke(context)
        } else {
            false
        }
    }

    fn focus(&mut self, context: &mut EventContext<'_>) {
        if let Some(focus) = &mut self.focus {
            focus.invoke(context);
        }
    }

    fn blur(&mut self, context: &mut EventContext<'_>) {
        if let Some(blur) = &mut self.blur {
            blur.invoke(context);
        }
    }

    fn activate(&mut self, context: &mut EventContext<'_>) {
        if let Some(activate) = &mut self.activate {
            activate.invoke(context);
        }
    }

    fn deactivate(&mut self, context: &mut EventContext<'_>) {
        if let Some(deactivate) = &mut self.deactivate {
            deactivate.invoke(context);
        }
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        if let Some(mouse_down) = &mut self.mouse_down {
            mouse_down.invoke(location, device_id, button, context)
        } else {
            IGNORED
        }
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) {
        if let Some(mouse_drag) = &mut self.mouse_drag {
            mouse_drag.invoke(location, device_id, button, context);
        }
    }

    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) {
        if let Some(mouse_up) = &mut self.mouse_up {
            mouse_up.invoke(location, device_id, button, context);
        }
    }

    fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        if let Some(keyboard_input) = &mut self.keyboard_input {
            keyboard_input.invoke(device_id, input, is_synthetic, context)
        } else {
            IGNORED
        }
    }

    fn ime(&mut self, ime: Ime, context: &mut EventContext<'_>) -> EventHandling {
        if let Some(f) = &mut self.ime {
            f.invoke(ime, context)
        } else {
            IGNORED
        }
    }

    fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        if let Some(mouse_wheel) = &mut self.mouse_wheel {
            mouse_wheel.invoke(device_id, delta, phase, context)
        } else {
            IGNORED
        }
    }

    fn advance_focus(
        &mut self,
        direction: VisualOrder,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        if let Some(advance_focus) = &mut self.advance_focus {
            advance_focus.invoke(direction, context)
        } else {
            IGNORED
        }
    }

    fn allow_blur(&mut self, context: &mut EventContext<'_>) -> bool {
        if let Some(allow_blur) = &mut self.allow_blur {
            allow_blur.invoke(context)
        } else {
            true
        }
    }
}

trait RedrawFunc: Send {
    fn invoke(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>);
}

impl<Func> RedrawFunc for Func
where
    Func: Send
        + 'static
        + for<'context, 'clip, 'gfx, 'pass> FnMut(&mut GraphicsContext<'context, 'clip, 'gfx, 'pass>),
{
    fn invoke(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        self(context);
    }
}

trait AdjustChildConstraintsFunc: Send {
    fn invoke(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<ConstraintLimit>;
}

impl<Func> AdjustChildConstraintsFunc for Func
where
    Func: Send
        + 'static
        + for<'context, 'clip, 'gfx, 'pass> FnMut(
            Size<ConstraintLimit>,
            &mut LayoutContext<'context, 'clip, 'gfx, 'pass>,
        ) -> Size<ConstraintLimit>,
{
    fn invoke(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<ConstraintLimit> {
        self(available_space, context)
    }
}

trait PositionChildFunc: Send {
    fn invoke(
        &mut self,
        size: Size<Px>,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WrappedLayout;
}

impl<Func> PositionChildFunc for Func
where
    Func: Send
        + 'static
        + for<'context, 'clip, 'gfx, 'pass> FnMut(
            Size<Px>,
            Size<ConstraintLimit>,
            &mut LayoutContext<'context, 'clip, 'gfx, 'pass>,
        ) -> WrappedLayout,
{
    fn invoke(
        &mut self,
        size: Size<Px>,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WrappedLayout {
        self(size, available_space, context)
    }
}

trait EventFunc<R = ()>: Send {
    fn invoke(&mut self, context: &mut EventContext<'_>) -> R;
}

impl<R, Func> EventFunc<R> for Func
where
    Func: Send + 'static + for<'context> FnMut(&mut EventContext<'context>) -> R,
{
    fn invoke(&mut self, context: &mut EventContext<'_>) -> R {
        self(context)
    }
}

trait OneParamEventFunc<P, R = ()>: Send {
    fn invoke(&mut self, param: P, context: &mut EventContext<'_>) -> R;
}

impl<P, R, Func> OneParamEventFunc<P, R> for Func
where
    Func: Send + 'static + for<'context> FnMut(P, &mut EventContext<'context>) -> R,
{
    fn invoke(&mut self, location: P, context: &mut EventContext<'_>) -> R {
        self(location, context)
    }
}

trait ThreeParamEventFunc<P1, P2, P3, R = ()>: Send {
    fn invoke(
        &mut self,
        location: P1,
        device_id: P2,
        button: P3,
        context: &mut EventContext<'_>,
    ) -> R;
}

type MouseUpFunc = dyn ThreeParamEventFunc<Option<Point<Px>>, DeviceId, MouseButton>;

impl<P1, P2, P3, R, Func> ThreeParamEventFunc<P1, P2, P3, R> for Func
where
    Func: Send + 'static + for<'context> FnMut(P1, P2, P3, &mut EventContext<'context>) -> R,
{
    fn invoke(
        &mut self,
        location: P1,
        device_id: P2,
        button: P3,
        context: &mut EventContext<'_>,
    ) -> R {
        self(location, device_id, button, context)
    }
}
