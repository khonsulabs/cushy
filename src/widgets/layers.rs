//! Widgets that stack in the Z-direction.

use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::time::Duration;

use alot::{LotId, OrderedLots};
use cushy::widget::{RootBehavior, WidgetInstance};
use easing_function::EasingFunction;
use figures::units::{Lp, Px, UPx};
use figures::{IntoSigned, IntoUnsigned, Point, Rect, Size, Zero};
use intentional::Assert;

use super::super::widget::MountedWidget;
use super::{Custom, Space};
use crate::animation::{AnimationHandle, AnimationTarget, IntoAnimate, Spawn, ZeroToOne};
use crate::context::{AsEventContext, EventContext, GraphicsContext, LayoutContext, Trackable};
use crate::reactive::value::{
    Destination, Dynamic, DynamicGuard, DynamicRead, IntoValue, Source, Value,
};
use crate::dialog::{ButtonBehavior, ShouldClose};
use crate::styles::components::{EasingIn, ScrimColor};
use crate::widget::{
    Callback, MakeWidget, MakeWidgetWithTag, MountedChildren, SharedCallback, Widget, WidgetId,
    WidgetList, WidgetRef, WidgetTag, WrapperWidget,
};
use crate::widgets::container::ContainerShadow;
use crate::ConstraintLimit;

/// A Z-direction stack of widgets.
#[derive(Debug)]
pub struct Layers {
    /// The children that are laid out as layers with index 0 being the lowest (bottom).
    pub children: Value<WidgetList>,
    mounted: MountedChildren,
}

impl Layers {
    /// Returns a new instance that lays out `children` as layers.
    pub fn new(children: impl IntoValue<WidgetList>) -> Self {
        Self {
            children: children.into_value(),
            mounted: MountedChildren::default(),
        }
    }

    fn synchronize_children(&mut self, context: &mut EventContext<'_>) {
        self.children.invalidate_when_changed(context);
        self.mounted.synchronize_with(&self.children, context);
    }
}

impl Widget for Layers {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
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
        context: &mut LayoutContext<'_, '_, '_, '_>,
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
            available_space.width.fit_measured(size.width),
            available_space.height.fit_measured(size.height),
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

    fn mounted(&mut self, context: &mut EventContext<'_>) {
        self.synchronize_children(context);
    }

    fn unmounted(&mut self, context: &mut EventContext<'_>) {
        for child in self.mounted.drain() {
            context.remove_child(&child);
        }
    }

