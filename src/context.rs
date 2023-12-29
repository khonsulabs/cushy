//! Types that provide access to the Cushy runtime.
use std::borrow::Cow;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use figures::units::{Lp, Px, UPx};
use figures::{IntoSigned, Point, Px2D, Rect, Round, ScreenScale, Size, Zero};
use kempt::Set;
use kludgine::app::winit::event::{
    DeviceId, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::app::winit::window::CursorIcon;
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::{Color, Kludgine, KludgineId};

use crate::animation::ZeroToOne;
use crate::graphics::Graphics;
use crate::styles::components::{
    CornerRadius, FontFamily, FontStyle, FontWeight, HighlightColor, LayoutOrder, LineHeight,
    Opacity, TextSize, WidgetBackground,
};
use crate::styles::{ComponentDefinition, Styles, Theme, ThemePair};
use crate::tree::Tree;
use crate::utils::IgnorePoison;
use crate::value::{IntoValue, Value};
use crate::widget::{EventHandling, MountedWidget, RootBehavior, WidgetId, WidgetInstance};
use crate::window::{CursorState, RunningWindow, ThemeMode};
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
    const MAX_PENDING_CHANGE_CYCLES: u8 = 100;

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
        let changes = self.tree.hover(Some(&self.current_node));
        for unhovered in changes.unhovered {
            let mut context = self.for_other(&unhovered);
            unhovered.lock().as_widget().unhover(&mut context);
        }

        let mut cursor = None;
        for hover in changes.hovered.into_iter().rev() {
            let mut context = self.for_other(&hover);
            let Some(last_layout) = context.last_layout() else {
                continue;
            };
            let widget_cursor = hover
                .lock()
                .as_widget()
                .hover(location - last_layout.origin, &mut context);

            if cursor.is_none() {
                cursor = widget_cursor;
            }
        }
        self.winit().set_cursor_icon(cursor.unwrap_or_default());
    }

    pub(crate) fn clear_hover(&mut self) {
        let changes = self.tree.hover(None);
        assert!(changes.hovered.is_empty());

        for old_hover in changes.unhovered {
            let mut old_hover_context = self.for_other(&old_hover);
            old_hover.lock().as_widget().unhover(&mut old_hover_context);
        }

        self.winit().set_cursor_icon(CursorIcon::Default);
    }

    fn apply_pending_activation(&mut self) {
        let mut activation_changes = 0;
        while activation_changes < Self::MAX_PENDING_CHANGE_CYCLES {
            let active = self.pending_state.active.and_then(|w| self.tree.widget(w));
            if self.tree.active_widget() == active.as_ref().map(|w| w.node_id) {
                break;
            }
            activation_changes += 1;

            let new = match self.tree.activate(active.as_ref()) {
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
                let active = self.pending_state.active.and_then(|w| self.tree.widget(w));
                if let Some(active) = &active {
                    active
                        .lock()
                        .as_widget()
                        .activate(&mut self.for_other(active));
                }
            } else {
                break;
            }
        }

        if activation_changes == Self::MAX_PENDING_CHANGE_CYCLES {
            tracing::error!(
                "activation change force stopped after {activation_changes} sequential changes"
            );
        }
    }

    fn apply_pending_focus(&mut self) {
        let mut focus_changes = 0;
        while focus_changes < Self::MAX_PENDING_CHANGE_CYCLES {
            let focus = self.pending_state.focus.and_then(|w| self.tree.widget(w));
            if self.tree.focused_widget() == focus.as_ref().map(|w| w.node_id) {
                break;
            }
            focus_changes += 1;

            self.pending_state.focus = focus.and_then(|mut focus| loop {
                let mut focus_context = self.for_other(&focus);
                let accept_focus = focus.lock().as_widget().accept_focus(&mut focus_context);
                drop(focus_context);

                if accept_focus {
                    break Some(focus.id());
                } else if let Some(next_focus) =
                    focus.explicit_focus_target(self.pending_state.focus_is_advancing)
                {
                    focus = next_focus;
                } else {
                    break self.next_focus_after(focus, self.pending_state.focus_is_advancing);
                }
            });
            let new = match self.tree.focus(self.pending_state.focus) {
                Ok(old) => {
                    if let Some(old_widget) = old {
                        let mut old_context = self.for_other(&old_widget);
                        let mut old = old_widget.lock();
                        if old.as_widget().allow_blur(&mut old_context) {
                            old.as_widget().blur(&mut old_context);
                        } else {
                            // This widget is rejecting the focus change.
                            drop(old_context);
                            let _result = self.tree.focus(Some(old_widget.id()));
                            self.pending_state.focus = Some(old_widget.id());
                            break;
                        }
                    }
                    true
                }
                Err(()) => false,
            };
            if new {
                if let Some(focus) = self.pending_state.focus.and_then(|w| self.tree.widget(w)) {
                    focus.lock().as_widget().focus(&mut self.for_other(&focus));
                }
            } else {
                break;
            }
        }

        if focus_changes == Self::MAX_PENDING_CHANGE_CYCLES {
            tracing::error!("focus change force stopped after {focus_changes} sequential changes");
        }
    }

    pub(crate) fn apply_pending_state(&mut self) {
        // These two blocks apply active/focus in a loop to pick up the event
        // where during the process of calling deactivate/blur or activate/focus
        // the active/focus widget is changed again. This can lead to infinite
        // loops, which is a programmer error. However, rather than block
        // forever, we log a message that this is happening and break.

        self.apply_pending_activation();

        self.apply_pending_focus();

        // Check that our hover widget still exists. If not, we should try to find a new one.
        if let Some(hover) = self.tree.hovered_widget() {
            if self.tree.widget_from_node(hover).is_none() {
                self.update_hovered_widget();
            }
        }
    }

    pub(crate) fn update_hovered_widget(&mut self) {
        self.cursor.widget = None;
        if let Some(location) = self.cursor.location {
            for widget in self.tree.widgets_under_point(location) {
                let mut widget_context = self.for_other(&widget);
                let Some(widget_layout) = widget_context.last_layout() else {
                    continue;
                };
                let relative = location - widget_layout.origin;

                if widget_context.hit_test(relative) {
                    widget_context.hover(location);
                    drop(widget_context);
                    self.cursor.widget = Some(widget.id());
                    break;
                }
            }
        }

        if self.cursor.widget.is_none() {
            self.clear_hover();
        }
    }

    fn next_focus_after(&mut self, mut focus: MountedWidget, advance: bool) -> Option<WidgetId> {
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
        let mut child_context = self.for_other(&root);
        let accept_focus = root.lock().as_widget().accept_focus(&mut child_context);
        drop(child_context);
        if accept_focus {
            Some(root.id())
        } else if stop_at == root.id() {
            // We cycled completely.
            None
        } else if let Some(next_focus) = self.widget().explicit_focus_target(advance) {
            Some(next_focus.id())
        } else {
            self.next_focus_within(&root, None, stop_at, advance)
        }
    }

    fn next_focus_sibling(
        &mut self,
        focus: &MountedWidget,
        stop_at: WidgetId,
        advance: bool,
    ) -> Option<WidgetId> {
        self.next_focus_within(&focus.parent()?, Some(focus.id()), stop_at, advance)
    }

    /// Searches for the next focus inside of `focus`, returning `None` if
    /// `stop_at` is reached or all children are checked before finding a widget
    /// that returns true from `accept_focus`.
    fn next_focus_within(
        &mut self,
        focus: &MountedWidget,
        start_at: Option<WidgetId>,
        stop_at: WidgetId,
        advance: bool,
    ) -> Option<WidgetId> {
        let last_layout = self.current_node.last_layout()?;
        if last_layout.size.width <= 0 || last_layout.size.height <= 0 {
            return None;
        }

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
            let mut child_context = self.for_other(&child);
            let accept_focus = child.lock().as_widget().accept_focus(&mut child_context);
            drop(child_context);
            if accept_focus {
                return Some(child.id());
            } else if stop_at == child.id() {
                // We cycled completely, and the original widget didn't accept
                // focus.
                return None;
            } else if let Some(next_focus) = self.widget().explicit_focus_target(advance) {
                return Some(next_focus.id());
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
        let node = self.current_node.clone();
        let mut direction = self.get(&LayoutOrder);
        if !advance {
            direction = direction.rev();
        }
        if node
            .lock()
            .as_widget()
            .advance_focus(direction, self)
            .is_break()
        {
            return;
        }

        if let Some(explicit_next_focus) = self.current_node.explicit_focus_target(advance) {
            self.for_other(&explicit_next_focus).focus();
        } else {
            self.pending_state.focus = self.next_focus_after(self.current_node.clone(), advance);
        }
        // It is important to set focus-is_advancing after `focus()` because it
        // sets it to `true` explicitly.
        self.pending_state.focus_is_advancing = advance;
    }

    /// Invokes
    /// [`Widget::root_behavior()`](crate::widget::Widget::root_behavior) on
    /// this context's widget and returns the result.
    pub fn root_behavior(&mut self) -> Option<(RootBehavior, WidgetInstance)> {
        self.current_node
            .clone()
            .lock()
            .as_widget()
            .root_behavior(self)
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
        let opacity = self.get(&Opacity);
        widget.manage(self).map(|widget| {
            let widget = self.widget.for_other(&widget);
            let layout = widget.last_layout().map_or_else(
                || Rect::from(self.gfx.clip_rect().size).into_signed(),
                |rect| rect - self.gfx.region().origin,
            );
            let mut gfx = self.gfx.clipped_to(layout);
            gfx.opacity *= opacity;
            GraphicsContext {
                widget,
                gfx: Exclusive::Owned(gfx),
            }
        })
    }

    /// Updates `self` to have `opacity`.
    ///
    /// This setting will be mixed with the current opacity value.
    pub fn apply_opacity(&mut self, opacity: impl Into<ZeroToOne>) {
        self.gfx.opacity *= opacity.into();
    }

    /// Returns a new graphics context that renders to the `clip` rectangle.
    pub fn clipped_to(&mut self, clip: Rect<Px>) -> GraphicsContext<'_, 'window, '_, 'gfx, 'pass> {
        GraphicsContext {
            widget: self.widget.borrowed(),
            gfx: Exclusive::Owned(self.gfx.clipped_to(clip)),
        }
    }

    /// Fills the background of this widget with `color`, honoring the current
    /// [`CornerRadius`] setting.
    ///
    /// If the alpha channel of `color` is 0, this function does nothing.
    pub fn fill(&mut self, color: Color) {
        if color.alpha() > 0 {
            let visible_rect = Rect::from(self.gfx.region().size - Size::px(1, 1));

            let radii = self.get(&CornerRadius);
            let radii = radii.map(|r| r.into_px(self.gfx.scale()));

            let focus_ring = if radii.is_zero() {
                Shape::filled_rect(visible_rect, color)
            } else {
                Shape::filled_round_rect(visible_rect, radii, color)
            };
            self.gfx.draw_shape(&focus_ring);
        }
    }

    /// Strokes an outline around this widget's contents.
    pub fn stroke_outline<Unit>(&mut self, color: Color, options: StrokeOptions<Unit>)
    where
        Unit: ScreenScale<Px = Px, Lp = Lp, UPx = UPx> + Zero,
    {
        if color.alpha() > 0 {
            let options = options.colored(color).into_px(self.gfx.scale());
            let visible_rect = Rect::new(
                Point::squared(options.line_width / 2),
                self.gfx.region().size - Point::squared(options.line_width),
            );

            let radii = self.get(&CornerRadius);
            let radii = radii.map(|r| r.into_px(self.gfx.scale()));

            let focus_ring = if radii.is_zero() {
                Shape::stroked_rect(visible_rect, options.into_px(self.gfx.scale()))
            } else {
                Shape::stroked_round_rect(visible_rect, radii, options)
            };
            self.gfx.draw_shape(&focus_ring);
        }
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

    /// Applies the current style settings for font family, text size, font
    /// style, and font weight.
    pub fn apply_current_font_settings(&mut self) {
        self.gfx
            .set_available_font_family(&self.widget.get(&FontFamily));
        self.gfx.set_font_size(self.widget.get(&TextSize));
        self.gfx.set_line_height(self.widget.get(&LineHeight));
        self.gfx.set_font_style(self.widget.get(&FontStyle));
        self.gfx.set_font_weight(self.widget.get(&FontWeight));
    }

    /// Invokes [`Widget::redraw()`](crate::widget::Widget::redraw) on this
    /// context's widget.
    pub fn redraw(&mut self) {
        let Some(layout) = self.last_layout() else {
            return;
        };
        if layout.size.width <= 0 || layout.size.height <= 0 {
            return;
        }

        self.tree.note_widget_rendered(self.current_node.node_id);
        let widget = self.current_node.clone();
        let mut widget = widget.lock();
        if !widget.as_widget().full_control_redraw() {
            let background = self.get(&WidgetBackground);
            self.fill(background);

            self.apply_current_font_settings();
            self.apply_opacity(self.get(&Opacity));
        }

        widget.as_widget().redraw(self);
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
            .layout(available_space, self)
            .map(Round::ceil);
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
    pub fn set_child_layout(&mut self, child: &MountedWidget, layout: Rect<Px>) {
        // TODO verify that `child` belongs to the current node.
        if self.persist_layout {
            child.set_layout(layout);
        }
    }
}

impl<'context, 'window, 'clip, 'gfx, 'pass> AsEventContext<'window>
    for LayoutContext<'context, 'window, 'clip, 'gfx, 'pass>
{
    fn as_event_context(&mut self) -> EventContext<'_, 'window> {
        self.graphics.as_event_context()
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
    fn push_child(&mut self, child: WidgetInstance) -> MountedWidget {
        let mut context = self.as_event_context();
        let pushed_widget = context.tree.push_boxed(child, Some(&context.current_node));
        pushed_widget
            .lock()
            .as_widget()
            .mounted(&mut context.for_other(&pushed_widget));
        pushed_widget
    }

    /// Removes a widget from the hierarchy.
    fn remove_child(&mut self, child: &MountedWidget) {
        let mut context = self.as_event_context();
        child
            .lock()
            .as_widget()
            .unmounted(&mut context.for_other(child));
        context.tree.remove_child(child, &context.current_node);
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
    current_node: MountedWidget,
    pub(crate) tree: Tree,
    window: &'context mut RunningWindow<'window>,
    theme: Cow<'context, ThemePair>,
    cursor: &'context mut CursorState,
    pending_state: PendingState<'context>,
    effective_styles: Styles,
    cache: WidgetCacheKey,
}

impl<'context, 'window> WidgetContext<'context, 'window> {
    pub(crate) fn new(
        current_node: MountedWidget,
        theme: &'context ThemePair,
        window: &'context mut RunningWindow<'window>,
        theme_mode: ThemeMode,
        cursor: &'context mut CursorState,
    ) -> Self {
        let enabled = current_node.enabled(&window.handle());
        let tree = current_node.tree();
        Self {
            pending_state: PendingState::Owned(PendingWidgetState {
                focus: tree
                    .focused_widget()
                    .and_then(|id| tree.widget_from_node(id).map(|w| w.id())),
                active: tree
                    .active_widget()
                    .and_then(|id| tree.widget_from_node(id).map(|w| w.id())),
                focus_is_advancing: false,
            }),
            tree,
            effective_styles: current_node.effective_styles(),
            cache: WidgetCacheKey {
                kludgine_id: Some(window.kludgine_id()),
                theme_mode,
                enabled,
            },
            cursor,
            current_node,
            theme: Cow::Borrowed(theme),
            window,
        }
    }

    /// Returns a new instance that borrows from `self`.
    pub fn borrowed(&mut self) -> WidgetContext<'_, 'window> {
        WidgetContext {
            tree: self.tree.clone(),
            current_node: self.current_node.clone(),
            window: &mut *self.window,
            theme: Cow::Borrowed(self.theme.as_ref()),
            pending_state: self.pending_state.borrowed(),
            cache: self.cache,
            effective_styles: self.effective_styles.clone(),
            cursor: &mut *self.cursor,
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
                Cow::Owned(theme.get_tracking_redraw(self))
            } else {
                Cow::Borrowed(self.theme.as_ref())
            };
            let theme_mode = if let Some(mode) = theme_mode {
                mode.get_tracking_redraw(self)
            } else {
                self.cache.theme_mode
            };
            WidgetContext {
                effective_styles,
                cache: WidgetCacheKey {
                    kludgine_id: self.cache.kludgine_id,
                    theme_mode,
                    enabled: current_node.enabled(&self.handle()),
                },
                current_node,
                tree: self.tree.clone(),
                window: &mut *self.window,
                theme,
                pending_state: self.pending_state.borrowed(),
                cursor: &mut *self.cursor,
            }
        })
    }

    /// Returns true if `possible_parent` is in this widget's parent list.
    #[must_use]
    pub fn is_child_of(&self, possible_parent: &WidgetInstance) -> bool {
        self.tree
            .is_child(self.current_node.node_id, possible_parent)
    }

    /// Returns true if this widget is enabled.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.cache.enabled
    }

    pub(crate) fn parent(&self) -> Option<MountedWidget> {
        self.current_node.parent()
    }

    /// Ensures that this widget will be redrawn when `value` has been updated.
    pub fn redraw_when_changed(&self, value: &impl Trackable) {
        value.redraw_when_changed(self.handle());
    }

    /// Ensures that this widget will be redrawn when `value` has been updated.
    pub fn invalidate_when_changed(&self, value: &impl Trackable) {
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
        self.pending_state.focus = Some(self.current_node.id());
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
        if self.focused(true) {
            self.clear_focus();
            true
        } else {
            false
        }
    }

    /// Returns true if the last focus event was an advancing motion, not a
    /// returning motion.
    ///
    /// This value is meaningless outside of focus-related events.
    pub fn focus_is_advancing(&mut self) -> bool {
        self.pending_state.focus_is_advancing
    }

    /// Activates this widget, if it is not already active.
    ///
    /// Returns true if this function resulted in the currently active widget
    /// being changed.
    ///
    /// Widget events relating to activation changes are deferred until after
    /// the all contexts for the currently firing event are dropped.
    pub fn activate(&mut self) -> bool {
        if self.pending_state.active == Some(self.current_node.id()) {
            false
        } else {
            self.pending_state.active = Some(self.current_node.id());
            true
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
        self.pending_state.active == Some(self.current_node.id())
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
    pub fn focused(&self, check_window: bool) -> bool {
        self.pending_state.focus == Some(self.current_node.id())
            && (!check_window || self.window.focused().get_tracking_redraw(self))
    }

    /// Returns true if this widget is the target to activate when the user
    /// triggers a default action.
    ///
    /// See
    /// [`MakeWidget::into_default()`](crate::widget::MakeWidget::into_default)
    /// for more information.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.tree.default_widget() == Some(self.current_node.node_id)
    }

    /// Returns true if this widget is the target to activate when the user
    /// triggers an escape action.
    ///
    /// See
    /// [`MakeWidget::into_escape()`](crate::widget::MakeWidget::into_escape)
    /// for more information.
    #[must_use]
    pub fn is_escape(&self) -> bool {
        self.tree.escape_widget() == Some(self.current_node.node_id)
    }

    /// Returns the widget this context is for.
    #[must_use]
    pub const fn widget(&self) -> &MountedWidget {
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

    /// Queries the widget hierarchy for a single style component.
    ///
    /// This function traverses up the widget hierarchy looking for the
    /// component being requested. If a matching component is found, it will be
    /// returned.
    #[must_use]
    pub fn try_get<Component: ComponentDefinition>(
        &self,
        query: &Component,
    ) -> Option<Component::ComponentType> {
        self.effective_styles.try_get(query, self)
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
        match self.cache.theme_mode {
            ThemeMode::Light => &self.theme.light,
            ThemeMode::Dark => &self.theme.dark,
        }
    }

    /// Returns the opposite theme of [`Self::theme()`].
    #[must_use]
    pub fn inverse_theme(&self) -> &Theme {
        match self.cache.theme_mode {
            ThemeMode::Light => &self.theme.dark,
            ThemeMode::Dark => &self.theme.light,
        }
    }

    /// Returns a key that can be checked to see if a widget should invalidate
    /// caches it stores.
    #[must_use]
    pub fn cache_key(&self) -> WidgetCacheKey {
        self.cache
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
    focus: Option<WidgetId>,
    active: Option<WidgetId>,
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

impl Eq for InvalidationStatus {}

impl PartialEq for InvalidationStatus {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.invalidated, &other.invalidated)
    }
}
impl std::hash::Hash for InvalidationStatus {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.invalidated).hash(state);
    }
}

