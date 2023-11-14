//! Types that provide access to the Gooey runtime.
use std::borrow::Cow;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use kempt::Set;
use kludgine::app::winit::event::{
    DeviceId, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::figures::units::{Lp, Px, UPx};
use kludgine::figures::{IntoSigned, Point, Rect, ScreenScale, Size};
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::{Color, Kludgine};

use crate::graphics::Graphics;
use crate::styles::components::{HighlightColor, LayoutOrder, WidgetBackground};
use crate::styles::{ComponentDefinition, Styles, Theme, ThemePair};
use crate::utils::IgnorePoison;
use crate::value::{Dynamic, IntoValue, Value};
use crate::widget::{EventHandling, ManagedWidget, WidgetId, WidgetInstance, WidgetRef};
use crate::window::sealed::WindowCommand;
use crate::window::{RunningWindow, ThemeMode};
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
    pub fn for_other<'child, Widget>(
        &'child mut self,
        widget: &Widget,
    ) -> <Widget::Managed as MapManagedWidget<EventContext<'child, 'window>>>::Result
    where
        Widget: ManageWidget,
        Widget::Managed: MapManagedWidget<EventContext<'child, 'window>>,
    {
        widget
            .manage(self)
            .map(|managed| EventContext::new(self.widget.for_other(&managed), self.kludgine))
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
            let mut context = self.for_other(&unhovered);
            unhovered.lock().as_widget().unhover(&mut context);
        }
        for hover in changes.hovered {
            let mut context = self.for_other(&hover);
            hover.lock().as_widget().hover(location, &mut context);
        }
    }

    pub(crate) fn clear_hover(&mut self) {
        let changes = self.current_node.tree.hover(None);
        assert!(changes.hovered.is_empty());

        for old_hover in changes.unhovered {
            let mut old_hover_context = self.for_other(&old_hover);
            old_hover.lock().as_widget().unhover(&mut old_hover_context);
        }
    }

    pub(crate) fn apply_pending_state(&mut self) {
        const MAX_ITERS: u8 = 100;
        // These two blocks apply active/focus in a loop to pick up the event
        // where during the process of calling deactivate/blur or activate/focus
        // the active/focus widget is changed again. This can lead to infinite
        // loops, which is a programmer error. However, rather than block
        // forever, we log a message that this is happening and break.

        let mut activation_changes = 0;
        while activation_changes < MAX_ITERS {
            let active = self.pending_state.active.clone();
            if self.current_node.tree.active_widget() == active.as_ref().map(|w| w.node_id) {
                break;
            }
            activation_changes += 1;

            let new = match self.current_node.tree.activate(active.as_ref()) {
                Ok(old) => {
                    if let Some(old) = old {
                        let mut old_context = self.for_other(&old);
                        old.lock().as_widget().deactivate(&mut old_context);
                    }
                    true
                }
                Err(()) => false,
            };
            if new {
                if let Some(active) = self.pending_state.active.clone() {
                    active
                        .lock()
                        .as_widget()
                        .activate(&mut self.for_other(&active));
                }
                self.pending_state.active = active;
            } else {
                break;
            }
        }

        if activation_changes == MAX_ITERS {
            tracing::error!(
                "activation change force stopped after {activation_changes} sequential changes"
            );
        }

        let mut focus_changes = 0;
        while focus_changes < MAX_ITERS {
            let focus = self.pending_state.focus.clone();
            if self.current_node.tree.focused_widget() == focus.as_ref().map(|w| w.node_id) {
                break;
            }
            focus_changes += 1;

            self.pending_state.focus = focus.and_then(|mut focus| loop {
                if focus
                    .lock()
                    .as_widget()
                    .accept_focus(&mut self.for_other(&focus))
                {
                    break Some(focus);
                } else if let Some(next_focus) =
                    focus.explicit_focus_target(self.pending_state.focus_is_advancing)
                {
                    focus = next_focus;
                } else {
                    break self.next_focus_after(focus, self.pending_state.focus_is_advancing);
                }
            });
            let new = match self
                .current_node
                .tree
                .focus(self.pending_state.focus.as_ref())
            {
                Ok(old) => {
                    if let Some(old) = old {
                        let mut old_context = self.for_other(&old);
                        old.lock().as_widget().blur(&mut old_context);
                    }
                    true
                }
                Err(()) => false,
            };
            if new {
                if let Some(focus) = self.pending_state.focus.clone() {
                    focus.lock().as_widget().focus(&mut self.for_other(&focus));
                }
            } else {
                break;
            }
        }

        if focus_changes == MAX_ITERS {
            tracing::error!("focus change force stopped after {focus_changes} sequential changes");
        }
    }

    fn next_focus_after(
        &mut self,
        mut focus: ManagedWidget,
        advance: bool,
    ) -> Option<ManagedWidget> {
        // First, look within the current focus for any focusable children.
        let stop_at = focus.id();
        if let Some(focus) = self.next_focus_within(&focus, None, stop_at, advance) {
            return Some(focus);
        }

        // Now, look for the next widget in each hierarchy
        let root = loop {
            if let Some(focus) = self.next_focus_sibling(&focus, stop_at, advance) {
                return Some(focus);
            }
            let Some(parent) = focus.parent() else {
                break focus;
            };
            focus = parent;
        };

        // We've exhausted a forward scan, we can now start searching the final
        // parent, which is the root.
        self.next_focus_within(&root, None, stop_at, advance)
    }

    fn next_focus_sibling(
        &mut self,
        focus: &ManagedWidget,
        stop_at: WidgetId,
        advance: bool,
    ) -> Option<ManagedWidget> {
        self.next_focus_within(&focus.parent()?, Some(focus.id()), stop_at, advance)
    }

    /// Searches for the next focus inside of `focus`, returning `None` if
    /// `stop_at` is reached or all children are checked before finding a widget
    /// that returns true from `accept_focus`.
    fn next_focus_within(
        &mut self,
        focus: &ManagedWidget,
        start_at: Option<WidgetId>,
        stop_at: WidgetId,
        advance: bool,
    ) -> Option<ManagedWidget> {
        let mut visual_order = self.get(&LayoutOrder);
        if !advance {
            visual_order = visual_order.rev();
        }
        let mut children = focus
            .visually_ordered_children(visual_order)
            .into_iter()
            .peekable();
        if let Some(start_at) = start_at {
            // Skip all children up to `start_at`
            while children.peek()?.id() != start_at {
                children.next();
            }
            // Skip `start_at`
            children.next();
        }

        for child in children {
            // Ensure we haven't cycled completely.
            if stop_at == child.id() {
                break;
            }

            if child
                .lock()
                .as_widget()
                .accept_focus(&mut self.for_other(&child))
            {
                return Some(child);
            } else if let Some(next_focus) = self.widget().explicit_focus_target(advance) {
                return Some(next_focus);
            } else if let Some(focus) = self.next_focus_within(&child, None, stop_at, advance) {
                return Some(focus);
            }
        }

        None
    }

    /// Advances the focus to the next widget after this widget in the
    /// configured focus order.
    ///
    /// To focus in the reverse order, use [`EventContext::return_focus()`].
    pub fn advance_focus(&mut self) {
        self.move_focus(true);
    }

    /// Returns the focus to the previous widget before this widget in the
    /// configured fous order.
    ///
    /// To focus in the forward order, use [`EventContext::advance_focus()`].
    pub fn return_focus(&mut self) {
        self.move_focus(false);
    }

    fn move_focus(&mut self, advance: bool) {
        if let Some(explicit_next_focus) = self.current_node.explicit_focus_target(advance) {
            self.for_other(&explicit_next_focus).focus();
        } else {
            self.pending_state.focus = self.next_focus_after(self.current_node.clone(), advance);
        }
        // It is important to set focus-is_advancing after `focus()` because it
        // sets it to `true` explicitly.
        self.pending_state.focus_is_advancing = advance;
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
    pub gfx: Exclusive<'context, Graphics<'clip, 'gfx, 'pass>>,
}