    fn root_behavior(
        &mut self,
        context: &mut EventContext<'_>,
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
    easing: Dynamic<EasingFunction>,
}

impl OverlayLayer {
    /// Returns a builder for a new overlay that can be shown on this layer.
    pub fn build_overlay(&self, overlay: impl MakeWidget) -> OverlayBuilder<'_> {
        OverlayBuilder {
            overlay: self,
            layout: OverlayLayout {
                widget: WidgetRef::new(overlay),
                relative_to: None,
                positioning: Position::Relative(Direction::Right),
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

    /// Dismisses all currently presented overlays.
    pub fn dismiss_all(&self) {
        let mut state = self.state.lock();
        state.hovering = None;
        let removed = state.overlays.drain().collect::<Vec<_>>();
        state.new_overlays = 0;
        drop(state);

        // Since overlays contain references back to this structure, we need to
        // ensure their drop implementations happen after we've dropped our
        // lock.
        drop(removed);
    }
}

impl Widget for OverlayLayer {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        self.easing.set(context.get(&EasingIn));
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
        context: &mut LayoutContext<'_, '_, '_, '_>,
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

    fn hit_test(&mut self, location: Point<Px>, context: &mut EventContext<'_>) -> bool {
        let state = self.state.lock();
        state.test_point(location, false, context).is_some()
    }

    fn hover(
        &mut self,
        location: Point<Px>,
        context: &mut EventContext<'_>,
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

    fn unhover(&mut self, _context: &mut EventContext<'_>) {
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
        context: &mut EventContext<'_>,
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
        context: &mut EventContext<'_>,
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

    fn process_new_overlays(&mut self, context: &mut EventContext<'_>) {
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
        context: &mut LayoutContext<'_, '_, '_, '_>,
        relative_to: WidgetId,
    ) -> Option<Rect<Px>> {
        let positioning = self.overlays[index].positioning;
        let relative_to = relative_to.find_in(context)?.last_layout()?;
        let relative_to_unsigned = relative_to.into_unsigned();

        let constraints = match positioning {
            Position::Relative(Direction::Up) => Size::new(
                relative_to_unsigned.size.width,
                relative_to_unsigned.origin.y,
            ),
            Position::Relative(Direction::Down) => Size::new(
                relative_to_unsigned.size.width,
                available_space.height
                    - relative_to_unsigned.origin.y
                    - relative_to_unsigned.size.height,
            ),
            Position::Relative(Direction::Left) => Size::new(
                relative_to_unsigned.origin.x,
                relative_to_unsigned.size.height,
            ),
            Position::Relative(Direction::Right) => Size::new(
                available_space.width.saturating_sub(
                    relative_to_unsigned
                        .origin
                        .x
                        .saturating_add(relative_to_unsigned.size.width),
                ),
                relative_to_unsigned.size.height,
            ),
            Position::At(_) => available_space,
        };

        let size = context
            .for_other(widget)
            .layout(constraints.map(ConstraintLimit::SizeToFit))
            .into_signed();

        let mut layout_direction = positioning;
        let mut layout;
        loop {
            let (origin, intersection_matters) = match layout_direction {
                Position::Relative(Direction::Up) => (
                    Point::new(
                        relative_to.origin.x + relative_to.size.width / 2 - size.width / 2,
                        relative_to.origin.y - size.height,
                    ),
                    true,
                ),
                Position::Relative(Direction::Down) => (
                    Point::new(
                        relative_to.origin.x + relative_to.size.width / 2 - size.width / 2,
                        relative_to.origin.y + relative_to.size.height,
                    ),
                    true,
                ),
                Position::Relative(Direction::Left) => (
                    Point::new(
                        relative_to.origin.x - size.width,
                        relative_to.origin.y + relative_to.size.height / 2 - size.height / 2,
                    ),
                    true,
                ),
                Position::Relative(Direction::Right) => (
                    Point::new(
                        relative_to.origin.x + relative_to.size.width,
                        relative_to.origin.y + relative_to.size.height / 2 - size.height / 2,
                    ),
                    true,
                ),
                Position::At(pt) => (pt, false),
            };

            layout = Rect::new(origin.max(Point::ZERO), size);

            let bottom_right = layout.extent();
            if bottom_right.x > available_space.width {
                layout.origin.x -= bottom_right.x - available_space.width.into_signed();
            }
            if bottom_right.y > available_space.height {
                layout.origin.y -= bottom_right.y - available_space.height.into_signed();
            }

            if intersection_matters
                && (layout.intersects(&relative_to)
                    || self.layout_intersects(index, &layout, context))
            {
                if let Some(next_direction) = layout_direction.next_clockwise() {
                    if layout_direction == positioning {
                        // No layout worked optimally.
                        break;
                    }
                    layout_direction = next_direction;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // TODO check to ensure the widget is fully on-window, otherwise attempt
        // to shift it to become visible.

        Some(layout)
    }

    fn layout_intersects(
        &self,
        checking_index: usize,
        layout: &Rect<Px>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
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
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Option<Rect<Px>> {
        if let Some(relative_to) = self.overlays[index].relative_to {
            self.layout_overlay_relative(index, widget, available_space, context, relative_to)
        } else {
            let direction = self.overlays[index].positioning;
            let size = context
                .for_other(widget)
                .layout(available_space.map(ConstraintLimit::SizeToFit))
                .into_signed();

            let available_space = available_space.into_signed();

            let origin = match direction {
                Position::Relative(Direction::Up) => Point::new(
                    available_space.width / 2,
                    (available_space.height - size.height) / 2,
                ),
                Position::Relative(Direction::Down) => Point::new(
                    available_space.width / 2,
                    available_space.height / 2 + size.height / 2,
                ),
                Position::Relative(Direction::Right) => Point::new(
                    available_space.width / 2 + size.width / 2,
                    available_space.height / 2,
                ),
                Position::Relative(Direction::Left) => Point::new(
                    (available_space.width - size.width) / 2,
                    available_space.height / 2,
                ),
                Position::At(pt) => pt,
            };

            Some(Rect::new(origin, size))
        }
    }
}

/// A type that is being prepared to be shown in an [`OverlayLayer`].
pub trait Overlayable: Sized {
    /// The resulting handle type when this overlay is shown.
    type Handle;

    /// Sets this overlay to hide automatically when it or its relative widget
    /// are no longer hovered by the mouse cursor.
    #[must_use]
    fn hide_on_unhover(self) -> Self;

    /// Show this overlay with a relationship to another widget.
    #[must_use]
    fn parent(self, id: WidgetId) -> Self;

    /// Show this overlay to the left of the specified widget.
    #[must_use]
    fn left_of(self, id: WidgetId) -> Self;

    /// Show this overlay to the right of the specified widget.
    #[must_use]
    fn right_of(self, id: WidgetId) -> Self;

    /// Show this overlay to show below the specified widget.
    #[must_use]
    fn below(self, id: WidgetId) -> Self;

    /// Show this overlay to show above the specified widget.
    #[must_use]
    fn above(self, id: WidgetId) -> Self;

    /// Shows this overlay near `id` off to the `direction` side.
    #[must_use]
    fn near(self, id: WidgetId, direction: Direction) -> Self;

    /// Shows this overlay at a specified window `location`.
    #[must_use]
    fn at(self, location: Point<Px>) -> Self;

    /// Sets `callback` to be invoked once this overlay is dismissed.
    #[must_use]
    fn on_dismiss(self, callback: Callback) -> Self;

    /// Shows this overlay, returning a handle that to the displayed overlay.
    fn show(self) -> Self::Handle;
}

/// A builder for overlaying a widget on an [`OverlayLayer`].
#[derive(Debug, Clone)]
pub struct OverlayBuilder<'a> {
    overlay: &'a OverlayLayer,
    layout: OverlayLayout,
}

impl Overlayable for OverlayBuilder<'_> {
    type Handle = OverlayHandle;

    fn hide_on_unhover(mut self) -> Self {
        self.layout.requires_hover = true;
        self
    }

    fn parent(mut self, id: WidgetId) -> Self {
        self.layout.relative_to = Some(id);
        self
    }

    fn left_of(self, id: WidgetId) -> Self {
        self.near(id, Direction::Left)
    }

    fn right_of(self, id: WidgetId) -> Self {
        self.near(id, Direction::Right)
    }

    fn below(self, id: WidgetId) -> Self {
        self.near(id, Direction::Down)
    }

    fn above(self, id: WidgetId) -> Self {
        self.near(id, Direction::Up)
    }

    fn near(mut self, id: WidgetId, direction: Direction) -> Self {
        self.layout.relative_to = Some(id);
        self.layout.positioning = Position::Relative(direction);
        self
    }

    fn at(mut self, location: Point<Px>) -> Self {
        self.layout.positioning = Position::At(location);
        self
    }

    fn on_dismiss(mut self, callback: Callback) -> Self {
        self.layout.on_dismiss = Some(SharedCallback::from(callback));
        self
    }

    fn show(self) -> Self::Handle {
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
}

impl OverlayBuilder<'_> {
    fn fade_in(&self) {
        self.layout
            .opacity
            .transition_to(ZeroToOne::ONE)
            .over(Duration::from_millis(250))
            .with_easing(self.overlay.easing.get())
            .launch();
    }
}

#[derive(Debug, Clone)]
struct OverlayLayout {
    widget: WidgetRef,
    opacity: Dynamic<ZeroToOne>,
    relative_to: Option<WidgetId>,
    positioning: Position<Px>,
    requires_hover: bool,
    layout: Option<Rect<Px>>,
    on_dismiss: Option<SharedCallback>,
}

impl Drop for OverlayLayout {
    fn drop(&mut self) {
        if let Some(on_dismiss) = &self.on_dismiss {
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
            && self.positioning == other.positioning
            && self.requires_hover == other.requires_hover
            && self.layout == other.layout
            && self.on_dismiss == other.on_dismiss
    }
}

/// An overlay position.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Position<T> {
    /// Relative to the parent in a given direction.
    Relative(Direction),
    /// At a window coordinate.
    At(Point<T>),
}

impl<T> Position<T> {
    /// Returns the next direction when rotating clockwise.
    #[must_use]
    pub fn next_clockwise(&self) -> Option<Self> {
        match self {
            Self::Relative(direction) => Some(Self::Relative(direction.next_clockwise())),
            Self::At(_) => None,
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
    /// Negative along the X axis.
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
#[must_use = "Overlay handles will dismiss the shown overlay when dropped."]
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

            while state.overlays.len() - state.new_overlays > index {
                let _removed = state.overlays.remove_by_index(index);
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
        context: &mut EventContext<'_>,
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
                                        .shadow(ContainerShadow::drop(Lp::mm(1))),
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

    fn unhover(&mut self, _context: &mut EventContext<'_>) {
        self.show_animation = None;
        self.data.shown_tooltip.set(None);
    }
}

/// A layer to present a widget in a modal session.
///
/// Designed to be used in a [`Layers`] widget.
#[derive(Debug, Clone, Default)]
pub struct Modal {
    modal: Dynamic<OrderedLots<WidgetInstance>>,
}

impl Modal {
    /// Returns a new modal layer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            modal: Dynamic::default(),
        }
    }

    /// Presents `contents` as the modal session.
    pub fn present(&self, contents: impl MakeWidget) {
        self.present_inner(contents);
    }

    fn present_inner(&self, contents: impl MakeWidget) -> LotId {
        let mut state = self.modal.lock();
        state.push(contents.make_widget())
    }

    /// Returns a new pending handle that can be used to show a modal and
    /// dismiss it.
    #[must_use]
    pub fn new_handle(&self) -> ModalHandle {
        ModalHandle {
            layer: self.clone(),
            above: None,
            id: Dynamic::default(),
        }
    }

    /// Presents a modal dialog containing `message` with a default button that
    /// dismisses the dialog.
    pub fn message(&self, message: impl MakeWidget, button_caption: impl MakeWidget) {
        self.build_dialog(message)
            .with_default_button(button_caption, ShouldClose::Close)
            .show();
    }

    /// Returns a builder for a modal dialog that displays `message`.
    pub fn build_dialog(&self, message: impl MakeWidget) -> DialogBuilder {
        DialogBuilder::new(self.new_handle(), message)
    }

    /// Dismisses the modal session.
    pub fn dismiss(&self) {
        self.modal.lock().clear();
    }

    /// Returns true if this layer is currently presenting a modal session.
    #[must_use]
    pub fn visible(&self) -> bool {
        !self.modal.lock().is_empty()
    }

    /// Returns a function that dismisses the modal when invoked.
    ///
    /// The input to the function is ignored. This function takes a single
    /// argument so that it is compatible with widgets that use a [`Callback`]
    /// for their events.
    pub fn dismiss_callback<T>(&self) -> impl FnMut(T) + Send + 'static {
        let modal = self.clone();
        move |_| {
            modal.dismiss();
        }
    }
}

impl MakeWidgetWithTag for Modal {
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance {
        let layer_widgets = Dynamic::default();

        ModalLayer {
            layers: WidgetRef::new(Layers::new(layer_widgets.clone())),
            layer_widgets,
            presented: Vec::new(),
            focus_top_layer: false,
            modal: self.modal,
        }
        .make_with_tag(tag)
    }
}

#[derive(Debug)]
struct ModalLayer {
    presented: Vec<WidgetInstance>,
    layer_widgets: Dynamic<WidgetList>,
    layers: WidgetRef,
    modal: Dynamic<OrderedLots<WidgetInstance>>,
    focus_top_layer: bool,
}

impl WrapperWidget for ModalLayer {
    fn child_mut(&mut self) -> &mut WidgetRef {
        &mut self.layers
    }

    fn adjust_child_constraints(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<ConstraintLimit> {
        self.modal.invalidate_when_changed(context);
        let modal = self.modal.read();
        let mut layer_widgets = self.layer_widgets.lock();
        self.focus_top_layer = false;
        for index in 0..modal.len().min(self.presented.len()) {
            let modal_widget = &modal[index];
            let presented = &mut self.presented[index];
            if presented != modal_widget {
                let modal_widget = modal_widget.clone();
                *presented = modal_widget.clone();
                layer_widgets[index * 2 + 1] = modal_widget.clone().centered().make_widget();

                self.focus_top_layer = true;
            }
        }

        for to_present in modal.iter().skip(self.presented.len()) {
            self.focus_top_layer = true;
            layer_widgets.push(
                Custom::new(Space::colored(context.get(&ScrimColor))).on_hit_test(|_, _| true),
            );
            self.presented.push(to_present.clone());
            layer_widgets.push(to_present.clone().centered());
        }

        if self.presented.len() > modal.len() {
            self.presented.truncate(modal.len());
            layer_widgets.truncate(modal.len() * 2);
            self.focus_top_layer = true;
        }

        available_space
    }

    fn position_child(
        &mut self,
        size: Size<Px>,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> crate::widget::WrappedLayout {
        if self.focus_top_layer {
            self.focus_top_layer = false;
            if let Some(mut ctx) = self
                .presented
                .last()
                .and_then(|topmost| context.for_other(topmost))
            {
                ctx.focus();
            }
        }
        Size::new(
            available_space.width.fit_measured(size.width),
            available_space.height.fit_measured(size.height),
        )
        .into()
    }
}

/// A marker type indicating a special [`DialogBuilder`] button type is not
/// present.
pub enum No {}

/// A marker type indicating a special [`DialogBuilder`] button type is present.
pub enum Yes {}

/// A modal dialog builder.
#[must_use = "DialogBuilder::show must be called for the dialog to be shown"]
pub struct DialogBuilder<HasDefault = No, HasCancel = No> {
    handle: ModalHandle,
    message: WidgetInstance,
    buttons: WidgetList,
    _state: PhantomData<(HasDefault, HasCancel)>,
}

impl DialogBuilder<No, No> {
    fn new(handle: ModalHandle, message: impl MakeWidget) -> Self {
        Self {
            handle,
            message: message.make_widget(),
            buttons: WidgetList::new(),
            _state: PhantomData,
        }
    }
}

impl<HasDefault, HasCancel> DialogBuilder<HasDefault, HasCancel> {
    /// Adds a button with `caption` that invokes `on_click` when activated.
    /// Returns self.
    pub fn with_button(mut self, caption: impl MakeWidget, on_click: impl ButtonBehavior) -> Self {
        self.push_button(caption, on_click);
        self
    }

    /// Pushes a button with `caption` that invokes `on_click` when activated.
    pub fn push_button(&mut self, caption: impl MakeWidget, on_click: impl ButtonBehavior) {
        self.inner_push_button(caption, DialogButtonKind::Plain, on_click);
    }

    fn inner_push_button(
        &mut self,
        caption: impl MakeWidget,
        kind: DialogButtonKind,
        mut on_click: impl ButtonBehavior,
    ) {
        let modal = self.handle.clone();
        let mut button = caption
            .into_button()
            .on_click(move |_| {
                if let ShouldClose::Close = on_click.clicked() {
                    modal.dismiss();
                }
            })
            .make_widget();
        match kind {
            DialogButtonKind::Plain => {}
            DialogButtonKind::Default => button = button.into_default(),
            DialogButtonKind::Cancel => button = button.into_escape(),
        }
        self.buttons.push(button.fit_horizontally().make_widget());
    }

    /// Shows the modal dialog, returning a handle that owns the session.
    pub fn show(mut self) {
        if self.buttons.is_empty() {
            self.inner_push_button("OK", DialogButtonKind::Default, ShouldClose::Close);
        }
        self.handle.present(
            self.message
                .and(self.buttons.into_columns().centered())
                .into_rows()
                .contain(),
        );
    }
}

impl<HasCancel> DialogBuilder<No, HasCancel> {
    /// Adds a default button with `caption` that invokes `on_click` when
    /// activated.
    pub fn with_default_button(
        mut self,
        caption: impl MakeWidget,
        on_click: impl ButtonBehavior,
    ) -> DialogBuilder<Yes, HasCancel> {
        self.inner_push_button(caption, DialogButtonKind::Default, on_click);
        let Self {
            handle,
            message,
            buttons,
            _state,
        } = self;
        DialogBuilder {
            handle,
            message,
            buttons,
            _state: PhantomData,
        }
    }
}

impl<HasDefault> DialogBuilder<HasDefault, No> {
    /// Adds a cancel button with `caption` that invokes `on_click` when
    /// activated.
    pub fn with_cancel_button(
        mut self,
        caption: impl MakeWidget,
        on_click: impl FnMut() -> ShouldClose + Send + 'static,
    ) -> DialogBuilder<HasDefault, Yes> {
        self.inner_push_button(caption, DialogButtonKind::Cancel, on_click);
        let Self {
            handle,
            message,
            buttons,
            _state,
        } = self;
        DialogBuilder {
            handle,
            message,
            buttons,
            _state: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
enum DialogButtonKind {
    Plain,
    Default,
    Cancel,
}

/// A handle to a modal dialog presented in a [`Modal`] layer.
#[derive(Clone)]
pub struct ModalHandle {
    layer: Modal,
    above: Option<Dynamic<Option<LotId>>>,
    id: Dynamic<Option<LotId>>,
}

impl ModalHandle {
    fn above(mut self, other: &Self) -> Self {
        self.above = Some(dbg!(other.id.clone()));
        self
    }

    /// Presents `contents` as a modal dialog, updating this handle to control
    /// it.
    pub fn present(&self, contents: impl MakeWidget) {
        let mut state = self.layer.modal.lock();
        if let Some(above) = self.above.as_ref().and_then(Source::get) {
            if let Some(index) = state.index_of_id(above) {
                state.truncate(index + 1);
            } else {
                self.id.set(None);
                return;
            }
        } else {
            state.clear();
        };
        self.id.set(Some(dbg!(state.push(contents.make_widget()))));
    }

    // /// Prevents the modal shown by this handle from being dismissed when the
    // /// last reference is dropped.
    // pub fn persist(self) {
    //     self.id.set(None);
    //     drop(self);
    // }

    /// Dismisses the modal shown by this handle.
    pub fn dismiss(&self) {
        let Some(id) = self.id.take() else {
            return;
        };
        let mut state = self.layer.modal.lock();
        let Some(index) = state.index_of_id(id) else {
            return;
        };
        state.truncate(index);
    }

    /// Returns the modal layer the dialog is presented on.
    #[must_use]
    pub const fn layer(&self) -> &Modal {
        &self.layer
    }

    /// Returns a builder for a modal dialog that replaces the current contents
    /// of this modal with `message` and presents it.
    pub fn build_dialog(&self, message: impl MakeWidget) -> DialogBuilder {
        DialogBuilder::new(self.clone(), message)
    }

    /// Returns a builder for a modal dialog that displays `message` in a modal
    /// dialog above the dialog shown by this handle.
    pub fn build_nested_dialog(&self, message: impl MakeWidget) -> DialogBuilder {
        DialogBuilder::new(self.new_handle(), message)
    }
}

impl Drop for ModalHandle {
    fn drop(&mut self) {
        if self.id.instances() == 1 {
            self.dismiss();
        }
    }
}

/// A target for a [`Modal`] session.
pub trait ModalTarget: Send + 'static {
    /// Returns a new handle that can be used to show a dialog above `self`.
    fn new_handle(&self) -> ModalHandle;
    /// Returns a reference to the modal layer this target presents to.
    fn layer(&self) -> &Modal;
}

impl ModalTarget for Modal {
    fn new_handle(&self) -> ModalHandle {
        self.new_handle()
    }

    fn layer(&self) -> &Modal {
        self
    }
}

impl ModalTarget for ModalHandle {
    fn new_handle(&self) -> ModalHandle {
        self.layer.new_handle().above(self)
    }

    fn layer(&self) -> &Modal {
        &self.layer
    }
}
