//! Types for creating reusable widgets (aka components or views).

use std::any::Any;
use std::clone::Clone;
use std::fmt::{self, Debug};
use std::ops::{ControlFlow, Deref, DerefMut};
use std::sync::atomic::{self, AtomicU64};
use std::sync::Arc;
use std::{slice, vec};

use alot::LotId;
use figures::units::{Px, UPx};
use figures::{IntoSigned, IntoUnsigned, Point, Rect, Size, Zero};
use intentional::Assert;
use kludgine::app::winit::event::{Ime, MouseButton, MouseScrollDelta, TouchPhase};
use kludgine::app::winit::keyboard::ModifiersState;
use kludgine::app::winit::window::CursorIcon;
use kludgine::Color;
use parking_lot::{Mutex, MutexGuard};

use crate::app::Run;
use crate::context::sealed::Trackable as _;
use crate::context::{
    AsEventContext, EventContext, GraphicsContext, LayoutContext, ManageWidget, WidgetContext,
};
use crate::styles::components::{
    FontFamily, FontStyle, FontWeight, Heading1FontFamily, Heading1Style, Heading1Weight,
    Heading2FontFamily, Heading2Style, Heading2Weight, Heading3FontFamily, Heading3Style,
    Heading3Weight, Heading4FontFamily, Heading4Style, Heading4Weight, Heading5FontFamily,
    Heading5Style, Heading5Weight, Heading6FontFamily, Heading6Style, Heading6Weight, LineHeight,
    LineHeight1, LineHeight2, LineHeight3, LineHeight4, LineHeight5, LineHeight6, LineHeight7,
    LineHeight8, TextSize, TextSize1, TextSize2, TextSize3, TextSize4, TextSize5, TextSize6,
    TextSize7, TextSize8,
};
use crate::styles::{
    ComponentDefinition, ContainerLevel, Dimension, DimensionRange, Edges, IntoComponentValue,
    IntoDynamicComponentValue, Styles, ThemePair, VisualOrder,
};
use crate::tree::{Tree, WeakTree};
use crate::value::{Dynamic, Generation, IntoDynamic, IntoValue, Validation, Value};
use crate::widgets::checkbox::{Checkable, CheckboxState};
use crate::widgets::layers::{OverlayLayer, Tooltipped};
use crate::widgets::list::List;
use crate::widgets::shortcuts::{ShortcutKey, Shortcuts};
use crate::widgets::{
    Align, Button, Checkbox, Collapse, Container, Disclose, Expand, Layers, Resize, Scroll, Space,
    Stack, Style, Themed, ThemedMode, Validated, Wrap,
};
use crate::window::sealed::WindowCommand;
use crate::window::{
    DeviceId, KeyEvent, MakeWindow, Rgb8, RunningWindow, StandaloneWindowBuilder, ThemeMode,
    VirtualRecorderBuilder, Window, WindowBehavior, WindowHandle, WindowLocal,
};
use crate::ConstraintLimit;