impl<'context, 'window, 'clip, 'gfx, 'pass> GraphicsContext<'context, 'window, 'clip, 'gfx, 'pass> {
    /// Returns a new instance that borrows from `self`.
    pub fn borrowed(&mut self) -> GraphicsContext<'_, 'window, 'clip, 'gfx, 'pass> {
        GraphicsContext {
            widget: self.widget.borrowed(),
            gfx: Exclusive::Borrowed(&mut self.gfx),
        }
    }

    /// Returns a new `GraphicsContext` that allows invoking graphics functions
    /// for `widget`.
    pub fn for_other<'child, Widget>(
        &'child mut self,
        widget: &Widget,
    ) -> <Widget::Managed as MapManagedWidget<
        GraphicsContext<'child, 'window, 'child, 'gfx, 'pass>,
    >>::Result
    where
        Widget: ManageWidget,
        Widget::Managed: MapManagedWidget<GraphicsContext<'child, 'window, 'child, 'gfx, 'pass>>,
    {
        widget.manage(self).map(|widget| {
            let widget = self.widget.for_other(&widget);
            let layout = widget.last_layout().map_or_else(
                || Rect::from(self.gfx.clip_rect().size).into_signed(),
                |rect| rect - self.gfx.region().origin,
            );
            GraphicsContext {
                widget,
                gfx: Exclusive::Owned(self.gfx.clipped_to(layout)),
            }
        })
    }

    /// Returns a new graphics context that renders to the `clip` rectangle.
    pub fn clipped_to(&mut self, clip: Rect<Px>) -> GraphicsContext<'_, 'window, '_, 'gfx, 'pass> {
        GraphicsContext {
            widget: self.widget.borrowed(),
            gfx: Exclusive::Owned(self.gfx.clipped_to(clip)),
        }
    }

    /// Strokes an outline around this widget's contents.
    pub fn stroke_outline<Unit>(&mut self, color: Color, options: StrokeOptions<Unit>)
    where
        Unit: ScreenScale<Px = Px, Lp = Lp>,
    {
        let visible_rect = Rect::from(self.gfx.region().size - (Px(1), Px(1)));
        let focus_ring =
            Shape::stroked_rect(visible_rect, color, options.into_px(self.gfx.scale()));
        self.gfx
            .draw_shape(&focus_ring, Point::default(), None, None);
    }

    /// Renders the default focus ring for this widget.
    pub fn draw_focus_ring(&mut self) {
        // If this is the root widget, don't draw a focus ring. It's redundant.
        if !self.current_node.has_parent() {
            return;
        }

        let color = self.get(&HighlightColor);
        self.stroke_outline::<Lp>(color, StrokeOptions::lp_wide(Lp::points(2)));
    }

    /// Invokes [`Widget::redraw()`](crate::widget::Widget::redraw) on this
    /// context's widget.
    ///
    /// # Panics
    ///
    /// This function panics if the widget being drawn has no layout set (via
    /// [`LayoutContext::set_child_layout()`]).
    pub fn redraw(&mut self) {
        assert!(
            self.last_layout().is_some(),
            "redraw called without set_widget_layout"
        );

        let background = self.get(&WidgetBackground);
        self.gfx.fill(background);

        self.current_node
            .tree
            .note_widget_rendered(self.current_node.node_id);
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
    /// The graphics context that this layout operation is being performed
    /// within.
    pub graphics: GraphicsContext<'context, 'window, 'clip, 'gfx, 'pass>,
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
    pub fn for_other<'child, Widget>(
        &'child mut self,
        widget: &Widget,
    ) -> <Widget::Managed as MapManagedWidget<LayoutContext<'child, 'window, 'child, 'gfx, 'pass>>>::Result
    where
        Widget: ManageWidget,
        Widget::Managed: MapManagedWidget<LayoutContext<'child, 'window, 'child, 'gfx, 'pass>>,
    {
        widget.manage(self).map(|widget| LayoutContext {
            graphics: self.graphics.for_other(&widget),
            persist_layout: self.persist_layout,
        })
    }

    /// Invokes [`Widget::layout()`](crate::widget::Widget::layout) on this
    /// context's widget and returns the result.
    pub fn layout(&mut self, available_space: Size<ConstraintLimit>) -> Size<UPx> {
        if self.persist_layout {
            if let Some(cached) = self.graphics.current_node.begin_layout(available_space) {
                return cached;
            }
        }
        let result = self
            .graphics
            .current_node
            .clone()
            .lock()
            .as_widget()
            .layout(available_space, self);
        if self.persist_layout {
            self.graphics
                .current_node
                .persist_layout(available_space, result);
        }
        result
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
            .mounted(&mut context.for_other(&pushed_widget));
        pushed_widget
    }

    /// Removes a widget from the hierarchy.
    fn remove_child(&mut self, child: &ManagedWidget) {
        let mut context = self.as_event_context();
        child
            .lock()
            .as_widget()
            .unmounted(&mut context.for_other(child));
        context
            .current_node
            .tree
            .remove_child(child, &context.current_node);
    }
}

