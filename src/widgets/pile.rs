//! A widget that piles multiple widgets into a single area.

use std::collections::VecDeque;
use std::sync::Arc;

use ahash::AHashMap;
use alot::{LotId, Lots};
use figures::units::UPx;
use figures::{IntoSigned, Rect, Size};
use intentional::Assert;

use crate::context::{EventContext, GraphicsContext, LayoutContext};
use crate::value::{Dynamic, DynamicRead, DynamicReader};
use crate::widget::{MakeWidget, MakeWidgetWithTag, Widget, WidgetInstance, WidgetRef, WidgetTag};
use crate::ConstraintLimit;

/// A pile of widgets that shows the top widget.
///
/// This is a lower level widget that is similar to a
/// [`Switcher`](super::switcher::Switcher) except that all widgets held in the
/// pile remain mounted in the window when not active. This allows widgets to
/// retain information stored in a [`WindowLocal`](crate::window::WindowLocal).
#[derive(Debug, Clone, Default)]
pub struct Pile {
    data: Dynamic<PileData>,
}

#[derive(Default, Debug)]
struct PileData {
    widgets: Lots<Option<WidgetInstance>>,
    visible: VecDeque<LotId>,
    focus_on_show: bool,
}

impl PileData {
    fn hide_id(&mut self, to_remove: LotId) {
        let Some((index, _)) = self
            .visible
            .iter()
            .enumerate()
            .find(|(_index, id)| **id == to_remove)
        else {
            return;
        };
        self.visible.remove(index);
    }
}

impl Pile {
    /// Returns a placeholder that can be used to show/close a piled widget
    /// before it has been constructed.
    #[must_use]
    pub fn new_pending(&self) -> PendingPiledWidget {
        let mut pile = self.data.lock();
        let id = pile.widgets.push(None);
        PendingPiledWidget(Some(PiledWidget(Arc::new(PiledWidgetData {
            pile: self.clone(),
            id,
        }))))
    }

    /// Adds a new widget to the pile.
    ///
    /// If this is the first widget, it will become visible automatically.
    /// Otherwise, it will be placed at the bottom of the pile.
    ///
    /// When the last clone of the returned [`PiledWidget`] is dropped, `widget`
    /// will be removed from the pile. If it is the currently visible widget,
    /// the next widget in the pile will be made visible.
    pub fn push(&self, widget: impl MakeWidget) -> PiledWidget {
        self.new_pending().finish(widget)
    }
}

impl MakeWidgetWithTag for Pile {
    fn make_with_tag(self, tag: WidgetTag) -> WidgetInstance {
        WidgetPile {
            pile: self.data.into_reader(),
            widgets: AHashMap::new(),
            last_visible: None,
        }
        .make_with_tag(tag)
    }
}

#[derive(Debug)]
struct WidgetPile {
    pile: DynamicReader<PileData>,
    widgets: AHashMap<LotId, WidgetRef>,
    last_visible: Option<LotId>,
}

impl WidgetPile {
    fn synchronize_widgets(&mut self) {
        let pile = self.pile.read();
        for (id, widget) in pile.widgets.entries() {
            if let Some(widget) = widget.as_ref() {
                self.widgets
                    .entry(id)
                    .or_insert_with(|| WidgetRef::new(widget.clone()));
            }
        }

        self.widgets.retain(|id, _| pile.widgets.get(*id).is_some());
    }
}

impl Widget for WidgetPile {
    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        context.invalidate_when_changed(&self.pile);
        self.synchronize_widgets();
        let pile = self.pile.read();
        let visible = pile.visible.front().copied();
        let size = if let Some(id) = visible {
            let visible = self
                .widgets
                .get_mut(&id)
                .expect("visible widget")
                .mounted(context);
            let mut child_context = context.for_other(&visible);
            if pile.focus_on_show && self.last_visible != Some(id) {
                child_context.focus();
            }
            let size = child_context.layout(available_space);
            drop(child_context);
            context.set_child_layout(&visible, Rect::from(size).into_signed());
            size
        } else {
            available_space.map(ConstraintLimit::min)
        };

        self.last_visible = visible;

        size
    }

    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        context.invalidate_when_changed(&self.pile);
        self.synchronize_widgets();
        let pile = self.pile.read();
        if let Some(visible) = pile.visible.front() {
            let visible = self
                .widgets
                .get_mut(visible)
                .expect("visible widget")
                .mounted(context);
            context.for_other(&visible).redraw();
        }
    }

    fn unmounted(&mut self, context: &mut EventContext<'_>) {
        for widget in self.widgets.values_mut() {
            widget.unmount_in(context);
        }
    }
}

/// A placeholder for a widget in a [`Pile`].
pub struct PendingPiledWidget(Option<PiledWidget>);

impl PendingPiledWidget {
    /// Place `widget` in the pile and returns a handle to the placed widget.
    #[allow(clippy::must_use_candidate)]
    pub fn finish(mut self, widget: impl MakeWidget) -> PiledWidget {
        let piled = self.0.take().assert("finished called once");
        let mut pile = piled.0.pile.data.lock();
        pile.widgets[piled.0.id] = Some(widget.make_widget());
        pile.visible.push_back(piled.0.id);
        drop(pile);

        piled
    }
}

impl std::ops::Deref for PendingPiledWidget {
    type Target = PiledWidget;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("accessed after finished")
    }
}

/// A widget that has been added to a [`Pile`].
#[derive(Clone, Debug)]
pub struct PiledWidget(Arc<PiledWidgetData>);

impl PiledWidget {
    /// Shows this widget in its pile.
    pub fn show(&self) {
        self.show_inner(false);
    }

    /// Shows this widget in its pile and directs keyboard focus to it.
    pub fn show_and_focus(&self) {
        self.show_inner(true);
    }

    fn show_inner(&self, focus: bool) {
        let mut pile = self.0.pile.data.lock();
        pile.hide_id(self.0.id);
        pile.visible.push_front(self.0.id);
        pile.focus_on_show = focus;
    }

    /// Removes this widget from the pile.
    pub fn remove(&self) {
        let mut pile = self.0.pile.data.lock();
        if pile.visible.front() == Some(&self.0.id) {
            pile.focus_on_show = false;
        }
        pile.hide_id(self.0.id);
        pile.widgets.remove(self.0.id);
    }
}

#[derive(Clone, Debug)]
struct PiledWidgetData {
    pile: Pile,
    id: LotId,
}

impl Drop for PiledWidgetData {
    fn drop(&mut self) {
        let mut pile = self.pile.data.lock();
        pile.hide_id(self.id);
        pile.widgets.remove(self.id);
    }
}