/// A type that makes up a graphical user interface.
///
/// This type can go by many names in other UI frameworks: View, Component,
/// Control.
///
/// # Widgets are hierarchical
///
/// Cushy's widgets are organized in a hierarchical structure: widgets can
/// contain other widgets. A window in Cushy contains a single root widget,
/// which may contain one or more additional widgets.
///
/// # How Widgets are created
///
/// Cushy offers several approaches to creating widgets. The primary trait that
/// is used to instantiate a widget is [`MakeWidget`]. This trait is
/// automatically implemented for all types that implement [`Widget`].
///
/// [`MakeWidget::make_widget`] is responsible for returning a
/// [`WidgetInstance`]. This is a wrapper for a type that implements [`Widget`]
/// that can be used without knowing the original type of the [`Widget`].
///
/// While all [`MakeWidget`] is automatically implemented for all [`Widget`]
/// types, it can also be implemented by types that do not implement [`Widget`].
/// This is a useful strategy when designing reusable widgets that are able to
/// be completely represented by composing existing widgets. The
/// [`ProgressBar`](crate::widgets::ProgressBar) type uses this strategy, as it
/// uses either a [`Spinner`](crate::widgets::progress::Spinner) or a
/// [`Slider`](crate::widgets::Slider) to show its progress.
///
/// One last convenience trait is provided to help create widgets that contain
/// exactly one child: [`WrapperWidget`]. [`WrapperWidget`] exposes most of the
/// same functions, but provides purpose-built functions for tweaking child's
/// layout and rendering behavior to minimize the amount of redundant code
/// between these types of widgets.
///
/// # Identifying Widgets
///
/// Once a widget has been instantiated as a [`WidgetInstance`], it will be
/// assigned a unique [`WidgetId`]. Sometimes, it may be helpful to pre-create a
/// [`WidgetId`] before the widget has been created. For these situations,
/// [`WidgetTag`] allows creating a tag that can be passed to
/// [`MakeWidgetWithTag::make_with_tag`] to set the returned
/// [`WidgetInstance`]'s id.
///
/// # How to "talk" to another widget
///
/// Once a widget has been wrapped inside of a [`WidgetInstance`], it is no
/// longer possible to invoke [`Widget`]/s functions directly. Instead, a
/// context must be created for that widget. In each of the [`Widget`]
/// functions, a context is provided that represents the current widget. Each
/// context type has a `for_other()` function that accepts any widget type: a
/// [`WidgetId`], a [`WidgetInstance`], a [`MountedWidget`], or a [`WidgetRef`].
/// The returned context will represent the associate widget, allowing access to
/// the exposed APIs through the context.
///
/// While [`WidgetInstance::lock`] can be used to gain access to the underlying
/// [`Widget`] type, this behavior should only be reserved for limited
/// situations. It should be preferred to pass data between widgets using
/// [`Dynamic`]s or style components if possible. This ensures that your code
/// can work with as many other widgets as possible, instead of restricting
/// features to a specific set of types.
///
/// # How layout and rendering works
///
/// When a window is rendered, the root widget has its
/// [`layout()`](Self::layout) function called with both constraints specifying
/// [`ConstraintLimit::SizeToFit`] with the window's inner size. The root widget
/// measures its content to try to fit within the specified constraints, and
/// returns its calculated size. If a widget has children, it can invoke
/// [`LayoutContext::layout()`] on a context for each of its children to
/// determine their required sizes.
///
/// Next, the window sets the root's layout. When a widget contains another
/// widget, it must call [`LayoutContext::set_child_layout`] for the child to be
/// able to be rendered. This tells Cushy the location to draw the widget. While
/// it is possible to provide any rectangle, Cushy clips all widgets and their
/// children so that they cannot draw outside of their assigned bounds.
///
/// Once the layout has been determined, the window will invoke the root
/// widget's [`redraw()`](Self::redraw) function. If a widget contains one or
/// more children, it needs to invoke [`GraphicsContext::redraw()`] on a context
/// for each of its children during its own render function. This allows full
/// control over the order of drawing calls, allowing widgets to draw behind,
/// in-between, or in front of their children.
///
/// The last responsibility the window has each frame is size adjustment. The
/// window will potentially adjust its size automatically based on the root
/// widget's [`root_behavior()`](Self::root_behavior).
///
/// # Controlling Invalidation and Redrawing
///
/// Cushy only redraws window contents when requested by the operating system or
/// a tracked [`Dynamic`] is updated. Similarly, Cushy caches the known layout
/// sizes and locations for widgets unless they are *invalidated*. Invalidation
/// is done automatically when the window size changes or a tracked [`Dynamic`]
/// is updated.
///
/// These systems require Cushy to track which [`Dynamic`] values a widget
/// depends on for redrawing and invalidation. During a widget's redraw and
/// layout functions, it needs to ensure that all depended upon [`Dynamic`]s are
/// tracked using one of the various
/// `*_tracking_redraw()`/`*_tracking_invalidate()` functions. For example,
/// [`Source::get_tracking_redraw()`](crate::value::Source::get_tracking_redraw)
/// and
/// [`Source::get_tracking_invalidate()`](crate::value::Source::get_tracking_invalidate).
///
/// # Hover State: Hit Testing
///
/// Before any cursor-related events are sent to a widget, the cursor's position
/// is tested with [`Widget::hit_test`]. When a widget returns true for a
/// position, it is eligible to receive events such as mouse buttons.
///
/// When a widget returns false, it will not receive any cursor related events
/// with one exception: hover events. Hover events will fire for widgets whose
/// children are currently being hovered, regardless of whether
/// [`Widget::hit_test`] returned true.
///
/// The provided [`Widget::hit_test`] implementation returns false.
///
/// As the cursor moves across the window, the window will look at the render
/// information to see what widgets are positioned under the cursor and the
/// order in which they were drawn. Beginning at the topmost widget,
/// [`Widget::hit_test`] is called on each widget.
///
/// The currently hovered widget state is tracked for events that target widgets
/// beneath the current cursor.
///
/// # Mouse Button Events
///
/// When a window receives an event for a mouse button being pressed, it calls
/// the hovered widget's [`mouse_down()`](Self::mouse_down) function. If the
/// function returns [`HANDLED`]/[`ControlFlow::Break`], the widget becomes the
/// *tracking* widget for that mouse button.
///
/// If the widget returns [`IGNORED`]/[`ControlFlow::Continue`], the window will
/// call the parent's `mouse_down()` function. This repeats until the root
/// widget is reached or a widget returns `HANDLED`.
///
/// Once a tracking widget is found, any cursor-related movements will cause
/// [`Widget::mouse_drag()`] to be called. Upon the mouse button being released,
/// the tracking widget's [`mouse_up()`](Self::mouse_up) function will be
/// called.
///
/// # User Input Focus
///
/// A window can have a widget be *focused* for user input. For example, a text
/// [`Input`](crate::widgets::Input) only responds to keyboard input once user
/// input focus has been directed at the widget. This state is generally
/// represented by drawing the theme's highlight color around the border of the
/// widget. [`GraphicsContext::draw_focus_ring`] can be used to draw the
/// standard focus ring for rectangular-shaped widgets.
///
/// The most direct way to give a widget focus is to call
/// [`WidgetContext::focus`]. However, not all widgets can accept focus. If a
/// widget returns true from its [`accept_focus()`](Self::accept_focus)
/// function, focus will be given to it and its [`focus()`](Self::focus)
/// function will be invoked.
///
/// If a widget returns false from its `accept_focus()` function, the window
/// will perform these steps:
///
/// 1. If the widget has any children, sort its children visually and attempt to
///    focus each one until a widget accepts focus. If any of these children
///    have children, those children should also be checked.
/// 2. The widget asks its parent to find the next focus after itself. The
///    parent finds the current widget in that list and attempts to focus each
///    widget after the current widget in the visual order.
/// 3. This repeats until the root widget is reached, at which point focus is
///    attempted using this algorithm until either a focused widget is found or
///    the original widget is reached again. If no widget can be found in a full
///    cycle of the widget tree, focus will be cleared.
///
/// When a window first opens, it call [`focus()`][WidgetContext::focus] on the
/// root widget's context.
///
/// ## Losing Focus
///
/// A Widget can deny the ability for focus to be taken away from it by
/// returning `false` from [`Widget::allow_blur()`]. In general, widgets should
/// not do this. However, some user interfaces are designed to always keep focus
/// on a single widget, and this feature enables that functionality.
///
/// When a widget currently has focused and loses it, its [`blur()`](Self::blur)
/// function will be invoked.
///
/// # Styling
///
/// Cushy allows widgets to receive styling information through the widget
/// hierarchy using [`Styles`]. Cushy calculates the effectives styles for each
/// widget by inheriting all inheritable styles from its parent.
///
/// The [`Style`] widget allows assigining [`Styles`] to all of its children
/// widget. It works by calling [`WidgetContext::attach_styles`], and Cushy
/// takes care of the rest.
///
/// Styling in Cushy aims to be simple, easy-to-understand, and extensible.
///
/// # Color Themes
///
/// Cushy aims to make it easy for developers to customize the appearance of its
/// applications. The way color themes work in Cushy begins with the
/// [`ColorScheme`](crate::styles::ColorScheme). A color scheme is a set of
/// [`ColorSource`](crate::styles::ColorSource) that are used to generate a
/// variety of shades of colors for various roles color plays in a user
/// interface. In a way, coloring Cushy apps is a bit like paint-by-number,
/// where the number is the name of the color role.
///
/// A `ColorScheme` can be used to create a [`ThemePair`], which is theme
/// definition that a theme for light and dark mode.
///
/// In [the repository][repo], the `theme` example is a good way to explore how
/// the color system works in Cushy.
///
/// [repo]: https://github.com/khonsulabs/cushy
pub trait Widget: Send + Debug + 'static {
    /// Redraw the contents of this widget.
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>);

    /// Writes a summary of this widget into `fmt`.
    ///
    /// The default implementation calls [`Debug::fmt`]. This function allows
    /// widget authors to print only publicly relevant information that will
    /// appear when debug formatting a [`WidgetInstance`].
    fn summarize(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }

    /// Returns true if this widget handles all built-in style components that
    /// apply.
    ///
    /// These components are:
    ///
    /// - [`Opacity`](crate::styles::components::Opacity)
    /// - [`WidgetBackground`](crate::styles::components::WidgetBackground)
    /// - [`FontFamily`]
    /// - [`TextSize`]
    /// - [`LineHeight`]
    /// - [`FontStyle`]
    /// - [`FontWeight`]
    fn full_control_redraw(&self) -> bool {
        false
    }

    /// Layout this widget and returns the ideal size based on its contents and
    /// the `available_space`.
    #[allow(unused_variables)]
    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        available_space.map(ConstraintLimit::min)
    }

    /// The widget has been mounted into a parent widget.
    ///
    /// Widgets that contain [`MountedWidget`] references should call
    /// [`MountedWidget::remount_if_needed`] in this function.
    #[allow(unused_variables)]
    fn mounted(&mut self, context: &mut EventContext<'_>) {}

    /// The widget has been removed from its parent widget.
    #[allow(unused_variables)]
    fn unmounted(&mut self, context: &mut EventContext<'_>) {}

    /// Returns true if this widget should respond to mouse input at `location`.
    #[allow(unused_variables)]
    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> bool {
        false
    }

    /// The widget is currently has a cursor hovering it at `location`.
    #[allow(unused_variables)]
    fn hover(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> Option<CursorIcon> {
        None
    }

    /// The widget is no longer being hovered.
    #[allow(unused_variables)]
    fn unhover(&mut self, context: &mut EventContext<'_>) {}

    /// This widget has been targeted to be focused. If this function returns
    /// true, the widget will be focused. If false, Cushy will continue
    /// searching for another focus target.
    #[allow(unused_variables)]
    fn accept_focus(&mut self, context: &mut EventContext<'_>) -> bool {
        false
    }

    /// The widget has received focus for user input.
    #[allow(unused_variables)]
    fn focus(&mut self, context: &mut EventContext<'_>) {}

    /// The widget should switch to the next focusable area within this widget,
    /// honoring `direction` in a consistent manner. Returning `HANDLED` will
    /// cause the search for the next focus widget stop.
    #[allow(unused_variables)]
    fn advance_focus(
        &mut self,
        direction: VisualOrder,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        IGNORED
    }

    /// The widget is about to lose focus. Returning true allows the focus to
    /// switch away from this widget.
    #[allow(unused_variables)]
    fn allow_blur(&mut self, context: &mut EventContext<'_>) -> bool {
        true
    }

    /// The widget is no longer focused for user input.
    #[allow(unused_variables)]
    fn blur(&mut self, context: &mut EventContext<'_>) {}

    /// The widget has become the active widget.
    #[allow(unused_variables)]
    fn activate(&mut self, context: &mut EventContext<'_>) {}

    /// The widget is no longer active.
    #[allow(unused_variables)]
    fn deactivate(&mut self, context: &mut EventContext<'_>) {}

    /// A mouse button event has occurred at `location`. Returns whether the
    /// event has been handled or not.
    ///
    /// If an event is handled, the widget will receive callbacks for
    /// [`mouse_drag`](Self::mouse_drag) and [`mouse_up`](Self::mouse_up).
    #[allow(unused_variables)]
    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        IGNORED
    }

    /// A mouse button is being held down as the cursor is moved across the
    /// widget.
    #[allow(unused_variables)]
    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) {
    }

    /// A mouse button is no longer being pressed.
    #[allow(unused_variables)]
    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) {
    }

    /// A keyboard event has been sent to this widget. Returns whether the event
    /// has been handled or not.
    #[allow(unused_variables)]
    fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        IGNORED
    }

    /// An input manager event has been sent to this widget. Returns whether the
    /// event has been handled or not.
    #[allow(unused_variables)]
    fn ime(&mut self, ime: Ime, context: &mut EventContext<'_>) -> EventHandling {
        IGNORED
    }

    /// A mouse wheel event has been sent to this widget. Returns whether the
    /// event has been handled or not.
    #[allow(unused_variables)]
    fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        IGNORED
    }

    /// Returns a reference to a single child widget if this widget is a widget
    /// that primarily wraps a single other widget to customize its behavior.
    #[must_use]
    #[allow(unused_variables)]
    fn root_behavior(
        &mut self,
        context: &mut EventContext<'_>,
    ) -> Option<(RootBehavior, WidgetInstance)> {
        None
    }
}

// ANCHOR: run
impl<T> Run for T
where
    T: MakeWidget,
{
    fn run(self) -> crate::Result {
        Window::for_widget(self).run()
    }
}
// ANCHOR_END: run

/// A behavior that should be applied to a root widget.
#[derive(Debug, Clone, Copy)]
pub enum RootBehavior {
    /// This widget does not care about root behaviors, and its child should be
    /// allowed to specify a behavior.
    PassThrough,
    /// This widget will try to expand to fill the window.
    Expand,
    /// This widget will measure its contents to fit its child, but Cushy should
    /// still stretch this widget to fill the window.
    Align,
    /// This widget adjusts its child layout with padding.
    Pad(Edges<Dimension>),
    /// This widget changes the size of its child.
    Resize(Size<DimensionRange>),
}

/// The layout of a [wrapped](WrapperWidget) child widget.
#[derive(Clone, Copy, Debug)]
pub struct WrappedLayout {
    /// The region the child widget occupies within its parent.
    pub child: Rect<Px>,
    /// The size the wrapper widget should report as.
    pub size: Size<UPx>,
}

impl From<Size<Px>> for WrappedLayout {
    fn from(size: Size<Px>) -> Self {
        WrappedLayout {
            child: size.into(),
            size: size.into_unsigned(),
        }
    }
}

impl From<Size<UPx>> for WrappedLayout {
    fn from(size: Size<UPx>) -> Self {
        WrappedLayout {
            child: size.into_signed().into(),
            size,
        }
    }
}

/// A [`Widget`] that contains a single child.
pub trait WrapperWidget: Debug + Send + 'static {
    /// Returns the child widget.
    fn child_mut(&mut self) -> &mut WidgetRef;

    /// Writes a summary of this widget into `fmt`.
    ///
    /// The default implementation calls [`Debug::fmt`]. This function allows
    /// widget authors to print only publicly relevant information that will
    /// appear when debug formatting a [`WidgetInstance`].
    fn summarize(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self, f)
    }

    /// Returns the behavior this widget should apply when positioned at the
    /// root of the window.
    ///
    /// The provided implementation for `WrapperWidget` returns
    /// [`RootBehavior::PassThrough`]. This is different from the provided
    /// implementation for [`Widget`].
    #[allow(unused_variables)]
    fn root_behavior(&mut self, context: &mut EventContext<'_>) -> Option<RootBehavior> {
        Some(RootBehavior::PassThrough)
    }

    /// Draws the background of the widget.
    ///
    /// This is invoked before the wrapped widget is drawn.
    #[allow(unused_variables)]
    fn redraw_background(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {}

    /// Draws the foreground of the widget.
    ///
    /// This is invoked after the wrapped widget is drawn.
    #[allow(unused_variables)]
    fn redraw_foreground(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {}

    /// Returns the rectangle that the child widget should occupy given
    /// `available_space`.
    #[allow(unused_variables)]
    fn layout_child(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WrappedLayout {
        let adjusted_space = self.adjust_child_constraints(available_space, context);
        let child = self.child_mut().mounted(&mut context.as_event_context());
        let size = context
            .for_other(&child)
            .layout(adjusted_space)
            .into_signed();

        self.position_child(size, available_space, context)
    }

    /// Returns the adjusted contraints to use when laying out the child.
    #[allow(unused_variables)]
    #[must_use]
    fn adjust_child_constraints(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<ConstraintLimit> {
        available_space
    }

    /// Returns the layout after positioning the child that occupies `size`.
    #[allow(unused_variables)]
    #[must_use]
    fn position_child(
        &mut self,
        size: Size<Px>,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> WrappedLayout {
        Size::new(
            available_space
                .width
                .fit_measured(size.width, context.gfx.scale()),
            available_space
                .height
                .fit_measured(size.height, context.gfx.scale()),
        )
        .into()
    }

    /// Returns the background color to render behind the wrapped widget.
    #[allow(unused_variables)]
    #[must_use]
    fn background_color(&mut self, context: &WidgetContext<'_>) -> Option<Color> {
        // WidgetBackground is already filled, so we don't need to do anything
        // else by default.
        None
    }

    /// The widget has been mounted into a parent widget.
    #[allow(unused_variables)]
    fn mounted(&mut self, context: &mut EventContext<'_>) {}

    /// The widget has been removed from its parent widget.
    #[allow(unused_variables)]
    fn unmounted(&mut self, context: &mut EventContext<'_>) {
        self.child_mut().unmount_in(context);
    }

    /// Returns true if this widget should respond to mouse input at `location`.
    #[allow(unused_variables)]
    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> bool {
        false
    }

    /// The widget is currently has a cursor hovering it at `location`.
    #[allow(unused_variables)]
    fn hover(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> Option<CursorIcon> {
        None
    }

    /// The widget is no longer being hovered.
    #[allow(unused_variables)]
    fn unhover(&mut self, context: &mut EventContext<'_>) {}

    /// This widget has been targeted to be focused. If this function returns
    /// true, the widget will be focused. If false, Cushy will continue
    /// searching for another focus target.
    #[allow(unused_variables)]
    fn accept_focus(&mut self, context: &mut EventContext<'_>) -> bool {
        false
    }

    /// The widget should switch to the next focusable area within this widget,
    /// honoring `direction` in a consistent manner. Returning `HANDLED` will
    /// cause the search for the next focus widget stop.
    #[allow(unused_variables)]
    fn advance_focus(
        &mut self,
        direction: VisualOrder,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        IGNORED
    }

    /// The widget has received focus for user input.
    #[allow(unused_variables)]
    fn focus(&mut self, context: &mut EventContext<'_>) {}

    /// The widget is about to lose focus. Returning true allows the focus to
    /// switch away from this widget.
    #[allow(unused_variables)]
    fn allow_blur(&mut self, context: &mut EventContext<'_>) -> bool {
        true
    }

    /// The widget is no longer focused for user input.
    #[allow(unused_variables)]
    fn blur(&mut self, context: &mut EventContext<'_>) {}

    /// The widget has become the active widget.
    #[allow(unused_variables)]
    fn activate(&mut self, context: &mut EventContext<'_>) {
        let child = self.child_mut().mounted(context);
        context.for_other(&child).activate();
    }

    /// The widget is no longer active.
    #[allow(unused_variables)]
    fn deactivate(&mut self, context: &mut EventContext<'_>) {}

    /// A mouse button event has occurred at `location`. Returns whether the
    /// event has been handled or not.
    ///
    /// If an event is handled, the widget will receive callbacks for
    /// [`mouse_drag`](Self::mouse_drag) and [`mouse_up`](Self::mouse_up).
    #[allow(unused_variables)]
    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        IGNORED
    }

    /// A mouse button is being held down as the cursor is moved across the
    /// widget.
    #[allow(unused_variables)]
    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) {
    }

    /// A mouse button is no longer being pressed.
    #[allow(unused_variables)]
    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) {
    }

    /// A keyboard event has been sent to this widget. Returns whether the event
    /// has been handled or not.
    #[allow(unused_variables)]
    fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        IGNORED
    }

    /// An input manager event has been sent to this widget. Returns whether the
    /// event has been handled or not.
    #[allow(unused_variables)]
    fn ime(&mut self, ime: Ime, context: &mut EventContext<'_>) -> EventHandling {
        IGNORED
    }

    /// A mouse wheel event has been sent to this widget. Returns whether the
    /// event has been handled or not.
    #[allow(unused_variables)]
    fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        IGNORED
    }
}

impl<T> Widget for T
where
    T: WrapperWidget,
{
    fn root_behavior(
        &mut self,
        context: &mut EventContext<'_>,
    ) -> Option<(RootBehavior, WidgetInstance)> {
        T::root_behavior(self, context)
            .map(|behavior| (behavior, T::child_mut(self).widget().clone()))
    }

    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        let background_color = self.background_color(context);
        if let Some(color) = background_color {
            context.fill(color);
        }

        self.redraw_background(context);

        let child = self.child_mut().mounted(&mut context.as_event_context());
        context.for_other(&child).redraw();

        self.redraw_foreground(context);
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let layout = self.layout_child(available_space, context);
        let child = self.child_mut().mounted(&mut context.as_event_context());
        context.set_child_layout(&child, layout.child);
        layout.size
    }

    fn mounted(&mut self, context: &mut EventContext<'_>) {
        T::mounted(self, context);
    }

    fn unmounted(&mut self, context: &mut EventContext<'_>) {
        T::unmounted(self, context);
    }

    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> bool {
        T::hit_test(self, location, context)
    }

    fn hover(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> Option<CursorIcon> {
        T::hover(self, location, context)
    }

    fn unhover(&mut self, context: &mut EventContext<'_>) {
        T::unhover(self, context);
    }

    fn accept_focus(&mut self, context: &mut EventContext<'_>) -> bool {
        T::accept_focus(self, context)
    }

    fn focus(&mut self, context: &mut EventContext<'_>) {
        T::focus(self, context);
    }

    fn blur(&mut self, context: &mut EventContext<'_>) {
        T::blur(self, context);
    }

    fn activate(&mut self, context: &mut EventContext<'_>) {
        T::activate(self, context);
    }

    fn deactivate(&mut self, context: &mut EventContext<'_>) {
        T::deactivate(self, context);
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        T::mouse_down(self, location, device_id, button, context)
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) {
        T::mouse_drag(self, location, device_id, button, context);
    }

    fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
        context: &mut EventContext<'_>,
    ) {
        T::mouse_up(self, location, device_id, button, context);
    }

    fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        T::keyboard_input(self, device_id, input, is_synthetic, context)
    }

    fn ime(&mut self, ime: Ime, context: &mut EventContext<'_>) -> EventHandling {
        T::ime(self, ime, context)
    }

    fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        T::mouse_wheel(self, device_id, delta, phase, context)
    }

    fn advance_focus(
        &mut self,
        direction: VisualOrder,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        T::advance_focus(self, direction, context)
    }

    fn allow_blur(&mut self, context: &mut EventContext<'_>) -> bool {
        T::allow_blur(self, context)
    }

    fn summarize(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        T::summarize(self, fmt)
    }
}

/// A type that can create a [`WidgetInstance`].
pub trait MakeWidget: Sized {
    /// Returns a new widget.
    fn make_widget(self) -> WidgetInstance;

    /// Returns a new window containing `self` as the root widget.
    fn into_window(self) -> Window {
        self.make_window()
    }

    /// Returns a builder for a standalone window.
    ///
    /// A standalone window can be either a
    /// [`VirtualWindow`](crate::window::VirtualWindow) or a
    /// [`CushyWindow`](crate::window::CushyWindow).
    fn build_standalone_window(self) -> StandaloneWindowBuilder {
        StandaloneWindowBuilder::new(self)
    }

    /// Returns a builder for a [`VirtualRecorder`](crate::window::VirtualRecorder)
    fn build_recorder(self) -> VirtualRecorderBuilder<Rgb8> {
        VirtualRecorderBuilder::new(self)
    }

    /// Associates `styles` with this widget.
    ///
    /// This is equivalent to `Style::new(styles, self)`.
    fn with_styles(self, styles: impl IntoValue<Styles>) -> Style
    where
        Self: Sized,
    {
        Style::new(styles, self)
    }

    /// Associates a style component with `self`.
    fn with<C: ComponentDefinition>(
        self,
        name: &C,
        component: impl IntoValue<C::ComponentType>,
    ) -> Style
    where
        Value<C::ComponentType>: IntoComponentValue,
    {
        Style::new(Styles::new().with(name, component), self)
    }

    /// Associates a style component with `self`, resolving its value using
    /// `dynamic` at runtime.
    fn with_dynamic<C: ComponentDefinition>(
        self,
        name: &C,
        dynamic: impl IntoDynamicComponentValue,
    ) -> Style
    where
        C::ComponentType: IntoComponentValue,
    {
        Style::new(Styles::new().with_dynamic(name, dynamic), self)
    }

    /// Invokes `callback` when `key` is pressed while `modifiers` are pressed.
    ///
    /// This shortcut will only be invoked if focus is within `self` or a child
    /// of `self`, or if the returned widget becomes the root widget of a
    /// window.
    #[must_use]
    fn with_shortcut<F>(
        self,
        key: impl Into<ShortcutKey>,
        modifiers: ModifiersState,
        callback: F,
    ) -> Shortcuts
    where
        F: FnMut(KeyEvent) -> EventHandling + Send + 'static,
    {
        Shortcuts::new(self).with_shortcut(key, modifiers, callback)
    }

    /// Invokes `callback` when `key` is pressed while `modifiers` are pressed.
    /// If the shortcut is held, the callback will be invoked on repeat events.
    ///
    /// This shortcut will only be invoked if focus is within `self` or a child
    /// of `self`, or if the returned widget becomes the root widget of a
    /// window.
    #[must_use]
    fn with_repeating_shortcut<F>(
        self,
        key: impl Into<ShortcutKey>,
        modifiers: ModifiersState,
        callback: F,
    ) -> Shortcuts
    where
        F: FnMut(KeyEvent) -> EventHandling + Send + 'static,
    {
        Shortcuts::new(self).with_repeating_shortcut(key, modifiers, callback)
    }

    /// Styles `self` with the largest of 6 heading styles.
    fn h1(self) -> Style {
        self.xxxx_large()
            .with_dynamic(&FontStyle, Heading1Style)
            .with_dynamic(&FontFamily, Heading1FontFamily)
            .with_dynamic(&FontWeight, Heading1Weight)
    }

    /// Styles `self` with the second largest of 6 heading styles.
    fn h2(self) -> Style {
        self.xxx_large()
            .with_dynamic(&FontStyle, Heading2Style)
            .with_dynamic(&FontFamily, Heading2FontFamily)
            .with_dynamic(&FontWeight, Heading2Weight)
    }

    /// Styles `self` with the third largest of 6 heading styles.
    fn h3(self) -> Style {
        self.xx_large()
            .with_dynamic(&FontStyle, Heading3Style)
            .with_dynamic(&FontFamily, Heading3FontFamily)
            .with_dynamic(&FontWeight, Heading3Weight)
    }

    /// Styles `self` with the third smallest of 6 heading styles.
    fn h4(self) -> Style {
        self.x_large()
            .with_dynamic(&FontStyle, Heading4Style)
            .with_dynamic(&FontFamily, Heading4FontFamily)
            .with_dynamic(&FontWeight, Heading4Weight)
    }

    /// Styles `self` with the second smallest of 6 heading styles.
    fn h5(self) -> Style {
        self.large()
            .with_dynamic(&FontStyle, Heading5Style)
            .with_dynamic(&FontFamily, Heading5FontFamily)
            .with_dynamic(&FontWeight, Heading5Weight)
    }

    /// Styles `self` with the smallest of 6 heading styles.
    fn h6(self) -> Style {
        self.default_size()
            .with_dynamic(&FontStyle, Heading6Style)
            .with_dynamic(&FontFamily, Heading6FontFamily)
            .with_dynamic(&FontWeight, Heading6Weight)
    }

    /// Styles `self` with the largest text size.
    #[must_use]
    fn xxxx_large(self) -> Style {
        self.with_dynamic(&TextSize, TextSize8)
            .with_dynamic(&LineHeight, LineHeight8)
    }

    /// Styles `self` with the second largest text size.
    #[must_use]
    fn xxx_large(self) -> Style {
        self.with_dynamic(&TextSize, TextSize7)
            .with_dynamic(&LineHeight, LineHeight7)
    }

    /// Styles `self` with the third largest text size.
    #[must_use]
    fn xx_large(self) -> Style {
        self.with_dynamic(&TextSize, TextSize6)
            .with_dynamic(&LineHeight, LineHeight6)
    }

    /// Styles `self` with the fourth largest text size.
    #[must_use]
    fn x_large(self) -> Style {
        self.with_dynamic(&TextSize, TextSize5)
            .with_dynamic(&LineHeight, LineHeight5)
    }

    /// Styles `self` with the fifth largest text size.
    #[must_use]
    fn large(self) -> Style {
        self.with_dynamic(&TextSize, TextSize4)
            .with_dynamic(&LineHeight, LineHeight4)
    }

    /// Styles `self` with the third smallest text size.
    #[must_use]
    fn default_size(self) -> Style {
        self.with_dynamic(&TextSize, TextSize3)
            .with_dynamic(&LineHeight, LineHeight3)
    }

    /// Styles `self` with the second smallest text size.
    #[must_use]
    fn small(self) -> Style {
        self.with_dynamic(&TextSize, TextSize2)
            .with_dynamic(&LineHeight, LineHeight2)
    }

    /// Styles `self` with the smallest text size.
    #[must_use]
    fn x_small(self) -> Style {
        self.with_dynamic(&TextSize, TextSize1)
            .with_dynamic(&LineHeight, LineHeight1)
    }

    /// Sets the widget that should be focused next.
    ///
    /// Cushy automatically determines reverse tab order by using this same
    /// relationship.
    fn with_next_focus(self, next_focus: impl IntoValue<Option<WidgetId>>) -> WidgetInstance {
        self.make_widget().with_next_focus(next_focus)
    }

    /// Sets this widget to be enabled/disabled based on `enabled` and returns
    /// self.
    ///
    /// If this widget is disabled, all children widgets will also be disabled.
    ///
    /// # Panics
    ///
    /// This function can only be called when one instance of the widget exists.
    /// If any clones exist, a panic will occur.
    fn with_enabled(self, enabled: impl IntoValue<bool>) -> WidgetInstance {
        self.make_widget().with_enabled(enabled)
    }

    /// Sets this widget as a "default" widget.
    ///
    /// Default widgets are automatically activated when the user signals they
    /// are ready for the default action to occur.
    ///
    /// Example widgets this is used for are:
    ///
    /// - Submit buttons on forms
    /// - Ok buttons
    #[must_use]
    fn into_default(self) -> WidgetInstance {
        self.make_widget().into_default()
    }

    /// Sets this widget as an "escape" widget.
    ///
    /// Escape widgets are automatically activated when the user signals they
    /// are ready to escape their current situation.
    ///
    /// Example widgets this is used for are:
    ///
    /// - Close buttons
    /// - Cancel buttons
    #[must_use]
    fn into_escape(self) -> WidgetInstance {
        self.make_widget().into_escape()
    }

    /// Returns a collection of widgets using `self` and `other`.
    fn and(self, other: impl MakeWidget) -> WidgetList {
        let mut children = WidgetList::new();
        children.push(self);
        children.push(other);
        children
    }

    /// Chains `self` and `others` into a [`WidgetList`].
    fn chain<W: MakeWidget>(self, others: impl IntoIterator<Item = W>) -> WidgetList {
        let others = others.into_iter();
        let mut widgets = WidgetList::with_capacity(others.size_hint().0 + 1);
        widgets.push(self);
        widgets.extend(others);
        widgets
    }

    /// Expands `self` to grow to fill its parent.
    #[must_use]
    fn expand(self) -> Expand {
        Expand::new(self)
    }

    /// Expands `self` to grow to fill its parent proportionally with other
    /// weighted siblings.
    #[must_use]
    fn expand_weighted(self, weight: u8) -> Expand {
        Expand::weighted(weight, self)
    }

    /// Expands `self` to grow to fill its parent horizontally.
    #[must_use]
    fn expand_horizontally(self) -> Expand {
        Expand::horizontal(self)
    }

    /// Expands `self` to grow to fill its parent vertically.
    #[must_use]
    fn expand_vertically(self) -> Expand {
        Expand::vertical(self)
    }

    /// Resizes `self` to `size`.
    #[must_use]
    fn size<T>(self, size: Size<T>) -> Resize
    where
        T: Into<DimensionRange>,
    {
        Resize::to(size, self)
    }

    /// Resizes `self` to `width`.
    ///
    /// `width` can be an any of:
    ///
    /// - [`Dimension`]
    /// - [`Px`]
    /// - [`Lp`](crate::figures::units::Lp)
    /// - A range of any fo the above.
    #[must_use]
    fn width(self, width: impl Into<DimensionRange>) -> Resize {
        Resize::from_width(width, self)
    }

    /// Resizes `self` to `height`.
    ///
    /// `height` can be an any of:
    ///
    /// - [`Dimension`]
    /// - [`Px`]
    /// - [`Lp`](crate::figures::units::Lp)
    /// - A range of any fo the above.
    #[must_use]
    fn height(self, height: impl Into<DimensionRange>) -> Resize {
        Resize::from_height(height, self)
    }

    /// Returns this widget as the contents of a clickable button.
    fn into_button(self) -> Button {
        Button::new(self)
    }

    /// Returns this widget as the contents of a clickable button.
    fn to_button(&self) -> Button
    where
        Self: Clone,
    {
        self.clone().into_button()
    }

    /// Returns this widget as the label of a Checkbox.
    fn into_checkbox(self, value: impl IntoDynamic<CheckboxState>) -> Checkbox {
        value.into_checkbox(self)
    }

    /// Returns this widget as the label of a Checkbox.
    fn to_checkbox(&self, value: impl IntoDynamic<CheckboxState>) -> Checkbox
    where
        Self: Clone,
    {
        self.clone().into_checkbox(value)
    }

    /// Aligns `self` to the center vertically and horizontally.
    #[must_use]
    fn centered(self) -> Align {
        Align::centered(self)
    }

    /// Aligns `self` to the left.
    fn align_left(self) -> Align {
        self.centered().align_left()
    }

    /// Aligns `self` to the right.
    fn align_right(self) -> Align {
        self.centered().align_right()
    }

    /// Aligns `self` to the top.
    fn align_top(self) -> Align {
        self.centered().align_top()
    }

    /// Aligns `self` to the bottom.
    fn align_bottom(self) -> Align {
        self.centered().align_bottom()
    }

    /// Fits `self` horizontally within its parent.
    fn fit_horizontally(self) -> Align {
        self.centered().fit_horizontally()
    }

    /// Fits `self` vertically within its parent.
    fn fit_vertically(self) -> Align {
        self.centered().fit_vertically()
    }

    /// Allows scrolling `self` both vertically and horizontally.
    #[must_use]
    fn scroll(self) -> Scroll {
        Scroll::new(self)
    }

    /// Allows scrolling `self` vertically.
    #[must_use]
    fn vertical_scroll(self) -> Scroll {
        Scroll::vertical(self)
    }

    /// Allows scrolling `self` horizontally.
    #[must_use]
    fn horizontal_scroll(self) -> Scroll {
        Scroll::horizontal(self)
    }

    /// Creates a [`WidgetRef`] for use as child widget.
    #[must_use]
    fn widget_ref(self) -> WidgetRef {
        WidgetRef::new(self)
    }

    /// Wraps `self` in a [`Container`].
    fn contain(self) -> Container {
        Container::new(self)
    }

    /// Wraps `self` in a [`Container`] with the specified level.
    fn contain_level(self, level: impl IntoValue<ContainerLevel>) -> Container {
        self.contain().contain_level(level)
    }

    /// Returns a new widget that renders `color` behind `self`.
    fn background_color(self, color: impl IntoValue<Color>) -> Container {
        self.contain().pad_by(Px::ZERO).background_color(color)
    }

    /// Wraps `self` with the default padding.
    fn pad(self) -> Container {
        self.contain().transparent()
    }

    /// Wraps `self` with the specified padding.
    fn pad_by(self, padding: impl IntoValue<Edges<Dimension>>) -> Container {
        self.contain().transparent().pad_by(padding)
    }

    /// Applies `theme` to `self` and its children.
    fn themed(self, theme: impl IntoValue<ThemePair>) -> Themed {
        Themed::new(theme, self)
    }

    /// Applies `mode` to `self` and its children.
    fn themed_mode(self, mode: impl IntoValue<ThemeMode>) -> ThemedMode {
        ThemedMode::new(mode, self)
    }

    /// Returns a widget that collapses `self` horizontally based on the dynamic boolean value.
    ///
    /// This widget will be collapsed when the dynamic contains `true`, and
    /// revealed when the dynamic contains `false`.
    fn collapse_horizontally(self, collapse_when: impl IntoDynamic<bool>) -> Collapse {
        Collapse::horizontal(collapse_when, self)
    }

    /// Returns a widget that collapses `self` vertically based on the dynamic
    /// boolean value.
    ///
    /// This widget will be collapsed when the dynamic contains `true`, and
    /// revealed when the dynamic contains `false`.
    fn collapse_vertically(self, collapse_when: impl IntoDynamic<bool>) -> Collapse {
        Collapse::vertical(collapse_when, self)
    }

    /// Returns a new widget that allows hiding and showing `contents`.
    fn disclose(self) -> Disclose {
        Disclose::new(self)
    }

    /// Returns a widget that shows validation errors and/or hints.
    fn validation(self, validation: impl IntoDynamic<Validation>) -> Validated {
        Validated::new(validation, self)
    }

    /// Returns a widget that shows `tip` on `layer` when `self` is hovered.
    fn tooltip(self, layer: &OverlayLayer, tip: impl MakeWidget) -> Tooltipped {
        layer.new_tooltip(tip, self)
    }
}

/// A type that can create a [`WidgetInstance`] with a preallocated
/// [`WidgetId`].
pub trait MakeWidgetWithTag: Sized {
    /// Returns a new [`WidgetInstance`] whose [`WidgetId`] comes from `tag`.
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance;
}

impl<T> MakeWidgetWithTag for T
where
    T: Widget,
{
    fn make_with_tag(self, id: WidgetTag) -> WidgetInstance {
        WidgetInstance::with_id(self, id)
    }
}

impl<T> MakeWidget for T
where
    T: MakeWidgetWithTag,
{
    fn make_widget(self) -> WidgetInstance {
        self.make_with_tag(WidgetTag::unique())
    }
}

impl MakeWidget for WidgetInstance {
    fn make_widget(self) -> WidgetInstance {
        self
    }
}

impl MakeWidgetWithTag for Color {
    fn make_with_tag(self, id: WidgetTag) -> WidgetInstance {
        Space::colored(self).make_with_tag(id)
    }
}

/// A type that represents whether an event has been handled or ignored.
pub type EventHandling = ControlFlow<EventHandled, EventIgnored>;

/// A marker type that represents a handled event.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]

pub struct EventHandled;
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// A marker type that represents an ignored event.
pub struct EventIgnored;

/// An [`EventHandling`] value that represents a handled event.
pub const HANDLED: EventHandling = EventHandling::Break(EventHandled);

/// An [`EventHandling`] value that represents an ignored event.
pub const IGNORED: EventHandling = EventHandling::Continue(EventIgnored);

pub(crate) trait AnyWidget: Widget {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> AnyWidget for T
where
    T: Widget,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// An instance of a [`Widget`].
#[derive(Clone)]
pub struct WidgetInstance {
    data: Arc<WidgetInstanceData>,
}

impl Debug for WidgetInstance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.data.widget.try_lock() {
            Some(widget) => widget.summarize(f),
            None => f.debug_struct("WidgetInstance").finish_non_exhaustive(),
        }
    }
}

#[derive(Debug)]
struct WidgetInstanceData {
    id: WidgetId,
    default: bool,
    cancel: bool,
    next_focus: Value<Option<WidgetId>>,
    enabled: Value<bool>,
    widget: Box<Mutex<dyn AnyWidget>>,
}

impl WidgetInstance {
    /// Returns a new instance containing `widget` that is assigned the unique
    /// `id` provided.
    pub fn with_id<W>(widget: W, id: WidgetTag) -> Self
    where
        W: Widget,
    {
        Self {
            data: Arc::new(WidgetInstanceData {
                id: id.into(),
                next_focus: Value::default(),
                default: false,
                cancel: false,
                widget: Box::new(Mutex::new(widget)),
                enabled: Value::Constant(true),
            }),
        }
    }

    /// Returns a new instance containing `widget`.
    pub fn new<W>(widget: W) -> Self
    where
        W: Widget,
    {
        Self::with_id(widget, WidgetTag::unique())
    }

    /// Returns the unique id of this widget instance.
    #[must_use]
    pub fn id(&self) -> WidgetId {
        self.data.id
    }

    /// Sets the widget that should be focused next.
    ///
    /// Cushy automatically determines reverse tab order by using this same
    /// relationship.
    ///
    /// # Panics
    ///
    /// This function can only be called when one instance of the widget exists.
    /// If any clones exist, a panic will occur.
    #[must_use]
    pub fn with_next_focus(
        mut self,
        next_focus: impl IntoValue<Option<WidgetId>>,
    ) -> WidgetInstance {
        let data = Arc::get_mut(&mut self.data)
            .expect("with_next_focus can only be called on newly created widget instances");
        data.next_focus = next_focus.into_value();
        self
    }

    /// Sets this widget to be enabled/disabled based on `enabled` and returns
    /// self.
    ///
    /// If this widget is disabled, all children widgets will also be disabled.
    ///
    /// # Panics
    ///
    /// This function can only be called when one instance of the widget exists.
    /// If any clones exist, a panic will occur.
    #[must_use]
    pub fn with_enabled(mut self, enabled: impl IntoValue<bool>) -> WidgetInstance {
        let data = Arc::get_mut(&mut self.data)
            .expect("with_enabled can only be called on newly created widget instances");
        data.enabled = enabled.into_value();
        self
    }

    /// Sets this widget as a "default" widget.
    ///
    /// Default widgets are automatically activated when the user signals they
    /// are ready for the default action to occur.
    ///
    /// Example widgets this is used for are:
    ///
    /// - Submit buttons on forms
    /// - Ok buttons
    ///
    /// # Panics
    ///
    /// This function can only be called when one instance of the widget exists.
    /// If any clones exist, a panic will occur.
    #[must_use]
    pub fn into_default(mut self) -> WidgetInstance {
        let data = Arc::get_mut(&mut self.data)
            .expect("with_next_focus can only be called on newly created widget instances");
        data.default = true;
        self
    }

    /// Sets this widget as an "escape" widget.
    ///
    /// Escape widgets are automatically activated when the user signals they
    /// are ready to escape their current situation.
    ///
    /// Example widgets this is used for are:
    ///
    /// - Close buttons
    /// - Cancel buttons
    ///
    /// # Panics
    ///
    /// This function can only be called when one instance of the widget exists.
    /// If any clones exist, a panic will occur.
    #[must_use]
    pub fn into_escape(mut self) -> WidgetInstance {
        let data = Arc::get_mut(&mut self.data)
            .expect("with_next_focus can only be called on newly created widget instances");
        data.cancel = true;
        self
    }

    /// Locks the widget for exclusive access. Locking widgets should only be
    /// done for brief moments of time when you are certain no deadlocks can
    /// occur due to other widget locks being held.
    #[must_use]
    pub fn lock(&self) -> WidgetGuard<'_> {
        WidgetGuard(self.data.widget.lock())
    }

    /// Returns the id of the widget that should receive focus after this
    /// widget.
    ///
    /// This value comes from [`MakeWidget::with_next_focus()`].
    #[must_use]
    pub fn next_focus(&self) -> Option<WidgetId> {
        self.data.next_focus.get()
    }

    /// Returns true if this is a default widget.
    ///
    /// See [`MakeWidget::into_default()`] for more information.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.data.default
    }

    /// Returns true if this is an escape widget.
    ///
    /// See [`MakeWidget::into_escape()`] for more information.
    #[must_use]
    pub fn is_escape(&self) -> bool {
        self.data.cancel
    }

    pub(crate) fn enabled(&self, context: &WindowHandle) -> bool {
        if let Value::Dynamic(dynamic) = &self.data.enabled {
            dynamic.inner_redraw_when_changed(context.clone());
        }
        self.data.enabled.get()
    }

    /// Returns a new window containing `self` as the root widget.
    pub fn to_window(&self) -> Window<Self>
    where
        Self: Clone,
    {
        self.clone().make_window()
    }
}