impl<'window> AsEventContext<'window> for EventContext<'_, 'window> {
    fn as_event_context(&mut self) -> EventContext<'_, 'window> {
        EventContext::new(self.widget.borrowed(), self.kludgine)
    }
}

impl<'window> AsEventContext<'window> for GraphicsContext<'_, 'window, '_, '_, '_> {
    fn as_event_context(&mut self) -> EventContext<'_, 'window> {
        EventContext::new(self.widget.borrowed(), &mut self.gfx)
    }
}

/// A context for a widget.
///
/// This type provides access to the widget hierarchy from the perspective of a
/// specific widget.
pub struct WidgetContext<'context, 'window> {
    current_node: ManagedWidget,
    redraw_status: &'context InvalidationStatus,
    window: &'context mut RunningWindow<'window>,
    theme: Cow<'context, ThemePair>,
    pending_state: PendingState<'context>,
    theme_mode: ThemeMode,
    effective_styles: Styles,
}

impl<'context, 'window> WidgetContext<'context, 'window> {
    pub(crate) fn new(
        current_node: ManagedWidget,
        redraw_status: &'context InvalidationStatus,
        theme: &'context ThemePair,
        window: &'context mut RunningWindow<'window>,
        theme_mode: ThemeMode,
    ) -> Self {
        Self {
            pending_state: PendingState::Owned(PendingWidgetState {
                focus: current_node
                    .tree
                    .focused_widget()
                    .and_then(|id| current_node.tree.widget_from_node(id)),
                active: current_node
                    .tree
                    .active_widget()
                    .and_then(|id| current_node.tree.widget_from_node(id)),
                focus_is_advancing: false,
            }),
            effective_styles: current_node.effective_styles(),
            current_node,
            redraw_status,
            theme: Cow::Borrowed(theme),
            theme_mode,
            window,
        }
    }

