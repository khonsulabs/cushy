//! Widgets that stack in the Z-direction.

use std::fmt::{self, Debug};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use alot::{LotId, OrderedLots};
use cushy::widget::{RootBehavior, WidgetInstance};
use figures::units::{Lp, Px, UPx};
use figures::{IntoSigned, IntoUnsigned, Point, Rect, Size, Zero};
use intentional::Assert;

use crate::animation::easings::EaseOutQuadradic;
use crate::animation::{AnimationHandle, AnimationTarget, IntoAnimate, Spawn, ZeroToOne};
use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext, Trackable};
use crate::utils::IgnorePoison;
use crate::value::{Destination, Dynamic, DynamicGuard, IntoValue, Source, Value};
use crate::widget::{
    Callback, Children, MakeWidget, MountedChildren, MountedWidget, Widget, WidgetId, WidgetRef,
    WrapperWidget,
};
use crate::widgets::container::ContainerShadow;
use crate::ConstraintLimit;

/// A Z-direction stack of widgets.
#[derive(Debug)]
pub struct Layers {
    /// The children that are laid out as layers with index 0 being the lowest (bottom).
    pub children: Value<Children>,
    mounted: MountedChildren,
}

impl Layers {
    /// Returns a new instance that lays out `children` as layers.
    pub fn new(children: impl IntoValue<Children>) -> Self {
        Self {
            children: children.into_value(),
            mounted: MountedChildren::default(),
        }
    }

    fn synchronize_children(&mut self, context: &mut EventContext<'_, '_>) {
        self.children.invalidate_when_changed(context);
        self.mounted.synchronize_with(&self.children, context);
    }
}

impl Widget for Layers {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        self.synchronize_children(&mut context.as_event_context());

        for mounted in self.mounted.children() {
            context.for_other(mounted).redraw();
        }
    }

    fn summarize(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.children.map(|children| {
            let mut f = f.debug_tuple("Layered");
            for child in children {
                f.field(child);
            }

            f.finish()
        })
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        self.synchronize_children(&mut context.as_event_context());

        let mut size = Size::ZERO;

        for child in self.mounted.children() {
            size = size.max(
                context
                    .for_other(child)
                    .as_temporary()
                    .layout(available_space),
            );
        }

        // Now we know the size of the widget, we can request the widgets fill
        // the allocated space.
        let size = Size::new(
            available_space
                .width
                .fit_measured(size.width, context.gfx.scale()),
            available_space
                .height
                .fit_measured(size.height, context.gfx.scale()),
        );
        let layout = Rect::from(size.into_signed());
        for child in self.mounted.children() {
            context
                .for_other(child)
                .layout(size.map(ConstraintLimit::Fill));
            context.set_child_layout(child, layout);
        }

        size
    }

    fn mounted(&mut self, context: &mut EventContext<'_, '_>) {
        self.synchronize_children(context);
    }

    fn unmounted(&mut self, context: &mut EventContext<'_, '_>) {
        for child in self.mounted.drain() {
            context.remove_child(&child);
        }
    }

    fn root_behavior(
        &mut self,
        context: &mut EventContext<'_, '_>,
    ) -> Option<(RootBehavior, WidgetInstance)> {
        self.synchronize_children(context);

        for child in self.mounted.children() {
            let Some((child_behavior, next_in_chain)) = context.for_other(child).root_behavior()
            else {
                continue;
            };

            return Some((child_behavior, next_in_chain));
        }

        None
    }
}

/// A widget that displays other widgets relative to widgets in another layer.
///
/// This widget is for use inside of a [`Layers`] widget.
#[derive(Debug, Clone, Default)]
pub struct OverlayLayer {
    state: Dynamic<OverlayState>,
}

impl OverlayLayer {
    /// Returns a builder for a new overlay that can be shown on this layer.
    pub fn build_overlay(&self, overlay: impl MakeWidget) -> OverlayBuilder<'_> {
        OverlayBuilder {
            overlay: self,
            layout: OverlayLayout {
                widget: WidgetRef::new(overlay),
                relative_to: None,
                direction: Direction::Right,
                requires_hover: false,
                on_dismiss: None,
                layout: None,
                opacity: Dynamic::default(),
            },
        }
    }

    /// Returns a new wudget that shows a `tooltip` when `content` is hovered.
    pub fn new_tooltip(&self, tooltip: impl MakeWidget, content: impl MakeWidget) -> Tooltipped {
        Tooltipped {
            child: WidgetRef::new(content),
            data: TooltipData {
                target_layer: self.clone(),
                tooltip: tooltip.make_widget(),
                direction: Direction::Down,
                shown_tooltip: Dynamic::default(),
            },
            show_animation: None,
        }
    }
}