impl AsRef<WidgetId> for WidgetInstance {
    fn as_ref(&self) -> &WidgetId {
        &self.data.id
    }
}

impl Eq for WidgetInstance {}

impl PartialEq for WidgetInstance {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }
}

impl WindowBehavior for WidgetInstance {
    type Context = Self;

    fn initialize(
        _window: &mut RunningWindow<kludgine::app::Window<'_, WindowCommand>>,
        context: Self::Context,
    ) -> Self {
        context
    }

    fn make_root(&mut self) -> WidgetInstance {
        self.clone()
    }
}

/// A function that can be invoked with a parameter (`T`) and returns `R`.
///
/// This type is used by widgets to signal various events.
pub struct Callback<T = (), R = ()>(Box<dyn CallbackFunction<T, R>>);

impl<T, R> Debug for Callback<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Callback")
            .field(&std::ptr::from_ref::<Self>(self))
            .finish()
    }
}

impl<T, R> Eq for Callback<T, R> {}

impl<T, R> PartialEq for Callback<T, R> {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl<T, R> Callback<T, R> {
    /// Returns a new instance that calls `function` each time the callback is
    /// invoked.
    pub fn new<F>(function: F) -> Self
    where
        F: FnMut(T) -> R + Send + 'static,
    {
        Self(Box::new(function))
    }

