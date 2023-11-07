//! Types that provide access to the Gooey runtime.
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use kludgine::app::winit::event::{
    DeviceId, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoSigned, Point, Rect, Size};
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::Kludgine;

use crate::graphics::Graphics;
use crate::styles::components::HighlightColor;
use crate::styles::{ComponentDefaultvalue, ComponentDefinition, Styles};
use crate::value::Dynamic;
use crate::widget::{EventHandling, ManagedWidget, WidgetId, WidgetInstance};
use crate::window::{sealed, RunningWindow};
use crate::ConstraintLimit;

/// A context to an event function.
///
/// This type is a combination of a reference to the rendering library,
/// [`Kludgine`], and a [`WidgetContext`].
pub struct EventContext<'context, 'window> {
    /// The context for the widget receiving the event.
    pub widget: WidgetContext<'context, 'window>,
    /// The rendering library's state.
    ///
    /// This is useful for accessing the current [scale](Kludgine::scale) or
    /// information needed to measure and layout text.
    pub kludgine: &'context mut Kludgine,
}

impl<'context, 'window> EventContext<'context, 'window> {
    pub(crate) fn new(
        widget: WidgetContext<'context, 'window>,
        kludgine: &'context mut Kludgine,
    ) -> Self {
        Self { widget, kludgine }
    }