impl Widget for OverlayLayer {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_, '_>) {
        let state = self.state.lock();

        for child in &state.overlays {
            let Some(mounted) = child.widget.as_mounted(context) else {
                continue;
            };

            let opacity = child.opacity.get_tracking_redraw(context);
            let mut context = context.for_other(mounted);
            context.apply_opacity(opacity);
            context.redraw();
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Size<UPx> {
        let mut state = self.state.lock();
        state.prevent_notifications();

        let available_space = available_space.map(ConstraintLimit::max);

        state.process_new_overlays(&mut context.as_event_context());

        for index in 0..state.overlays.len() {
            let widget = state.overlays[index]
                .widget
                .mounted(&mut context.as_event_context());
            let Some(layout) = state.overlays[index]
                .layout
                .or_else(|| state.layout_overlay(index, &widget, available_space, context))
            else {
                continue;
            };

            let _ignored = context
                .for_other(&widget)
                .layout(layout.size.into_unsigned().map(ConstraintLimit::Fill));

            state.overlays[index].layout = Some(layout);
            context.set_child_layout(&widget, layout);
        }

        drop(state);

        // Now that we're done mutating state, we can register for invalidation
        // tracking.
        context.invalidate_when_changed(&self.state);

        // The overlay widget should never actualy impact the layout of other
        // layers, despite what layouts its children are assigned. This may seem
        // weird, but it would also be weird for a tooltop to expand its window
        // when shown.
        Size::ZERO
    }

    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_, '_>) -> bool {
        let state = self.state.lock();
        state.test_point(location, false, context).is_some()
    }

    fn hover(
        &mut self,
        location: Point<Px>,
        context: &mut EventContext<'_, '_>,
    ) -> Option<kludgine::app::winit::window::CursorIcon> {
        let mut state = self.state.lock();

        let hovering = state.test_point(location, true, context);
        if let Some(hovering) = hovering {
            let should_remove = state.hovering > Some(hovering);
            state.hovering = Some(hovering);
            if should_remove {
                remove_children_after(state, hovering);
            }
        } else {
            state.hovering = None;
        }

        None
    }

    fn unhover(&mut self, _context: &mut EventContext<'_, '_>) {
        let mut state = self.state.lock();
        state.hovering = None;

        let mut remove_starting_at = None;
        for (index, overlay) in state.overlays.iter().enumerate() {
            if overlay.requires_hover {
                remove_starting_at = Some(index);
                break;
            }
        }

        if let Some(remove_starting_at) = remove_starting_at {
            remove_children_after(state, remove_starting_at);
        }
    }
}

#[derive(Debug, Eq, PartialEq, Default)]
struct OverlayState {
    overlays: OrderedLots<OverlayLayout>,
    new_overlays: usize,
    hovering: Option<usize>,
}

fn remove_children_after(mut state: DynamicGuard<'_, OverlayState>, remove_starting_at: usize) {
    let mut removed = Vec::with_capacity(state.overlays.len() - remove_starting_at);
    while remove_starting_at < state.overlays.len() && !state.overlays.is_empty() {
        removed.push(state.overlays.pop());
        state.new_overlays = state.new_overlays.saturating_sub(1);
    }
    drop(state);
    // We delay dropping the removed widgets, as they may contain a
    // reference to this OverlayLayer.
    drop(removed);
}

impl OverlayState {
    fn test_point(
        &self,
        location: Point<Px>,
        check_original_relative: bool,
        context: &mut EventContext<'_, '_>,
    ) -> Option<usize> {
        for (index, overlay) in self.overlays.iter().enumerate() {
            if overlay.requires_hover
                && !overlay
                    .layout
                    .map_or(false, |check| !check.contains(location))
            {
                return Some(index + 1);
            }
        }

        if check_original_relative
            && !self.overlays.is_empty()
            && self.point_is_in_root_relative(location, context)
        {
            Some(0)
        } else {
            None
        }
    }