    /// Returns a new instance that borrows from `self`.
    pub fn borrowed(&mut self) -> WidgetContext<'_, 'window> {
        WidgetContext {
            current_node: self.current_node.clone(),
            redraw_status: self.redraw_status,
            window: &mut *self.window,
            theme: Cow::Borrowed(self.theme.as_ref()),
            pending_state: self.pending_state.borrowed(),
            theme_mode: self.theme_mode,
            effective_styles: self.effective_styles.clone(),
        }
    }

    /// Returns a new context representing `widget`.
    pub fn for_other<'child, Widget>(
        &'child mut self,
        widget: &Widget,
    ) -> <Widget::Managed as MapManagedWidget<WidgetContext<'child, 'window>>>::Result
    where
        Widget: ManageWidget,
        Widget::Managed: MapManagedWidget<WidgetContext<'child, 'window>>,
    {
        widget.manage(self).map(|current_node| {
            let (effective_styles, theme, theme_mode) = current_node.overidden_theme();
            let theme = if let Some(theme) = theme {
                Cow::Owned(theme.get_tracked(self))
            } else {
                Cow::Borrowed(self.theme.as_ref())
            };
            let theme_mode = if let Some(mode) = theme_mode {
                mode.get_tracked(self)
            } else {
                self.theme_mode
            };
            WidgetContext {
                effective_styles,
                current_node,
                redraw_status: self.redraw_status,
                window: &mut *self.window,
                theme,
                pending_state: self.pending_state.borrowed(),
                theme_mode,
            }
        })
    }

    pub(crate) fn parent(&self) -> Option<ManagedWidget> {
        self.current_node.parent()
    }

    /// Ensures that this widget will be redrawn when `value` has been updated.
    pub fn redraw_when_changed<T>(&self, value: &Dynamic<T>) {
        value.redraw_when_changed(self.handle());
    }

    /// Ensures that this widget will be redrawn when `value` has been updated.
    pub fn invalidate_when_changed<T>(&self, value: &Dynamic<T>) {
        value.invalidate_when_changed(self.handle(), self.current_node.id());
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
        self.pending_state.focus_is_advancing = true;
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

    /// Returns true if this widget is the target to activate when the user
    /// triggers a default action.
    ///
    /// See
    /// [`MakeWidget::into_default()`](crate::widget::MakeWidget::into_default)
    /// for more information.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.current_node.tree.default_widget() == Some(self.current_node.node_id)
    }

    /// Returns true if this widget is the target to activate when the user
    /// triggers an escape action.
    ///
    /// See
    /// [`MakeWidget::into_escape()`](crate::widget::MakeWidget::into_escape)
    /// for more information.
    #[must_use]
    pub fn is_escape(&self) -> bool {
        self.current_node.tree.escape_widget() == Some(self.current_node.node_id)
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
    pub fn attach_styles(&self, styles: impl IntoValue<Styles>) {
        self.current_node.attach_styles(styles.into_value());
    }

    /// Attaches `theme` to the widget hierarchy for this widget.
    ///
    /// All children nodes will access this theme in their contexts.
    pub fn attach_theme(&self, theme: Value<ThemePair>) {
        self.current_node.attach_theme(theme);
    }

    /// Attaches `theme_mode` to the widget hierarchy for this widget.
    ///
    /// All children nodes will use this theme mode.
    pub fn attach_theme_mode(&self, theme_mode: Value<ThemeMode>) {
        self.current_node.attach_theme_mode(theme_mode);
    }

    /// Queries the widget hierarchy for a single style component.
    ///
    /// This function traverses up the widget hierarchy looking for the
    /// component being requested. If a matching component is found, it will be
    /// returned. Otherwise, the default value will be returned.
    #[must_use]
    pub fn get<Component: ComponentDefinition>(
        &self,
        query: &Component,
    ) -> Component::ComponentType {
        self.effective_styles.get(query, self)
    }

    pub(crate) fn handle(&self) -> WindowHandle {
        WindowHandle {
            kludgine: self.window.handle(),
            redraw_status: self.redraw_status.clone(),
        }
    }

    /// Returns the window containing this widget.
    #[must_use]
    pub fn window(&self) -> &RunningWindow<'window> {
        self.window
    }

    /// Returns an exclusive reference to the window containing this widget.
    #[must_use]
    pub fn window_mut(&mut self) -> &mut RunningWindow<'window> {
        self.window
    }

    /// Returns the theme pair for the window.
    #[must_use]
    pub fn theme_pair(&self) -> &ThemePair {
        self.theme.as_ref()
    }

    /// Returns the current theme in either light or dark mode.
    #[must_use]
    pub fn theme(&self) -> &Theme {
        match self.theme_mode {
            ThemeMode::Light => &self.theme.light,
            ThemeMode::Dark => &self.theme.dark,
        }
    }

    /// Returns the opposite theme of [`Self::theme()`].
    #[must_use]
    pub fn inverse_theme(&self) -> &Theme {
        match self.theme_mode {
            ThemeMode::Light => &self.theme.dark,
            ThemeMode::Dark => &self.theme.light,
        }
    }
}

