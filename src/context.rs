use std::ops::{Deref, DerefMut};

use kludgine::app::winit::event::{
    DeviceId, Ime, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoSigned, Point, Rect, Size};
use kludgine::shapes::{Shape, StrokeOptions};
use kludgine::Kludgine;

use crate::dynamic::Dynamic;
use crate::graphics::Graphics;
use crate::styles::{ComponentDefaultvalue, HighlightColor, Styles};
use crate::widget::{BoxedWidget, EventHandling, ManagedWidget};
use crate::window::RunningWindow;
use crate::ConstraintLimit;

pub struct EventContext<'context, 'window> {
    pub widget: WidgetContext<'context, 'window>,
    pub kludgine: &'context mut Kludgine,
}

impl<'context, 'window> EventContext<'context, 'window> {
    pub fn new(widget: WidgetContext<'context, 'window>, kludgine: &'context mut Kludgine) -> Self {
        Self { widget, kludgine }
    }

    pub fn for_other<'child>(
        &'child mut self,
        widget: &'child ManagedWidget,
    ) -> EventContext<'child, 'window> {
        EventContext::new(self.widget.for_other(widget), self.kludgine)
    }

    pub fn hit_test(&mut self, location: Point<Px>) -> bool {
        self.current_node.lock().hit_test(location, self)
    }

    pub fn mouse_down(
        &mut self,
        location: Point<Px>,
        device_id: DeviceId,
        button: MouseButton,
    ) -> EventHandling {
        self.current_node
            .lock()
            .mouse_down(location, device_id, button, self)
    }

    pub fn mouse_drag(&mut self, location: Point<Px>, device_id: DeviceId, button: MouseButton) {
        self.current_node
            .lock()
            .mouse_drag(location, device_id, button, self);
    }

    pub fn mouse_up(
        &mut self,
        location: Option<Point<Px>>,
        device_id: DeviceId,
        button: MouseButton,
    ) {
        self.current_node
            .lock()
            .mouse_up(location, device_id, button, self);
    }

    pub fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
    ) -> EventHandling {
        self.current_node
            .lock()
            .keyboard_input(device_id, input, is_synthetic, self)
    }

    pub fn ime(&mut self, ime: Ime) -> EventHandling {
        self.current_node.lock().ime(ime, self)
    }

    pub fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    ) -> EventHandling {
        self.current_node
            .lock()
            .mouse_wheel(device_id, delta, phase, self)
    }

    pub(crate) fn hover(&mut self, location: Point<Px>) {
        let newly_hovered = match self.current_node.tree.hover(Some(self.current_node)) {
            Ok(old_hover) => {
                if let Some(old_hover) = old_hover {
                    let mut old_hover_context = self.for_other(&old_hover);
                    old_hover.lock().unhover(&mut old_hover_context);
                }
                true
            }
            Err(_) => false,
        };
        if newly_hovered {
            self.current_node.lock().hover(location, self);
        }
    }

    pub(crate) fn clear_hover(&mut self) {
        if let Ok(Some(old_hover)) = self.current_node.tree.hover(None) {
            let mut old_hover_context = self.for_other(&old_hover);
            old_hover.lock().unhover(&mut old_hover_context);
        }
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

pub enum Exclusive<'a, T> {
    Borrowed(&'a mut T),
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

pub struct GraphicsContext<'context, 'window, 'clip, 'gfx, 'pass> {
    pub widget: WidgetContext<'context, 'window>,
    pub graphics: Exclusive<'context, Graphics<'clip, 'gfx, 'pass>>,
}

impl<'context, 'window, 'clip, 'gfx, 'pass> GraphicsContext<'context, 'window, 'clip, 'gfx, 'pass> {
    pub fn for_other<'child>(
        &'child mut self,
        widget: &'child ManagedWidget,
    ) -> GraphicsContext<'child, 'window, 'clip, 'gfx, 'pass> {
        GraphicsContext {
            widget: self.widget.for_other(widget),
            graphics: Exclusive::Borrowed(&mut *self.graphics),
        }
    }

    pub fn measure(&mut self, available_space: Size<ConstraintLimit>) -> Size<UPx> {
        self.current_node.lock().measure(available_space, self)
    }

    pub fn redraw(&mut self) {
        // TODO this should not use clip_rect, because it forces UPx, and once
        // we have scrolling, we can have negative offsets of rectangles where
        // it's clipped partially.
        self.current_node
            .note_rendered_rect(self.graphics.clip_rect().into_signed());
        self.current_node.lock().redraw(self);
    }

    pub fn clipped_to(&mut self, clip: Rect<UPx>) -> GraphicsContext<'_, 'window, '_, 'gfx, 'pass> {
        GraphicsContext {
            widget: self.widget.borrowed(),
            graphics: Exclusive::Owned(self.graphics.clipped_to(clip)),
        }
    }

    pub fn draw_focus_ring(&mut self, styles: &Styles) {
        let visible_rect = Rect::from(self.graphics.size() - (UPx(1), UPx(1)));
        let focus_ring = Shape::stroked_rect(
            visible_rect,
            styles.get_or_default(&HighlightColor),
            StrokeOptions::default(),
        );
        self.graphics
            .draw_shape(&focus_ring, Point::default(), None, None);
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

pub trait AsEventContext<'window> {
    fn as_event_context(&mut self) -> EventContext<'_, 'window>;

    #[must_use]
    fn push_child(&mut self, child: BoxedWidget) -> ManagedWidget {
        let mut context = self.as_event_context();
        let pushed_widget = context
            .current_node
            .tree
            .push_boxed(child, Some(context.current_node));
        pushed_widget
            .lock()
            .mounted(&mut context.for_other(&pushed_widget));
        pushed_widget
    }

    fn remove_child(&mut self, child: &ManagedWidget) {
        let mut context = self.as_event_context();
        context
            .current_node
            .tree
            .remove_child(child, context.current_node);
        child.lock().unmounted(&mut context.for_other(child));
    }

    fn apply_pending_state(&mut self) {
        let mut context = self.as_event_context();
        let active = context.pending_state.active.take();
        if context.current_node.tree.active_widget() != active.as_ref().map(|active| active.id) {
            let new = match context.current_node.tree.activate(active.as_ref()) {
                Ok(old) => {
                    if let Some(old) = old {
                        let mut old_context = context.for_other(&old);
                        old.lock().deactivate(&mut old_context);
                    }
                    true
                }
                Err(_) => false,
            };
            if new {
                if let Some(active) = active {
                    active.lock().activate(&mut context);
                }
            }
        }

        let focus = context.pending_state.focus.take();
        if context.current_node.tree.focused_widget() != focus.as_ref().map(|focus| focus.id) {
            let new = match context.current_node.tree.focus(focus.as_ref()) {
                Ok(old) => {
                    if let Some(old) = old {
                        let mut old_context = context.for_other(&old);
                        old.lock().blur(&mut old_context);
                    }
                    true
                }
                Err(_) => false,
            };
            if new {
                if let Some(focus) = focus {
                    focus.lock().focus(&mut context);
                }
            }
        }
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

pub struct WidgetContext<'context, 'window> {
    current_node: &'context ManagedWidget,
    window: &'context mut RunningWindow<'window>,
    pending_state: PendingState<'context>,
}

impl<'context, 'window> WidgetContext<'context, 'window> {
    pub(crate) fn new(
        current_node: &'context ManagedWidget,
        window: &'context mut RunningWindow<'window>,
    ) -> Self {
        Self {
            current_node,
            window,
            pending_state: PendingState::Owned(PendingWidgetState {
                focus: current_node
                    .tree
                    .focused_widget()
                    .map(|id| current_node.tree.widget(id)),
                active: current_node
                    .tree
                    .active_widget()
                    .map(|id| current_node.tree.widget(id)),
            }),
        }
    }

    pub fn borrowed(&mut self) -> WidgetContext<'_, 'window> {
        WidgetContext {
            current_node: self.current_node,
            window: &mut *self.window,
            pending_state: self.pending_state.borrowed(),
        }
    }

    pub fn for_other<'child>(
        &'child mut self,
        widget: &'child ManagedWidget,
    ) -> WidgetContext<'child, 'window> {
        WidgetContext {
            current_node: widget,
            window: &mut *self.window,
            pending_state: self.pending_state.borrowed(),
        }
    }

    pub(crate) fn parent(&self) -> Option<ManagedWidget> {
        self.current_node.parent()
    }

    pub fn redraw_when_changed<T>(&self, value: &Dynamic<T>) {
        value.redraw_when_changed(self.window.handle());
    }

    #[must_use]
    pub fn last_rendered_at(&self) -> Option<Rect<Px>> {
        self.current_node.last_rendered_at()
    }

    pub fn focus(&mut self) {
        self.pending_state.focus = Some(self.current_node.clone());
    }

    pub(crate) fn clear_focus(&mut self) {
        self.pending_state.focus = None;
    }

    pub fn blur(&mut self) -> bool {
        if self.focused() {
            self.clear_focus();
            true
        } else {
            false
        }
    }

    pub fn activate(&mut self) -> bool {
        if self
            .pending_state
            .active
            .as_ref()
            .map_or(true, |active| active != self.current_node)
        {
            self.pending_state.active = Some(self.current_node.clone());
            true
        } else {
            false
        }
    }

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

    #[must_use]
    pub fn active(&self) -> bool {
        self.pending_state.active.as_ref() == Some(self.current_node)
    }

    #[must_use]
    pub fn hovered(&self) -> bool {
        self.current_node.hovered()
    }

    #[must_use]
    pub fn focused(&self) -> bool {
        self.pending_state.focus.as_ref() == Some(self.current_node)
    }

    #[must_use]
    pub const fn widget(&self) -> &ManagedWidget {
        self.current_node
    }

    pub fn attach_styles(&self, styles: Styles) {
        self.current_node.attach_styles(styles);
    }

    #[must_use]
    pub fn query_style(&self, query: &[&dyn ComponentDefaultvalue]) -> Styles {
        self.current_node.tree.query_style(self.current_node, query)
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