    fn point_is_in_root_relative(
        &self,
        location: Point<Px>,
        context: &mut EventContext<'_, '_>,
    ) -> bool {
        if let Some(relative_to) = self
            .overlays
            .get_by_index(0)
            .and_then(|overlay| overlay.relative_to)
            .and_then(|relative_to| relative_to.find_in(context))
            .and_then(|w| w.last_layout())
        {
            if !relative_to.contains(location) {
                return true;
            }
        }
        false
    }

    fn process_new_overlays(&mut self, context: &mut EventContext<'_, '_>) {
        while self.new_overlays > 0 {
            let new_index = self.overlays.len() - self.new_overlays;
            self.new_overlays -= 1;

            // Determine if new_overlay is relative to an existing overlay
            let new_overlay = self.overlays.get_mut_by_index(new_index).assert_expected();
            new_overlay.widget.mount_if_needed(context);

            let mut dismiss_from = 0;
            if let Some(context) = new_overlay
                .relative_to
                .and_then(|id| context.for_other(&id))
            {
                for existing in (0..new_index).rev() {
                    if context.is_child_of(self.overlays[existing].widget.widget()) {
                        // Relative to this overlay. Dismiss any overlays
                        // between this and the new one.
                        dismiss_from = existing + 1;
                        break;
                    }
                }
            }

            // Dismiss any overlays that are no longer going to be shown.
            for index in (dismiss_from..new_index).rev() {
                self.overlays.remove_by_index(index);
            }
        }
    }

    fn layout_overlay_relative(
        &mut self,
        index: usize,
        widget: &MountedWidget,
        available_space: Size<UPx>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
        relative_to: WidgetId,
    ) -> Option<Rect<Px>> {
        let direction = self.overlays[index].direction;
        let relative_to = relative_to.find_in(context)?.last_layout()?;
        let relative_to_unsigned = relative_to.into_unsigned();

        let constraints = match direction {
            Direction::Up => Size::new(
                relative_to_unsigned.size.width,
                relative_to_unsigned.origin.y,
            ),
            Direction::Down => Size::new(
                relative_to_unsigned.size.width,
                available_space.height
                    - relative_to_unsigned.origin.y
                    - relative_to_unsigned.size.height,
            ),
            Direction::Left => Size::new(
                relative_to_unsigned.origin.x,
                relative_to_unsigned.size.height,
            ),
            Direction::Right => Size::new(
                available_space.width.saturating_sub(
                    relative_to_unsigned
                        .origin
                        .x
                        .saturating_add(relative_to_unsigned.size.width),
                ),
                relative_to_unsigned.size.height,
            ),
        };

        let size = context
            .for_other(widget)
            .layout(constraints.map(ConstraintLimit::SizeToFit))
            .into_signed();

        let mut layout_direction = direction;
        let mut layout;
        loop {
            let origin = match layout_direction {
                Direction::Up => Point::new(
                    relative_to.origin.x + relative_to.size.width / 2 - size.width / 2,
                    relative_to.origin.y - size.height,
                ),
                Direction::Down => Point::new(
                    relative_to.origin.x + relative_to.size.width / 2 - size.width / 2,
                    relative_to.origin.y + relative_to.size.height,
                ),
                Direction::Left => Point::new(
                    relative_to.origin.x - size.width,
                    relative_to.origin.y + relative_to.size.height / 2 - size.height / 2,
                ),
                Direction::Right => Point::new(
                    relative_to.origin.x + relative_to.size.width,
                    relative_to.origin.y + relative_to.size.height / 2 - size.height / 2,
                ),
            };

            layout = Rect::new(origin.max(Point::ZERO), size);

            let bottom_right = layout.extent();
            if bottom_right.x > available_space.width {
                layout.origin.x -= bottom_right.x - available_space.width.into_signed();
            }
            if bottom_right.y > available_space.height {
                layout.origin.y -= bottom_right.y - available_space.height.into_signed();
            }

            if layout.intersects(&relative_to) || self.layout_intersects(index, &layout, context) {
                layout_direction = layout_direction.next_clockwise();
                if layout_direction == direction {
                    // No layout worked optimally.
                    break;
                }
            } else {
                break;
            }
        }
        Some(layout)
    }