    /// Invokes the wrapped function and returns the produced value.
    pub fn invoke(&mut self, value: T) -> R {
        self.0.invoke(value)
    }
}

trait CallbackFunction<T, R>: Send {
    fn invoke(&mut self, value: T) -> R;
}

impl<T, R, F> CallbackFunction<T, R> for F
where
    F: FnMut(T) -> R + Send,
{
    fn invoke(&mut self, value: T) -> R {
        self(value)
    }
}

/// A [`Callback`] that can be cloned.
///
/// Only one thread can be invoking a shared callback at any given time.
pub struct SharedCallback<T = (), R = ()>(Arc<Mutex<Callback<T, R>>>);

impl<T, R> SharedCallback<T, R> {
    /// Returns a new instance that calls `function` each time the callback is
    /// invoked.
    pub fn new<F>(function: F) -> Self
    where
        F: FnMut(T) -> R + Send + 'static,
    {
        Self::from(Callback::new(function))
    }

    /// Invokes the wrapped function and returns the produced value.
    pub fn invoke(&self, value: T) -> R {
        self.0.lock().invoke(value)
    }
}

impl<T, R> Debug for SharedCallback<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SharedCallback")
            .field(&Arc::as_ptr(&self.0))
            .finish()
    }
}

impl<T, R> Eq for SharedCallback<T, R> {}