    /// Returns a new `EventContext` with `widget` being referenced in the
    /// contained [`WidgetContext`].
    ///
    /// This function is used when one widget contains other widgets, and the
    /// parent widget needs to invoke events on a child widget. This is done by
    /// creating an `EventContext` pointing to the child and calling the
    /// appropriate function to invoke the event.
    pub fn for_other<'child>(
        &'child mut self,
        widget: ManagedWidget,
    ) -> EventContext<'child, 'window> {
        EventContext::new(self.widget.for_other(widget), self.kludgine)
    }

    /// Invokes [`Widget::hit_test()`](crate::widget::Widget::hit_test) on this
    /// context's widget and returns the result.
    pub fn hit_test(&mut self, location: Point<Px>) -> bool {
        self.current_node
            .clone()
            .lock()
            .as_widget()
            .hit_test(location, self)
    }

    /// Invokes [`Widget::mouse_down()`](crate::widget::Widget::mouse_down) on
    /// this context's widget and returns the result.
    pub fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
    ) -> EventHandling {
        self.current_node
            .clone()
            .lock()
            .as_widget()
            .mouse_down(location, device_id, button, self)
    }

    /// Invokes [`Widget::hit_test()`](crate::widget::Widget::mouse_drag) on
    /// this context's widget and returns the result.
    pub fn mouse_drag(&mut self, location: Point<Px>, device_id: DeviceId, button: MouseButton) {
        self.current_node
            .clone()
            .lock()
            .as_widget()
            .mouse_drag(location, device_id, button, self);
    }

    /// Invokes [`Widget::mouse_up()`](crate::widget::Widget::mouse_up) on this
    /// context's widget and returns the result.
    pub fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
    ) {
        self.current_node
            .clone()
            .lock()
            .as_widget()
            .mouse_up(location, device_id, button, self);
    }

    /// Invokes [`Widget::keyboard_input()`](crate::widget::Widget::keyboard_input) on this
    /// context's widget and returns the result.
    pub fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
    ) -> EventHandling {
        self.current_node.clone().lock().as_widget().keyboard_input(
            device_id,
            input,
            is_synthetic,
            self,
        )
    }

    /// Invokes [`Widget::ime()`](crate::widget::Widget::ime) on this
    /// context's widget and returns the result.
    pub fn ime(&mut self, ime: Ime) -> EventHandling {
        self.current_node.clone().lock().as_widget().ime(ime, self)
    }

    /// Invokes [`Widget::mouse_wheel()`](crate::widget::Widget::mouse_wheel) on this
    /// context's widget and returns the result.
    pub fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    ) -> EventHandling {
        self.current_node
            .clone()
            .lock()
            .as_widget()
            .mouse_wheel(device_id, delta, phase, self)
    }

    pub(crate) fn hover(&mut self, location: Point<Px>) {
        let changes = self.current_node.tree.hover(Some(&self.current_node));
        for unhovered in changes.unhovered {
            let mut context = self.for_other(unhovered.clone());
            unhovered.lock().as_widget().unhover(&mut context);
        }
        for hover in changes.hovered {
            let mut context = self.for_other(hover.clone());
            hover.lock().as_widget().hover(location, &mut context);
        }
    }

    pub(crate) fn clear_hover(&mut self) {
        let changes = self.current_node.tree.hover(None);
        assert!(changes.hovered.is_empty());

        for old_hover in changes.unhovered {
            let mut old_hover_context = self.for_other(old_hover.clone());
            old_hover.lock().as_widget().unhover(&mut old_hover_context);
        }
    }

    pub(crate) fn apply_pending_state(&mut self) {
        let active = self.pending_state.active.clone();
        if self.current_node.tree.active_widget() != active.as_ref().map(ManagedWidget::id) {
            let new = match self.current_node.tree.activate(active.as_ref()) {
                Ok(old) => {
                    if let Some(old) = old {
                        let mut old_context = self.for_other(old.clone());
                        old.lock().as_widget().deactivate(&mut old_context);
                    }
                    true
                }
                Err(_) => false,
            };
            if new {
                if let Some(active) = &active {
                    active
                        .lock()
                        .as_widget()
                        .activate(&mut self.for_other(active.clone()));
                }
                self.pending_state.active = active;
            }
        }

        let focus = self.pending_state.focus.clone();
        if self.current_node.tree.focused_widget() != focus.as_ref().map(ManagedWidget::id) {
            let focus = focus.and_then(|mut focus| loop {
                if focus
                    .lock()
                    .as_widget()
                    .accept_focus(&mut self.for_other(focus.clone()))
                {
                    break Some(focus);
                } else if let Some(next_focus) = focus.next_focus() {
                    focus = next_focus;
                } else {
                    break self.next_focus_after(focus);
                }
            });
            let new = match self.current_node.tree.focus(focus.as_ref()) {
                Ok(old) => {
                    if let Some(old) = old {
                        let mut old_context = self.for_other(old.clone());
                        old.lock().as_widget().blur(&mut old_context);
                    }
                    true
                }
                Err(_) => false,
            };
            if new {
                if let Some(focus) = &focus {
                    focus
                        .lock()
                        .as_widget()
                        .focus(&mut self.for_other(focus.clone()));
                }
                self.pending_state.focus = focus;
            }
        }
    }

    fn next_focus_after(&mut self, mut focus: ManagedWidget) -> Option<ManagedWidget> {
        // First, look within the current focus for any focusable children.
        let stop_at = focus.id();
        if let Some(focus) = self.next_focus_within(&focus, None, stop_at) {
            return Some(focus);
        }

        // Now, look for the next widget in each hierarchy
        let root = loop {
            if let Some(focus) = self.next_focus_sibling(&focus, stop_at) {
                return Some(focus);
            }
            let Some(parent) = focus.parent() else {
                break focus;
            };
            focus = parent;
        };

        // We've exhausted a forward scan, we can now start searching the final
        // parent, which is the root.
        self.next_focus_within(&root, None, stop_at)
    }

    fn next_focus_sibling(
        &mut self,
        focus: &ManagedWidget,
        stop_at: WidgetId,
    ) -> Option<ManagedWidget> {
        self.next_focus_within(&focus.parent()?, Some(focus.id()), stop_at)
    }

    /// Searches for the next focus inside of `focus`, returning `None` if
    /// `stop_at` is reached or all children are checked before finding a widget
    /// that returns true from `accept_focus`.
    fn next_focus_within(
        &mut self,
        focus: &ManagedWidget,
        start_at: Option<WidgetId>,
        stop_at: WidgetId,
    ) -> Option<ManagedWidget> {
        let child_layouts = focus.child_layouts();
        // TODO visually sort the layouts

        let mut child_layouts = child_layouts.into_iter().peekable();
        if let Some(start_at) = start_at {
            // Skip all children up to `start_at`
            while child_layouts.peek()?.0.id() != start_at {
                child_layouts.next();
            }
            // Skip `start_at`
            child_layouts.next();
        }

        for (child, _layout) in child_layouts {
            // Ensure we haven't cycled completely.
            if stop_at == child.id() {
                break;
            }

            if child
                .lock()
                .as_widget()
                .accept_focus(&mut self.for_other(child.clone()))
            {
                return Some(child);
            } else if let Some(focus) = self.next_focus_within(&child, None, stop_at) {
                return Some(focus);
            }
        }

        None
    }
}