/// A type chat can convert to a [`MountedWidget`] through a [`WidgetContext`].
pub trait ManageWidget {
    /// The managed type, which can be `Option<MountedWidget>` or
    /// `MountedWidget`.
    type Managed: MapManagedWidget<MountedWidget>;

    /// Resolve `self` into a [`MountedWidget`].
    fn manage(&self, context: &WidgetContext<'_, '_>) -> Self::Managed;
}

impl ManageWidget for WidgetId {
    type Managed = Option<MountedWidget>;

    fn manage(&self, context: &WidgetContext<'_, '_>) -> Self::Managed {
        context.tree.widget(*self)
    }
}

impl ManageWidget for WidgetInstance {
    type Managed = Option<MountedWidget>;

    fn manage(&self, context: &WidgetContext<'_, '_>) -> Self::Managed {
        context.tree.widget(self.id())
    }
}

impl ManageWidget for MountedWidget {
    type Managed = Self;

    fn manage(&self, _context: &WidgetContext<'_, '_>) -> Self::Managed {
        self.clone()
    }
}

/// A type that can produce another type when provided a [`MountedWidget`].
pub trait MapManagedWidget<T> {
    /// The result of the mapping operation.
    type Result;

    /// Call `map` with a [`MountedWidget`].
    fn map(self, map: impl FnOnce(MountedWidget) -> T) -> Self::Result;
}