impl<T, R> PartialEq for SharedCallback<T, R> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl<T, R> Clone for SharedCallback<T, R> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T, R> From<Callback<T, R>> for SharedCallback<T, R> {
    fn from(callback: Callback<T, R>) -> Self {
        Self(Arc::new(Mutex::new(callback)))
    }
}

/// A function that can be invoked once with a parameter (`T`) and returns `R`.
///
/// This type is used by widgets to signal an event that can happen only onceq.
pub struct OnceCallback<T = (), R = ()>(Box<dyn OnceCallbackFunction<T, R>>);

impl<T, R> Debug for OnceCallback<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("OnceCallback")
            .field(&std::ptr::from_ref::<Self>(self))
            .finish()
    }
}

impl<T, R> Eq for OnceCallback<T, R> {}

impl<T, R> PartialEq for OnceCallback<T, R> {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl<T, R> OnceCallback<T, R> {
    /// Returns a new instance that calls `function` when the callback is
    /// invoked.
    pub fn new<F>(function: F) -> Self
    where
        F: FnOnce(T) -> R + Send + 'static,
    {
        Self(Box::new(Some(function)))
    }

    /// Invokes the wrapped function and returns the produced value.
    pub fn invoke(mut self, value: T) -> R {
        self.0.invoke(value)
    }
}

trait OnceCallbackFunction<T, R>: Send {
    fn invoke(&mut self, value: T) -> R;
}

impl<T, R, F> OnceCallbackFunction<T, R> for Option<F>
where
    F: FnOnce(T) -> R + Send,
{
    fn invoke(&mut self, value: T) -> R {
        (self.take().assert("invoked once"))(value)
    }
}

/// A [`Widget`] that has been attached to a widget hierarchy.
///
/// Because [`WidgetInstance`]s can be reused, a mounted widget can be unmounted
/// and eventually remounted. To ensure the widget is in a consistent state, all
/// types that own `MountedWidget`s should call
/// [`MountedWidget::remount_if_needed`] during their `mount()` functions.
#[derive(Clone)]
pub struct MountedWidget {
    pub(crate) node_id: LotId,
    pub(crate) widget: WidgetInstance,
    pub(crate) tree: WeakTree,
}

impl Debug for MountedWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.widget, f)
    }
}