impl<'context, 'window> Deref for EventContext<'context, 'window> {
    type Target = WidgetContext<'context, 'window>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<'context, 'window> DerefMut for EventContext<'context, 'window> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

/// An owned `T` or an exclusive reference to a `T`.
pub enum Exclusive<'a, T> {
    /// An exclusive borrow.
    Borrowed(&'a mut T),
    /// An owned instance.
    Owned(T),
}

impl<T> Deref for Exclusive<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Exclusive::Borrowed(wrapped) => wrapped,
            Exclusive::Owned(wrapped) => wrapped,
        }
    }
}

impl<T> DerefMut for Exclusive<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Exclusive::Borrowed(wrapped) => wrapped,
            Exclusive::Owned(wrapped) => wrapped,
        }
    }
}

/// A context to a function that is rendering a widget.
pub struct GraphicsContext<'context, 'window, 'clip, 'gfx, 'pass> {
    /// The context of the widget being rendered.
    pub widget: WidgetContext<'context, 'window>,
    /// The graphics context clipped and offset to the area of the widget being
    /// rendered. Drawing at 0,0 will draw at the top-left pixel of the laid-out
    /// widget region.
    pub graphics: Exclusive<'context, Graphics<'clip, 'gfx, 'pass>>,
}

impl<'context, 'window, 'clip, 'gfx, 'pass> GraphicsContext<'context, 'window, 'clip, 'gfx, 'pass> {
    /// Returns a new instance that borrows from `self`.
    pub fn borrowed(&mut self) -> GraphicsContext<'_, 'window, 'clip, 'gfx, 'pass> {
        GraphicsContext {
            widget: self.widget.borrowed(),
            graphics: Exclusive::Borrowed(&mut self.graphics),
        }
    }

    /// Returns a new `GraphicsContext` that allows invoking graphics functions
    /// for `widget`.
    pub fn for_other<'child>(
        &'child mut self,
        widget: ManagedWidget,
    ) -> GraphicsContext<'child, 'window, 'child, 'gfx, 'pass> {
        let widget = self.widget.for_other(widget);
        let layout = widget.last_layout().map_or_else(
            || Rect::from(self.graphics.clip_rect().size).into_signed(),
            |rect| rect - self.graphics.region().origin,
        );
        GraphicsContext {
            widget,
            graphics: Exclusive::Owned(self.graphics.clipped_to(layout)),
        }
    }

    /// Returns a new graphics context that renders to the `clip` rectangle.
    pub fn clipped_to(&mut self, clip: Rect<Px>) -> GraphicsContext<'_, 'window, '_, 'gfx, 'pass> {
        GraphicsContext {
            widget: self.widget.borrowed(),
            graphics: Exclusive::Owned(self.graphics.clipped_to(clip)),
        }
    }

    /// Renders the default focus ring for this widget.
    ///
    /// To ensure the correct color is used, include [`HighlightColor`] in the
    /// styles request.
    pub fn draw_focus_ring_using(&mut self, styles: &Styles) {
        let visible_rect = Rect::from(self.graphics.region().size - (Px(1), Px(1)));
        let focus_ring = Shape::stroked_rect(
            visible_rect,
            styles.get_or_default(&HighlightColor),
            StrokeOptions::default(),
        );
        self.graphics
            .draw_shape(&focus_ring, Point::default(), None, None);
    }

    /// Renders the default focus ring for this widget.
    pub fn draw_focus_ring(&mut self) {
        self.draw_focus_ring_using(&self.query_styles(&[&HighlightColor]));
    }

    /// Invokes [`Widget::redraw()`](crate::widget::Widget::redraw) on this
    /// context's widget.
    pub fn redraw(&mut self) {
        self.current_node.clone().lock().as_widget().redraw(self);
    }
}

impl Drop for GraphicsContext<'_, '_, '_, '_, '_> {
    fn drop(&mut self) {
        if matches!(self.widget.pending_state, PendingState::Owned(_)) {
            self.as_event_context().apply_pending_state();
        }
    }
}

impl<'context, 'window, 'clip, 'gfx, 'pass> Deref
    for GraphicsContext<'context, 'window, 'clip, 'gfx, 'pass>
{
    type Target = WidgetContext<'context, 'window>;

    fn deref(&self) -> &Self::Target {
        &self.widget
    }
}