    fn layout_intersects(
        &self,
        checking_index: usize,
        layout: &Rect<Px>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> bool {
        for index in (0..self.overlays.len()).filter(|&i| i != checking_index) {
            if self.overlays[index]
                .layout
                .map_or(false, |check| check.intersects(layout))
            {
                return true;
            }
        }

        // Verify that the the popup won't also obscure the original content.
        if checking_index != 0 {
            if let Some(relative_to) = self.overlays[0]
                .relative_to
                .and_then(|relative_to| relative_to.find_in(context))
                .and_then(|w| w.last_layout())
            {
                if relative_to.intersects(layout) {
                    return true;
                }
            }
        }

        false
    }

    fn layout_overlay(
        &mut self,
        index: usize,
        widget: &MountedWidget,
        available_space: Size<UPx>,
        context: &mut LayoutContext<'_, '_, '_, '_, '_>,
    ) -> Option<Rect<Px>> {
        if let Some(relative_to) = self.overlays[index].relative_to {
            self.layout_overlay_relative(index, widget, available_space, context, relative_to)
        } else {
            let direction = self.overlays[index].direction;
            let size = context
                .for_other(widget)
                .layout(available_space.map(ConstraintLimit::SizeToFit))
                .into_signed();

            let available_space = available_space.into_signed();

            let origin = match direction {
                Direction::Up => Point::new(
                    available_space.width / 2,
                    (available_space.height - size.height) / 2,
                ),
                Direction::Down => Point::new(
                    available_space.width / 2,
                    available_space.height / 2 + size.height / 2,
                ),
                Direction::Right => Point::new(
                    available_space.width / 2 + size.width / 2,
                    available_space.height / 2,
                ),
                Direction::Left => Point::new(
                    (available_space.width - size.width) / 2,
                    available_space.height / 2,
                ),
            };

            Some(Rect::new(origin, size))
        }
    }
}

/// A builder for overlaying a widget on an [`OverlayLayer`].
#[derive(Debug, Clone)]
pub struct OverlayBuilder<'a> {
    overlay: &'a OverlayLayer,
    layout: OverlayLayout,
}

impl OverlayBuilder<'_> {
    /// Sets this overlay to hide automatically when it or its relative widget
    /// are no longer hovered by the mouse cursor.
    #[must_use]
    pub fn hide_on_unhover(mut self) -> Self {
        self.layout.requires_hover = true;
        self
    }

    /// Show this overlay to the left of the specified widget.
    #[must_use]
    pub fn left_of(self, id: WidgetId) -> Self {
        self.near(id, Direction::Left)
    }

    /// Show this overlay to the right of the specified widget.
    #[must_use]
    pub fn right_of(self, id: WidgetId) -> Self {
        self.near(id, Direction::Right)
    }

    /// Show this overlay to show below the specified widget.
    #[must_use]
    pub fn below(self, id: WidgetId) -> Self {
        self.near(id, Direction::Down)
    }

    /// Show this overlay to show above the specified widget.
    #[must_use]
    pub fn above(self, id: WidgetId) -> Self {
        self.near(id, Direction::Up)
    }

    /// Shows this overlay near `id` off to the `direction` side.
    #[must_use]
    pub fn near(mut self, id: WidgetId, direction: Direction) -> Self {
        self.layout.relative_to = Some(id);
        self.layout.direction = direction;
        self
    }

    /// Sets `callback` to be invoked once this overlay is dismissed.
    #[must_use]
    pub fn on_dismiss(mut self, callback: Callback) -> Self {
        self.layout.on_dismiss = Some(Arc::new(Mutex::new(callback)));
        self
    }

    /// Shows this overlay, returning a handle that to the displayed overlay.
    #[must_use]
    pub fn show(self) -> OverlayHandle {
        self.fade_in();
        self.overlay.state.map_mut(|mut state| {
            state.new_overlays += 1;
            OverlayHandle {
                state: self.overlay.state.clone(),
                id: state.overlays.push(self.layout),
                dismiss_on_drop: true,
            }
        })
    }

    fn fade_in(&self) {
        self.layout
            .opacity
            .transition_to(ZeroToOne::ONE)
            .over(Duration::from_millis(250))
            .with_easing(EaseOutQuadradic)
            .launch();
    }
}