impl MountedWidget {
    pub(crate) fn tree(&self) -> Tree {
        self.tree.upgrade().expect("tree missing")
    }

    /// Remounts this widget, if it was previously unmounted.
    pub fn remount_if_needed(&mut self, context: &mut EventContext<'_>) {
        if !self.is_mounted() {
            *self = context.push_child(self.widget.clone());
        }
    }

    /// Returns true if this widget is still mounted in a window.
    #[must_use]
    pub fn is_mounted(&self) -> bool {
        let Some(tree) = self.tree.upgrade() else {
            return false;
        };
        tree.widget_is_valid(self.node_id)
    }

    /// Locks the widget for exclusive access. Locking widgets should only be
    /// done for brief moments of time when you are certain no deadlocks can
    /// occur due to other widget locks being held.
    #[must_use]
    pub fn lock(&self) -> WidgetGuard<'_> {
        self.widget.lock()
    }

    /// Invalidates this widget.
    pub fn invalidate(&self) {
        let Some(tree) = self.tree.upgrade() else {
            return;
        };
        tree.invalidate(self.node_id, false);
    }

    pub(crate) fn set_layout(&self, rect: Rect<Px>) {
        self.tree().set_layout(self.node_id, rect);
    }

    /// Returns the unique id of this widget instance.
    #[must_use]
    pub fn id(&self) -> WidgetId {
        self.widget.id()
    }

    /// Returns the underlying widget instance
    #[must_use]
    pub const fn instance(&self) -> &WidgetInstance {
        &self.widget
    }

    /// Returns the next widget to focus after this widget.
    ///
    /// This function returns the value set in
    /// [`MakeWidget::with_next_focus()`].
    #[must_use]
    pub fn next_focus(&self) -> Option<MountedWidget> {
        self.widget
            .next_focus()
            .and_then(|next_focus| self.tree.upgrade()?.widget(next_focus))
    }

    /// Returns the widget to focus before this widget.
    ///
    /// There is no direct way to set this value. This relationship is created
    /// automatically using [`MakeWidget::with_next_focus()`].
    #[must_use]
    pub fn previous_focus(&self) -> Option<MountedWidget> {
        self.tree.upgrade()?.previous_focus(self.id())
    }

    /// Returns the next or previous focus target, if one was set using
    /// [`MakeWidget::with_next_focus()`].
    #[must_use]
    pub fn explicit_focus_target(&self, advance: bool) -> Option<MountedWidget> {
        if advance {
            self.next_focus()
        } else {
            self.previous_focus()
        }
    }

    /// Returns the region that the widget was last rendered at.
    #[must_use]
    pub fn last_layout(&self) -> Option<Rect<Px>> {
        self.tree.upgrade()?.layout(self.node_id)
    }

    /// Returns the effective styles for the current tree.
    #[must_use]
    pub fn effective_styles(&self) -> Styles {
        self.tree().effective_styles(self.node_id)
    }

    /// Returns true if this widget is the currently active widget.
    #[must_use]
    pub fn active(&self) -> bool {
        self.tree().active_widget() == Some(self.node_id)
    }

    pub(crate) fn enabled(&self, handle: &WindowHandle) -> bool {
        self.tree().is_enabled(self.node_id, handle)
    }

    /// Returns true if this widget is currently the hovered widget.
    #[must_use]
    pub fn hovered(&self) -> bool {
        self.tree().is_hovered(self.node_id)
    }

    /// Returns true if this widget that is directly beneath the cursor.
    #[must_use]
    pub fn primary_hover(&self) -> bool {
        self.tree().hovered_widget() == Some(self.node_id)
    }

    /// Returns true if this widget is the currently focused widget.
    #[must_use]
    pub fn focused(&self) -> bool {
        self.tree().focused_widget() == Some(self.node_id)
    }

    /// Returns the parent of this widget.
    #[must_use]
    pub fn parent(&self) -> Option<MountedWidget> {
        let tree = self.tree.upgrade()?;

        tree.parent(self.node_id)
            .and_then(|id| tree.widget_from_node(id))
    }

    /// Returns true if this node has a parent.
    #[must_use]
    pub fn has_parent(&self) -> bool {
        let Some(tree) = self.tree.upgrade() else {
            return false;
        };
        tree.parent(self.node_id).is_some()
    }

    pub(crate) fn attach_styles(&self, styles: Value<Styles>) {
        self.tree().attach_styles(self.node_id, styles);
    }

    pub(crate) fn attach_theme(&self, theme: Value<ThemePair>) {
        self.tree().attach_theme(self.node_id, theme);
    }

    pub(crate) fn attach_theme_mode(&self, theme: Value<ThemeMode>) {
        self.tree().attach_theme_mode(self.node_id, theme);
    }

    pub(crate) fn overidden_theme(
        &self,
    ) -> (Styles, Option<Value<ThemePair>>, Option<Value<ThemeMode>>) {
        self.tree().overriden_theme(self.node_id)
    }

    pub(crate) fn begin_layout(&self, constraints: Size<ConstraintLimit>) -> Option<Size<UPx>> {
        self.tree().begin_layout(self.node_id, constraints)
    }

    pub(crate) fn persist_layout(&self, constraints: Size<ConstraintLimit>, size: Size<UPx>) {
        self.tree().persist_layout(self.node_id, constraints, size);
    }

    pub(crate) fn visually_ordered_children(&self, order: VisualOrder) -> Vec<MountedWidget> {
        self.tree().visually_ordered_children(self.node_id, order)
    }
}