impl<'context, 'window, 'clip, 'gfx, 'pass> DerefMut
    for GraphicsContext<'context, 'window, 'clip, 'gfx, 'pass>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.widget
    }
}

/// A context to a function that is rendering a widget.
pub struct LayoutContext<'context, 'window, 'clip, 'gfx, 'pass> {
    graphics: GraphicsContext<'context, 'window, 'clip, 'gfx, 'pass>,
    persist_layout: bool,
}

impl<'context, 'window, 'clip, 'gfx, 'pass> LayoutContext<'context, 'window, 'clip, 'gfx, 'pass> {
    pub(crate) fn new(
        graphics: &'context mut GraphicsContext<'_, 'window, 'clip, 'gfx, 'pass>,
    ) -> Self {
        Self {
            graphics: graphics.borrowed(),
            persist_layout: true,
        }
    }

    /// Returns a new layout context that does not persist any child layout
    /// operations.
    ///
    /// This type of context is useful for asking widgets to measuree themselves
    /// in hypothetical layout conditions while trying to determine the best
    /// layout for a composite control.
    #[must_use]
    pub fn as_temporary(mut self) -> Self {
        self.persist_layout = false;
        self
    }

    /// Returns a new `LayoutContext` that allows invoking layout functions for
    /// `widget`.
    pub fn for_other<'child, 'widget>(
        &'child mut self,
        widget: ManagedWidget,
    ) -> LayoutContext<'child, 'window, 'child, 'gfx, 'pass>
    where
        'widget: 'child,
    {
        LayoutContext {
            graphics: self.graphics.for_other(widget),
            persist_layout: self.persist_layout,
        }
    }

    /// Invokes [`Widget::layout()`](crate::widget::Widget::layout) on this
    /// context's widget and returns the result.
    pub fn layout(&mut self, available_space: Size<ConstraintLimit>) -> Size<UPx> {
        if self.persist_layout {
            self.graphics.current_node.reset_child_layouts();
        }
        self.graphics
            .current_node
            .clone()
            .lock()
            .as_widget()
            .layout(available_space, self)
    }

    /// Sets the layout for `child` to `layout`.
    ///
    /// `layout` is relative to the current widget's controls.
    pub fn set_child_layout(&mut self, child: &ManagedWidget, layout: Rect<Px>) {
        // TODO verify that `child` belongs to the current node.
        if self.persist_layout {
            child.set_layout(layout);
        }
    }
}

impl<'context, 'window, 'clip, 'gfx, 'pass> Deref
    for LayoutContext<'context, 'window, 'clip, 'gfx, 'pass>
{
    type Target = GraphicsContext<'context, 'window, 'clip, 'gfx, 'pass>;

    fn deref(&self) -> &Self::Target {
        &self.graphics
    }
}

impl<'context, 'window, 'clip, 'gfx, 'pass> DerefMut
    for LayoutContext<'context, 'window, 'clip, 'gfx, 'pass>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.graphics
    }
}

/// Converts from one context to an [`EventContext`].
pub trait AsEventContext<'window> {
    /// Returns this context as an [`EventContext`].
    fn as_event_context(&mut self) -> EventContext<'_, 'window>;

    /// Pushes a new child widget into the widget hierarchy beneathq the
    /// context's widget.
    #[must_use]
    fn push_child(&mut self, child: WidgetInstance) -> ManagedWidget {
        let mut context = self.as_event_context();
        let pushed_widget = context
            .current_node
            .tree
            .push_boxed(child, Some(&context.current_node));
        pushed_widget
            .lock()
            .as_widget()
            .mounted(&mut context.for_other(pushed_widget.clone()));
        pushed_widget
    }

    /// Removes a widget from the hierarchy.
    fn remove_child(&mut self, child: &ManagedWidget) {
        let mut context = self.as_event_context();
        context
            .current_node
            .tree
            .remove_child(child, &context.current_node);
        child
            .lock()
            .as_widget()
            .unmounted(&mut context.for_other(child.clone()));
    }
}

impl<'window> AsEventContext<'window> for EventContext<'_, 'window> {
    fn as_event_context(&mut self) -> EventContext<'_, 'window> {
        EventContext::new(self.widget.borrowed(), self.kludgine)
    }
}