pub(crate) struct WindowHandle {
    kludgine: kludgine::app::WindowHandle<WindowCommand>,
    redraw_status: InvalidationStatus,
}

impl Eq for WindowHandle {}

impl PartialEq for WindowHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(
            &self.redraw_status.invalidated,
            &other.redraw_status.invalidated,
        )
    }
}

impl Hash for WindowHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.redraw_status.invalidated).hash(state);
    }
}

impl WindowHandle {
    pub fn redraw(&self) {
        if self.redraw_status.should_send_refresh() {
            let _result = self.kludgine.send(WindowCommand::Redraw);
        }
    }

    pub fn invalidate(&self, widget: WidgetId) {
        if self.redraw_status.invalidate(widget) {
            self.redraw();
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
    focus_is_advancing: bool,
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
pub(crate) struct InvalidationStatus {
    refresh_sent: Arc<AtomicBool>,
    invalidated: Arc<Mutex<Set<WidgetId>>>,
}

impl InvalidationStatus {
    pub fn should_send_refresh(&self) -> bool {
        self.refresh_sent
            .compare_exchange(false, true, Ordering::Release, Ordering::Acquire)
            .is_ok()
    }

    pub fn refresh_received(&self) {
        self.refresh_sent.store(false, Ordering::Release);
    }

    pub fn invalidate(&self, widget: WidgetId) -> bool {
        let mut invalidated = self.invalidated.lock().ignore_poison();
        invalidated.insert(widget)
    }

    pub fn invalidations(&self) -> MutexGuard<'_, Set<WidgetId>> {
        self.invalidated.lock().ignore_poison()
    }
}

/// A type chat can convert to a [`ManagedWidget`] through a [`WidgetContext`].
pub trait ManageWidget {
    /// The managed type, which can be `Option<ManagedWidget>` or
    /// `ManagedWidget`.
    type Managed: MapManagedWidget<ManagedWidget>;

    /// Resolve `self` into a [`ManagedWidget`].
    fn manage(&self, context: &WidgetContext<'_, '_>) -> Self::Managed;
}

impl ManageWidget for WidgetInstance {
    type Managed = Option<ManagedWidget>;

    fn manage(&self, context: &WidgetContext<'_, '_>) -> Self::Managed {
        context.current_node.tree.widget(self.id())
    }
}

impl ManageWidget for WidgetRef {
    type Managed = Option<ManagedWidget>;

    fn manage(&self, context: &WidgetContext<'_, '_>) -> Self::Managed {
        match self {
            WidgetRef::Unmounted(instance) => context.current_node.tree.widget(instance.id()),
            WidgetRef::Mounted(instance) => Some(instance.clone()),
        }
    }
}

impl ManageWidget for ManagedWidget {
    type Managed = Self;

    fn manage(&self, _context: &WidgetContext<'_, '_>) -> Self::Managed {
        self.clone()
    }
}

/// A type that can produce another type when provided a [`ManagedWidget`].
pub trait MapManagedWidget<T> {
    /// The result of the mapping operation.
    type Result;

    /// Call `map` with a [`ManagedWidget`].
    fn map(self, map: impl FnOnce(ManagedWidget) -> T) -> Self::Result;
}

impl<T> MapManagedWidget<T> for Option<ManagedWidget> {
    type Result = Option<T>;

    fn map(self, map: impl FnOnce(ManagedWidget) -> T) -> Self::Result {
        self.map(map)
    }
}

impl<T> MapManagedWidget<T> for ManagedWidget {
    type Result = T;

    fn map(self, map: impl FnOnce(ManagedWidget) -> T) -> Self::Result {
        map(self)
    }
}