impl AsRef<WidgetId> for MountedWidget {
    fn as_ref(&self) -> &WidgetId {
        self.widget.as_ref()
    }
}

impl PartialEq for MountedWidget {
    fn eq(&self, other: &Self) -> bool {
        self.widget == other.widget
    }
}

impl PartialEq<WidgetInstance> for MountedWidget {
    fn eq(&self, other: &WidgetInstance) -> bool {
        &self.widget == other
    }
}

/// Exclusive access to a widget.
///
/// This type is powered by a `Mutex`, which means care must be taken to prevent
/// deadlocks.
pub struct WidgetGuard<'a>(MutexGuard<'a, dyn AnyWidget>);

impl WidgetGuard<'_> {
    pub(crate) fn as_widget(&mut self) -> &mut dyn AnyWidget {
        &mut *self.0
    }

    /// Returns a reference to `T` if it is the type contained.
    #[must_use]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.0.as_any().downcast_ref()
    }

    /// Returns an exclusive reference to `T` if it is the type contained.
    #[must_use]
    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: 'static,
    {
        self.0.as_any_mut().downcast_mut()
    }
}

/// A list of [`Widget`]s without a layout strategy.
///
/// To use a `WidgetList` in a user interface, a choice must be made for how
/// each child should be positioned. The built-in widgets that can layout a
/// `WidgetList` are:
///
/// - As rows: [`Stack::rows`] / [`Self::into_rows`]
/// - As columns: [`Stack::columns`] / [`Self::into_columns`]
/// - Positioned on top of each other in the Z orientation: [`Layers::new`] /
///   [`Self::into_layers`]
/// - Layout horizontally, wrapping into multiple rows as needed: [`Wrap::new`]
///   / [`Self::into_wrap`].
#[derive(Default, Eq, PartialEq)]
#[must_use]
pub struct WidgetList {
    ordered: Vec<WidgetInstance>,
}

impl WidgetList {
    /// Returns an empty list.
    pub const fn new() -> Self {
        Self {
            ordered: Vec::new(),
        }
    }

    /// Returns a list with enough capacity to hold `capacity` widgets without
    /// reallocation.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            ordered: Vec::with_capacity(capacity),
        }
    }

    /// Pushes `widget` into the list.
    pub fn push<W>(&mut self, widget: W)
    where
        W: MakeWidget,
    {
        self.ordered.push(widget.make_widget());
    }

    /// Inserts `widget` into the list at `index`.
    pub fn insert<W>(&mut self, index: usize, widget: W)
    where
        W: MakeWidget,
    {
        self.ordered.insert(index, widget.make_widget());
    }

    /// Extends this collection with the contents of `iter`.
    pub fn extend<T, Iter>(&mut self, iter: Iter)
    where
        Iter: IntoIterator<Item = T>,
        T: MakeWidget,
    {
        self.ordered.extend(iter.into_iter().map(T::make_widget));
    }

    /// Adds `widget` to self and returns the updated list.
    pub fn and<W>(mut self, widget: W) -> Self
    where
        W: MakeWidget,
    {
        self.push(widget);
        self
    }

    /// Chains `self` and `others` into a [`WidgetList`].
    pub fn chain<T, Iter>(mut self, iter: Iter) -> Self
    where
        Iter: IntoIterator<Item = T>,
        T: MakeWidget,
    {
        self.extend(iter);
        self
    }

    /// Returns the number of widgets in this list.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ordered.len()
    }

    /// Returns true if there are no widgets in this list.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ordered.is_empty()
    }

    /// Truncates the collection of children to `length`.
    ///
    /// If this collection is already smaller or the same size as `length`, this
    /// function does nothing.
    pub fn truncate(&mut self, length: usize) {
        self.ordered.truncate(length);
    }

    /// Clear the list
    pub fn clear(&mut self) {
        self.ordered.clear();
    }

    /// Returns `self` as a vertical [`Stack`] of rows.
    #[must_use]
    pub fn into_rows(self) -> Stack {
        Stack::rows(self)
    }

    /// Returns `self` as a horizontal [`Stack`] of columns.
    #[must_use]
    pub fn into_columns(self) -> Stack {
        Stack::columns(self)
    }

    /// Returns `self` as [`Layers`], with the widgets being stacked in the Z
    /// direction.
    #[must_use]
    pub fn into_layers(self) -> Layers {
        Layers::new(self)
    }

    /// Returns a [`Wrap`] that lays the children out horizontally, wrapping
    /// into additional rows as needed.
    #[must_use]
    pub fn into_wrap(self) -> Wrap {
        Wrap::new(self)
    }

    /// Returns `self` as an unordered [`List`].
    #[must_use]
    pub fn into_list(self) -> List {
        List::new(self)
    }

    /// Synchronizes this list of children with another collection.
    ///
    /// This function updates `collection` by calling `change_fn` for each
    /// operation that needs to be performed to synchronize. The algorithm first
    /// mounts/inserts all new children before sending a final change to
    /// `change_fn`: [`ChildrenSyncChange::Truncate`].
    pub fn synchronize_with<Collection>(
        &self,
        collection: &mut Collection,
        get_index: impl Fn(&Collection, usize) -> Option<&WidgetInstance>,
        mut change_fn: impl FnMut(&mut Collection, ChildrenSyncChange),
    ) {
        for (index, widget) in self.iter().enumerate() {
            if get_index(collection, index).map_or(true, |child| child != widget) {
                // These entries do not match. See if we can find the
                // new id somewhere else, if so we can swap the entries.
                if let Some(Some(swap_index)) = (index + 1..usize::MAX).find_map(|index| {
                    if let Some(child) = get_index(collection, index) {
                        if widget == child {
                            Some(Some(index))
                        } else {
                            None
                        }
                    } else {
                        Some(None)
                    }
                }) {
                    change_fn(collection, ChildrenSyncChange::Swap(index, swap_index));
                } else {
                    change_fn(
                        collection,
                        ChildrenSyncChange::Insert(index, widget.clone()),
                    );
                }
            }
        }

        change_fn(collection, ChildrenSyncChange::Truncate(self.len()));
    }
}

impl Debug for WidgetList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.ordered, f)
    }
}

impl Dynamic<WidgetList> {
    /// Returns `self` as a vertical [`Stack`] of rows.
    #[must_use]
    pub fn into_rows(self) -> Stack {
        Stack::rows(self)
    }

    /// Returns `self` as a vertical [`Stack`] of rows.
    #[must_use]
    pub fn to_rows(&self) -> Stack {
        self.clone().into_rows()
    }

    /// Returns `self` as a horizontal [`Stack`] of columns.
    #[must_use]
    pub fn into_columns(self) -> Stack {
        Stack::columns(self)
    }

    /// Returns `self` as a horizontal [`Stack`] of columns.
    #[must_use]
    pub fn to_columns(&self) -> Stack {
        self.clone().into_columns()
    }

    /// Returns `self` as [`Layers`], with the widgets being stacked in the Z
    /// direction.
    #[must_use]
    pub fn into_layers(self) -> Layers {
        Layers::new(self)
    }

    /// Returns `self` as [`Layers`], with the widgets being stacked in the Z
    /// direction.
    #[must_use]
    pub fn to_layers(&self) -> Layers {
        self.clone().into_layers()
    }

    /// Returns `self` as an unordered [`List`].
    #[must_use]
    pub fn into_list(self) -> List {
        List::new(self)
    }

    /// Returns `self` as an unordered [`List`].
    #[must_use]
    pub fn to_list(self) -> List {
        self.clone().into_list()
    }

    /// Returns a [`Wrap`] that lays the children out horizontally, wrapping
    /// into additional rows as needed.
    #[must_use]
    pub fn into_wrap(self) -> Wrap {
        Wrap::new(self)
    }

    /// Returns a [`Wrap`] that lays the children out horizontally, wrapping
    /// into additional rows as needed.
    #[must_use]
    pub fn to_wrap(&self) -> Wrap {
        self.clone().into_wrap()
    }
}

impl FromIterator<WidgetList> for WidgetList {
    fn from_iter<T: IntoIterator<Item = WidgetList>>(iter: T) -> Self {
        let mut iter = iter.into_iter();
        let Some(mut dest) = iter.next() else {
            return Self::new();
        };
        for other in iter {
            dest.extend(other);
        }
        dest
    }
}

impl<W> FromIterator<W> for WidgetList
where
    W: MakeWidget,
{
    fn from_iter<T: IntoIterator<Item = W>>(iter: T) -> Self {
        Self {
            ordered: iter.into_iter().map(MakeWidget::make_widget).collect(),
        }
    }
}

impl Deref for WidgetList {
    type Target = [WidgetInstance];

    fn deref(&self) -> &Self::Target {
        &self.ordered
    }
}

impl DerefMut for WidgetList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ordered
    }
}

impl IntoIterator for WidgetList {
    type IntoIter = std::vec::IntoIter<WidgetInstance>;
    type Item = WidgetInstance;

    fn into_iter(self) -> Self::IntoIter {
        self.ordered.into_iter()
    }
}

impl<'a> IntoIterator for &'a WidgetList {
    type IntoIter = slice::Iter<'a, WidgetInstance>;
    type Item = &'a WidgetInstance;

    fn into_iter(self) -> Self::IntoIter {
        self.ordered.iter()
    }
}

impl<I: IntoIterator> MakeWidgetList for I
where
    I::Item: MakeWidget,
{
    fn make_widget_list(self) -> WidgetList {
        self.into_iter().collect()
    }
}