#[derive(Debug, Clone)]
struct OverlayLayout {
    widget: WidgetRef,
    opacity: Dynamic<ZeroToOne>,
    relative_to: Option<WidgetId>,
    direction: Direction,
    requires_hover: bool,
    layout: Option<Rect<Px>>,
    on_dismiss: Option<Arc<Mutex<Callback>>>,
}

impl Drop for OverlayLayout {
    fn drop(&mut self) {
        if let Some(on_dismiss) = &self.on_dismiss {
            let mut on_dismiss = on_dismiss.lock().ignore_poison();
            on_dismiss.invoke(());
        }
    }
}

impl Eq for OverlayLayout {}

impl PartialEq for OverlayLayout {
    fn eq(&self, other: &Self) -> bool {
        self.widget == other.widget
            && self.opacity == other.opacity
            && self.relative_to == other.relative_to
            && self.direction == other.direction
            && self.requires_hover == other.requires_hover
            && self.layout == other.layout
            && match (&self.on_dismiss, &other.on_dismiss) {
                (Some(this), Some(other)) => Arc::ptr_eq(this, other),
                (None, None) => true,
                _ => false,
            }
    }
}

/// A relative direction.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Direction {
    /// Negative along the Y axis.
    Up,
    /// Positive along the X axis.
    Right,
    /// Positive along the Y axis.
    Down,
    /// Legative along the X axis.
    Left,
}

impl Direction {
    /// Returns the next direction when rotating clockwise.
    #[must_use]
    pub fn next_clockwise(&self) -> Self {
        match self {
            Direction::Up => Direction::Right,
            Direction::Down => Direction::Left,
            Direction::Right => Direction::Down,
            Direction::Left => Direction::Up,
        }
    }
}

/// A handle to an overlay that was shown in an [`OverlayLayer`].
#[derive(PartialEq, Eq)]
pub struct OverlayHandle {
    state: Dynamic<OverlayState>,
    id: LotId,
    dismiss_on_drop: bool,
}

impl OverlayHandle {
    /// Dismisses this overlay and any overlays that have been displayed
    /// relative to it.
    pub fn dismiss(self) {
        drop(self);
    }

    /// Drops this handle without dismissing the overlay.
    pub fn forget(mut self) {
        self.dismiss_on_drop = false;
        drop(self);
    }
}

impl Drop for OverlayHandle {
    fn drop(&mut self) {
        if self.dismiss_on_drop {
            let mut state = self.state.lock();
            let Some(index) = state.overlays.index_of_id(self.id) else {
                return;
            };

            while state.overlays.len() > index {
                let _removed = state.overlays.pop();
            }
        }
    }
}

impl Debug for OverlayHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OverlayHandle")
            .field("id", &self.id)
            .field("dismiss_on_drop", &self.dismiss_on_drop)
            .finish_non_exhaustive()
    }
}

/// A widget that shows a tooltip when hovered.
#[derive(Debug)]
pub struct Tooltipped {
    child: WidgetRef,
    show_animation: Option<AnimationHandle>,
    data: TooltipData,
}

#[derive(Debug, Clone)]
struct TooltipData {
    target_layer: OverlayLayer,
    tooltip: WidgetInstance,
    direction: Direction,
    shown_tooltip: Dynamic<Option<OverlayHandle>>,
}

impl WrapperWidget for Tooltipped {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.child
    }

    fn hover(
        &mut self,
        _location: Point<Px>,
        context: &mut EventContext<'_, '_>,
    ) -> Option<kludgine::app::winit::window::CursorIcon> {
        let background_color = context.theme().surface.highest_container;

        let data = self.data.clone();
        let my_id = self.child.widget().id();

        self.show_animation = Some(
            Duration::from_millis(500)
                .on_complete(move || {
                    let mut shown_tooltip = data.shown_tooltip.lock();
                    if shown_tooltip.is_none() {
                        *shown_tooltip = Some(
                            data.target_layer
                                .build_overlay(
                                    data.tooltip
                                        .clone()
                                        .contain()
                                        .background_color(background_color)
                                        .shadow(ContainerShadow::drop(Lp::mm(1), Lp::mm(2))),
                                )
                                .hide_on_unhover()
                                .near(my_id, data.direction)
                                .show(),
                        );
                    }
                })
                .spawn(),
        );
        None
    }

    fn unhover(&mut self, _context: &mut EventContext<'_, '_>) {
        self.show_animation = None;
        self.data.shown_tooltip.set(None);
    }
}