impl<'window> AsEventContext<'window> for GraphicsContext<'_, 'window, '_, '_, '_> {
    fn as_event_context(&mut self) -> EventContext<'_, 'window> {
        EventContext::new(self.widget.borrowed(), &mut self.graphics)
    }
}

/// A context for a widget.
///
/// This type provides access to the widget hierarchy from the perspective of a
/// specific widget.
pub struct WidgetContext<'context, 'window> {
    current_node: ManagedWidget,
    redraw_status: &'context RedrawStatus,
    window: &'context mut RunningWindow<'window>,
    pending_state: PendingState<'context>,
}

impl<'context, 'window> WidgetContext<'context, 'window> {
    pub(crate) fn new(
        current_node: ManagedWidget,
        redraw_status: &'context RedrawStatus,
        window: &'context mut RunningWindow<'window>,
    ) -> Self {
        Self {
            pending_state: PendingState::Owned(PendingWidgetState {
                focus: current_node
                    .tree
                    .focused_widget()
                    .and_then(|id| current_node.tree.widget(id)),
                active: current_node
                    .tree
                    .active_widget()
                    .and_then(|id| current_node.tree.widget(id)),
            }),
            current_node,
            redraw_status,
            window,
        }
    }

    /// Returns a new instance that borrows from `self`.
    pub fn borrowed(&mut self) -> WidgetContext<'_, 'window> {
        WidgetContext {
            current_node: self.current_node.clone(),
            redraw_status: self.redraw_status,
            window: &mut *self.window,
            pending_state: self.pending_state.borrowed(),
        }
    }

    /// Returns a new context representing `widget`.
    pub fn for_other<'child>(
        &'child mut self,
        widget: ManagedWidget,
    ) -> WidgetContext<'child, 'window> {
        WidgetContext {
            current_node: widget,
            redraw_status: self.redraw_status,
            window: &mut *self.window,
            pending_state: self.pending_state.borrowed(),
        }
    }

    pub(crate) fn parent(&self) -> Option<ManagedWidget> {
        self.current_node.parent()
    }

    /// Ensures that this widget will be redrawn when `value` has been updated.
    pub fn redraw_when_changed<T>(&self, value: &Dynamic<T>) {
        value.redraw_when_changed(self.handle());
    }

    /// Returns the last layout of this widget.
    #[must_use]
    pub fn last_layout(&self) -> Option<Rect<Px>> {
        self.current_node.last_layout()
    }

    /// Sets the currently focused widget to this widget.
    ///
    /// Widget events relating to focus changes are deferred until after the all
    /// contexts for the currently firing event are dropped.
    pub fn focus(&mut self) {
        self.pending_state.focus = Some(self.current_node.clone());
    }

    pub(crate) fn clear_focus(&mut self) {
        self.pending_state.focus = None;
    }

    /// Clears focus from this widget, if it is the focused widget.
    ///
    /// Returns true if this function resulted in the focus being changed.
    ///
    /// Widget events relating to focus changes are deferred until after the all
    /// contexts for the currently firing event are dropped.
    pub fn blur(&mut self) -> bool {
        if self.focused() {
            self.clear_focus();
            true
        } else {
            false
        }
    }

    /// Activates this widget, if it is not already active.
    ///
    /// Returns true if this function resulted in the currently active widget
    /// being changed.
    ///
    /// Widget events relating to activation changes are deferred until after
    /// the all contexts for the currently firing event are dropped.
    pub fn activate(&mut self) -> bool {
        if self
            .pending_state
            .active
            .as_ref()
            .map_or(true, |active| active != &self.current_node)
        {
            self.pending_state.active = Some(self.current_node.clone());
            true
        } else {
            false
        }
    }

    /// Deactivates this widget, if it is the currently active widget.
    ///
    /// Returns true if this function resulted in the active widget being
    /// changed.
    ///
    /// Widget events relating to activation changes are deferred until after
    /// the all contexts for the currently firing event are dropped.
    pub fn deactivate(&mut self) -> bool {
        if self.active() {
            self.clear_active();
            true
        } else {
            false
        }
    }

    pub(crate) fn clear_active(&mut self) {
        self.pending_state.active = None;
    }

    /// Returns true if this widget is currently the active widget.
    #[must_use]
    pub fn active(&self) -> bool {
        self.pending_state.active.as_ref() == Some(&self.current_node)
    }

    /// Returns true if this widget is currently hovered, even if the cursor is
    /// over a child widget.
    #[must_use]
    pub fn hovered(&self) -> bool {
        self.current_node.hovered()
    }

    /// Returns true if this widget that is directly beneath the cursor.
    #[must_use]
    pub fn primary_hover(&self) -> bool {
        self.current_node.primary_hover()
    }

    /// Returns true if this widget is currently focused for user input.
    #[must_use]
    pub fn focused(&self) -> bool {
        self.pending_state.focus.as_ref() == Some(&self.current_node)
    }

    /// Returns the widget this context is for.
    #[must_use]
    pub const fn widget(&self) -> &ManagedWidget {
        &self.current_node
    }

    /// Attaches `styles` to the widget hierarchy for this widget.
    ///
    /// Style queries for children will return any values matching this
    /// collection.
    pub fn attach_styles(&self, styles: Styles) {
        self.current_node.attach_styles(styles);
    }

    /// Queries the widget hierarchy for matching style components.
    ///
    /// This function traverses up the widget hierarchy looking for the
    /// components being requested. The resulting styles will contain the values
    /// from the closest matches in the widget hierarchy.
    ///
    /// For style components to be found, they must have previously been
    /// [attached](Self::attach_styles). The [`Style`](crate::widgets::Style)
    /// widget is provided as a convenient way to attach styles into the widget
    /// hierarchy.
    #[must_use]
    pub fn query_styles(&self, query: &[&dyn ComponentDefaultvalue]) -> Styles {
        self.current_node
            .tree
            .query_styles(&self.current_node, query)
    }

    /// Queries the widget hierarchy for a single style component.
    ///
    /// This function traverses up the widget hierarchy looking for the
    /// component being requested. If a matching component is found, it will be
    /// returned. Otherwise, the default value will be returned.

    #[must_use]
    pub fn query_style<Component: ComponentDefinition>(
        &self,
        query: &Component,
    ) -> Component::ComponentType {
        self.current_node
            .tree
            .query_style(&self.current_node, query)
    }

    pub(crate) fn handle(&self) -> WindowHandle {
        WindowHandle {
            kludgine: self.window.handle(),
            redraw_status: self.redraw_status.clone(),
        }
    }
}