impl<T> MapManagedWidget<T> for Option<MountedWidget> {
    type Result = Option<T>;

    fn map(self, map: impl FnOnce(MountedWidget) -> T) -> Self::Result {
        self.map(map)
    }
}

impl<T> MapManagedWidget<T> for MountedWidget {
    type Result = T;

    fn map(self, map: impl FnOnce(MountedWidget) -> T) -> Self::Result {
        map(self)
    }
}

/// An type that contains information about the state of a widget.
///
/// This value can be stored and compared in future widget events. If the cache
/// keys are not equal, the widget should clear all caches.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct WidgetCacheKey {
    kludgine_id: Option<KludgineId>,
    theme_mode: ThemeMode,
    enabled: bool,
}

impl Default for WidgetCacheKey {
    fn default() -> Self {
        Self {
            kludgine_id: None,
            theme_mode: ThemeMode::default().inverse(),
            enabled: false,
        }
    }
}

/// A type that can be tracked to refresh or invalidate widgets.
pub trait Trackable: sealed::Trackable {}

impl<T> Trackable for T where T: sealed::Trackable {}

pub(crate) mod sealed {
    use crate::widget::WidgetId;
    use crate::window::WindowHandle;

    pub trait Trackable {
        fn redraw_when_changed(&self, handle: WindowHandle);
        fn invalidate_when_changed(&self, handle: WindowHandle, id: WidgetId);
    }
}