/// Allows to convert collections or iterators directly into [`Stack`], [`Layers`], etc.
///
/// ```
/// use cushy::widget::{MakeWidget, MakeWidgetList};
///
/// vec!["hello", "label"].into_rows();
///
/// vec!["hello", "button"]
///     .into_iter()
///     .map(|l| l.into_button())
///     .into_columns();
/// ```
pub trait MakeWidgetList: Sized {
    /// Returns self as a `WidgetList`.
    fn make_widget_list(self) -> WidgetList;

    /// Adds `widget` to self and returns the updated list.
    fn and<W>(self, widget: W) -> WidgetList
    where
        W: MakeWidget,
    {
        let mut list = self.make_widget_list();
        list.push(widget);
        list
    }
}

/// A type that can be converted to a `Value<WidgetList>`.
pub trait IntoWidgetList: Sized {
    /// Returns this list of widgets as a `Value<WidgetList>`.
    fn into_widget_list(self) -> Value<WidgetList>;

    /// Returns `self` as a vertical [`Stack`] of rows.
    #[must_use]
    fn into_rows(self) -> Stack {
        Stack::rows(self.into_widget_list())
    }

    /// Returns `self` as a horizontal [`Stack`] of columns.
    #[must_use]
    fn into_columns(self) -> Stack {
        Stack::columns(self.into_widget_list())
    }

    /// Returns `self` as [`Layers`], with the widgets being stacked in the Z
    /// direction.
    #[must_use]
    fn into_layers(self) -> Layers {
        Layers::new(self.into_widget_list())
    }

    /// Returns a [`Wrap`] that lays the children out horizontally, wrapping
    /// into additional rows as needed.
    #[must_use]
    fn into_wrap(self) -> Wrap {
        Wrap::new(self.into_widget_list())
    }

    /// Returns `self` as an unordered [`List`].
    #[must_use]
    fn into_list(self) -> List {
        List::new(self.into_widget_list())
    }
}

impl<T> IntoWidgetList for T
where
    T: MakeWidgetList,
{
    fn into_widget_list(self) -> Value<WidgetList> {
        Value::Constant(self.make_widget_list())
    }
}

impl IntoWidgetList for Dynamic<WidgetList> {
    fn into_widget_list(self) -> Value<WidgetList> {
        Value::Dynamic(self)
    }
}

impl IntoWidgetList for Value<WidgetList> {
    fn into_widget_list(self) -> Value<WidgetList> {
        self
    }
}

/// A change to perform during [`WidgetList::synchronize_with`].
pub enum ChildrenSyncChange {
    /// Insert a new widget at the given index.
    Insert(usize, WidgetInstance),
    /// Swap the widgets at the given indices.
    Swap(usize, usize),
    /// Truncate the collection to the length given.
    Truncate(usize),
}

/// A collection of mounted children.
///
/// This collection is a helper aimed at making it easier to build widgets that
/// contain multiple children widgets. It is used in conjunction with a
/// `Value<WidgetList>`.
#[derive(Debug)]
pub struct MountedChildren<T = MountedWidget> {
    generation: Option<Generation>,
    children: Vec<T>,
}

impl<T> MountedChildren<T>
where
    T: MountableChild,
{
    /// Mounts and unmounts all children needed to be in sync with `children`.
    pub fn synchronize_with(
        &mut self,
        children: &Value<WidgetList>,
        context: &mut EventContext<'_>,
    ) {
        let current_generation = children.generation();
        if current_generation.map_or_else(
            || children.map(WidgetList::len) != self.children.len(),
            |gen| Some(gen) != self.generation,
        ) {
            self.generation = current_generation;
            children.map(|children| {
                children.synchronize_with(
                    self,
                    |this, index| {
                        this.children
                            .get(index)
                            .map(|mounted| mounted.widget().instance())
                    },
                    |this, change| match change {
                        ChildrenSyncChange::Insert(index, widget) => {
                            this.children
                                .insert(index, T::mount(context.push_child(widget), this, index));
                        }
                        ChildrenSyncChange::Swap(a, b) => {
                            this.children.swap(a, b);
                        }
                        ChildrenSyncChange::Truncate(length) => {
                            for removed in this.children.drain(length..) {
                                context.remove_child(&removed.unmount());
                            }
                        }
                    },
                );
            });
        }
    }

    /// Returns an iterator that contains every widget in this collection.
    ///
    /// When the iterator is dropped, this collection will be empty.
    pub fn drain(&mut self) -> vec::Drain<'_, T> {
        self.generation = None;
        self.children.drain(..)
    }

    /// Returns a reference to the children.
    #[must_use]
    pub fn children(&self) -> &[T] {
        &self.children
    }
}

impl<T> Default for MountedChildren<T> {
    fn default() -> Self {
        Self {
            generation: None,
            children: Vec::default(),
        }
    }
}

/// A child in a [`MountedChildren`] collection.
pub trait MountableChild: Sized {
    /// Returns the mounted representation of `widget`.
    fn mount(widget: MountedWidget, into: &MountedChildren<Self>, index: usize) -> Self;
    /// Returns the widget and performs any other cleanup for this widget being unmounted.q
    fn unmount(self) -> MountedWidget;
    /// Returns a reference to the widget.
    fn widget(&self) -> &MountedWidget;
}

impl MountableChild for MountedWidget {
    fn mount(widget: MountedWidget, _into: &MountedChildren<Self>, _index: usize) -> Self {
        widget
    }

    fn widget(&self) -> &MountedWidget {
        self
    }

    fn unmount(self) -> MountedWidget {
        self
    }
}

/// A child widget
#[derive(Clone)]
pub struct WidgetRef {
    instance: WidgetInstance,
    mounted: WindowLocal<MountedWidget>,
}

impl WidgetRef {
    /// Returns a new unmounted child
    pub fn new(widget: impl MakeWidget) -> Self {
        Self {
            instance: widget.make_widget(),
            mounted: WindowLocal::default(),
        }
    }

    /// Returns this child, mounting it in the process if necessary.
    fn mounted_for_context(&mut self, context: &mut impl AsEventContext) -> &MountedWidget {
        let mut context = context.as_event_context();
        self.mounted
            .entry(&context)
            .and_modify(|w| {
                w.remount_if_needed(&mut context.as_event_context());
            })
            .or_insert_with(|| context.push_child(self.instance.clone()))
    }

    /// Returns this child, mounting it in the process if necessary.
    pub fn mount_if_needed(&mut self, context: &mut impl AsEventContext) {
        self.mounted_for_context(context);
    }

    /// Returns this child, mounting it in the process if necessary.
    pub fn mounted(&mut self, context: &mut impl AsEventContext) -> MountedWidget {
        self.mounted_for_context(context).clone()
    }

    /// Returns this child, if it has been mounted.
    #[must_use]
    pub fn as_mounted(&self, context: &WidgetContext<'_>) -> Option<&MountedWidget> {
        self.mounted.get(context)
    }

    /// Returns the a reference to the underlying widget instance.
    #[must_use]
    pub const fn widget(&self) -> &WidgetInstance {
        &self.instance
    }

    /// Unmounts this widget from the window belonging to `context`, if needed.
    pub fn unmount_in(&mut self, context: &mut impl AsEventContext) {
        let mut context = context.as_event_context();
        if let Some(mounted) = self.mounted.clear_for(&context) {
            context.remove_child(&mounted);
        }
    }
}

impl From<WidgetRef> for WindowLocal<MountedWidget> {
    fn from(value: WidgetRef) -> Self {
        value.mounted
    }
}

impl AsRef<WidgetId> for WidgetRef {
    fn as_ref(&self) -> &WidgetId {
        self.instance.as_ref()
    }
}

impl Debug for WidgetRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.instance, f)
    }
}

impl Eq for WidgetRef {}

impl PartialEq for WidgetRef {
    fn eq(&self, other: &Self) -> bool {
        self.instance == other.instance
    }
}

impl ManageWidget for WidgetRef {
    type Managed = Option<MountedWidget>;

    fn manage(&self, context: &WidgetContext<'_>) -> Self::Managed {
        self.mounted
            .get(context)
            .cloned()
            .or_else(|| context.tree.widget(self.instance.id()))
    }
}

/// The unique id of a [`WidgetInstance`].
///
/// Each [`WidgetInstance`] is guaranteed to have a unique [`WidgetId`] across
/// the lifetime of an application.
#[derive(Clone, Copy, Eq, PartialEq, Debug, Hash, Ord, PartialOrd)]
pub struct WidgetId(u64);

impl WidgetId {
    fn unique() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, atomic::Ordering::Acquire))
    }

    /// Finds this widget mounted in this window, if present.
    #[must_use]
    pub fn find_in(self, context: &WidgetContext<'_>) -> Option<MountedWidget> {
        context.tree.widget(self)
    }
}

/// A [`WidgetId`] that has not been assigned to a [`WidgetInstance`].
///
/// This type is passed to [`MakeWidgetWithTag::make_with_tag()`] to create a
/// [`WidgetInstance`] with a preallocated id.
///
/// This type cannot be cloned or copied to ensure only a single widget can be
/// assigned a given [`WidgetId`]. The contained [`WidgetId`] can be accessed
/// via [`id()`](Self::id), `Into<WidgetId>`, or `Deref`.
#[derive(Eq, PartialEq, Debug)]
pub struct WidgetTag(WidgetId);

impl WidgetTag {
    /// Returns a unique tag and its contained id.
    #[must_use]
    pub fn new() -> (Self, WidgetId) {
        let tag = Self::unique();
        let id = *tag;
        (tag, id)
    }

    /// Returns a newly allocated [`WidgetId`] that is guaranteed to be unique
    /// for the lifetime of the application.
    #[must_use]
    pub fn unique() -> Self {
        Self(WidgetId::unique())
    }

    /// Returns the contained widget id.
    #[must_use]
    pub const fn id(&self) -> WidgetId {
        self.0
    }
}

impl From<WidgetTag> for WidgetId {
    fn from(value: WidgetTag) -> Self {
        value.0
    }
}

impl Deref for WidgetTag {
    type Target = WidgetId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