pub(crate) struct WindowHandle {
    kludgine: kludgine::app::WindowHandle<sealed::WindowCommand>,
    redraw_status: RedrawStatus,
}

impl WindowHandle {
    pub fn redraw(&self) {
        if self.redraw_status.should_send_refresh() {
            let _result = self.kludgine.send(sealed::WindowCommand::Redraw);
        }
    }
}

impl dyn AsEventContext<'_> {}

impl Drop for EventContext<'_, '_> {
    fn drop(&mut self) {
        if matches!(self.widget.pending_state, PendingState::Owned(_)) {
            self.apply_pending_state();
        }
    }
}

impl<'window> Deref for WidgetContext<'_, 'window> {
    type Target = RunningWindow<'window>;

    fn deref(&self) -> &Self::Target {
        self.window
    }
}
impl<'window> DerefMut for WidgetContext<'_, 'window> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.window
    }
}

enum PendingState<'a> {
    Borrowed(&'a mut PendingWidgetState),
    Owned(PendingWidgetState),
}

#[derive(Default)]
struct PendingWidgetState {
    focus: Option<ManagedWidget>,
    active: Option<ManagedWidget>,
}

impl PendingState<'_> {
    pub fn borrowed(&mut self) -> PendingState<'_> {
        PendingState::Borrowed(self)
    }
}

impl Deref for PendingState<'_> {
    type Target = PendingWidgetState;

    fn deref(&self) -> &Self::Target {
        match self {
            PendingState::Borrowed(state) => state,
            PendingState::Owned(state) => state,
        }
    }
}

impl DerefMut for PendingState<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            PendingState::Borrowed(state) => state,
            PendingState::Owned(state) => state,
        }
    }
}

#[derive(Default, Clone)]
pub(crate) struct RedrawStatus {
    refresh_sent: Arc<AtomicBool>,
}

impl RedrawStatus {
    pub fn should_send_refresh(&self) -> bool {
        self.refresh_sent
            .compare_exchange(false, true, Ordering::Release, Ordering::Acquire)
            .is_ok()
    }

    pub fn refresh_received(&self) {
        self.refresh_sent.store(false, Ordering::Release);
    }
}
