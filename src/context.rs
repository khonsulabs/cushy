use std::ops::{Deref, DerefMut};

use kludgine::app::winit::event::{DeviceId, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase};
use kludgine::figures::units::{Px, UPx};
use kludgine::figures::{IntoSigned, Point, Rect, Size};
use kludgine::Kludgine;

use crate::dynamic::Dynamic;
use crate::graphics::Graphics;
use crate::styles::{ComponentDefaultvalue, Styles};
use crate::widget::{BoxedWidget, EventHandling, ManagedWidget};
use crate::window::RunningWindow;
use crate::ConstraintLimit;

pub struct Context<'context, 'window> {
    current_node: &'context ManagedWidget,
    window: &'context mut RunningWindow<'window>,
    pending_state: PendingState<'context>,
}

impl<'context, 'window> Context<'context, 'window> {
    pub fn new(
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

    pub fn for_other<'child>(
        &'child mut self,
        widget: &'child ManagedWidget,
    ) -> Context<'child, 'window> {
        Context {
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

    pub fn redraw(&mut self, graphics: &mut Graphics<'_, '_, '_>) {
        // TODO this should not use clip_rect, because it forces UPx, and once
        // we have scrolling, we can have negative offsets of rectangles where
        // it's clipped partially.
        self.current_node
            .note_rendered_rect(graphics.clip_rect().into_signed());
        self.current_node.lock().redraw(graphics, self);
    }

    pub fn measure(
        &mut self,
        available_space: Size<ConstraintLimit>,
        graphics: &mut Graphics<'_, '_, '_>,
    ) -> Size<UPx> {
        self.current_node
            .lock()
            .measure(available_space, graphics, self)
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
        kludgine: &mut Kludgine,
    ) -> EventHandling {
        self.current_node
            .lock()
            .keyboard_input(device_id, input, is_synthetic, kludgine, self)
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

    #[must_use]
    pub fn push_child(&mut self, child: BoxedWidget) -> ManagedWidget {
        let pushed_widget = self
            .current_node
            .tree
            .push_boxed(child, Some(self.current_node));
        pushed_widget
            .lock()
            .mounted(&mut self.for_other(&pushed_widget));
        pushed_widget
    }

    pub fn remove_child(&mut self, child: &ManagedWidget) {
        self.current_node
            .tree
            .remove_child(child, self.current_node);
        child.lock().unmounted(&mut self.for_other(child));
    }

    #[must_use]
    pub fn last_rendered_at(&self) -> Option<Rect<Px>> {
        self.current_node.last_rendered_at()
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

    fn apply_pending_state(&mut self) {
        let active = self.pending_state.active.take();
        if self.current_node.tree.active_widget() != active.as_ref().map(|active| active.id) {
            let new = match self.current_node.tree.activate(active.as_ref()) {
                Ok(old) => {
                    if let Some(old) = old {
                        let mut old_context = self.for_other(&old);
                        old.lock().deactivate(&mut old_context);
                    }
                    true
                }
                Err(_) => false,
            };
            if new {
                if let Some(active) = active {
                    active.lock().activate(self);
                }
            }
        }

        let focus = self.pending_state.focus.take();
        if self.current_node.tree.focused_widget() != focus.as_ref().map(|focus| focus.id) {
            let new = match self.current_node.tree.focus(focus.as_ref()) {
                Ok(old) => {
                    if let Some(old) = old {
                        let mut old_context = self.for_other(&old);
                        old.lock().blur(&mut old_context);
                    }
                    true
                }
                Err(_) => false,
            };
            if new {
                if let Some(focus) = focus {
                    focus.lock().focus(self);
                }
            }
        }
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

impl Drop for Context<'_, '_> {
    fn drop(&mut self) {
        if matches!(self.pending_state, PendingState::Owned(_)) {
            self.apply_pending_state();
        }
    }
}

impl<'window> Deref for Context<'_, 'window> {
    type Target = RunningWindow<'window>;

    fn deref(&self) -> &Self::Target {
        self.window
    }
}
impl<'window> DerefMut for Context<'_, 'window> {
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
