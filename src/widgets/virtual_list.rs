use std::collections::VecDeque;
use std::fmt::Debug;
use std::ops::Range;

use cushy::context::LayoutContext;
use cushy::ConstraintLimit;
use figures::IntoUnsigned;

use super::scroll::OwnedWidget;
use crate::context::{AsEventContext, EventContext, Trackable};
use crate::figures::units::{Px, UPx};
use crate::figures::{IntoSigned, Point, Rect, Round, Size, Zero};
use crate::kludgine::app::winit::event::{MouseScrollDelta, TouchPhase};
use crate::kludgine::app::winit::window::CursorIcon;
use crate::value::{
    Destination, Dynamic, DynamicReader, IntoDynamic, IntoValue, MapEachCloned, Source, Watcher,
};
use crate::widget::{
    Callback, EventHandling, MakeWidget, MountedWidget, Widget, WidgetInstance, HANDLED, IGNORED,
};
use crate::widgets::scroll::ScrollBar;
use crate::window::DeviceId;

#[derive(Debug)]
struct RowMaker(Callback<usize, WidgetInstance>);

impl RowMaker {
    fn make_row(
        &mut self,
        index: usize,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> VirtualListItem {
        VirtualListItem {
            index,
            mounted: context.push_child(self.0.invoke(index)),
        }
    }
}

#[derive(Debug)]
struct VirtualListItem {
    index: usize,
    mounted: MountedWidget,
}

#[derive(Debug)]
/// A virtuallized list view
///
/// This widget allows scrolling a list of rows by lazily loading only the rows
/// that are currently being displayed to the screen.
pub struct VirtualList {
    make_row: RowMaker,
    vertical_scroll: OwnedWidget<ScrollBar>,
    horizontal_scroll: OwnedWidget<ScrollBar>,
    items: VecDeque<VirtualListItem>,
    content_size: Dynamic<Size<UPx>>,
    contents: Watcher,
    contents_generation: usize,
    /// Maximum scroll value - `max_scroll.y` + `control_size.height` should be
    /// the height of the content.
    pub max_scroll: DynamicReader<Point<UPx>>,
    /// Current scroll value. Changes to this dynamic will scroll the list
    /// programmatically.
    pub scroll: Dynamic<Point<UPx>>,
    control_size: Dynamic<Size<UPx>>,

    item_count: DynamicReader<usize>,
    item_size: Dynamic<Size<UPx>>,

    visible_range: Dynamic<Range<usize>>,
}

impl VirtualList {
    /// Creates a new [`VirtualList`] that displays `item_count` rows, loading
    /// each row as needed by invoking `make_row`.
    ///
    /// `make_row` will be called each time a new row becomes visible. As rows
    /// are no longer visible, they will be freed, ensuring a minimum number of
    /// widgets is kept in memory at any given time.
    ///
    /// Each row will be sized to match the first visible row. To ensure all
    /// rows have a consistent size, use the [`Resize`](../Resize) widget.
    pub fn new<MakeRow, Row>(item_count: impl IntoValue<usize>, mut make_row: MakeRow) -> Self
    where
        MakeRow: FnMut(usize) -> Row + Send + 'static,
        Row: MakeWidget,
    {
        let make_row = RowMaker(Callback::new(move |row| make_row(row).make_widget()));
        let scroll = Dynamic::<Point<UPx>>::default();
        let item_size = Dynamic::new(Size::ZERO);
        let item_count = item_count.into_value().into_dynamic().into_reader();
        let content_size = Dynamic::new(Size::default());

        let x = scroll.map_each_cloned(|scroll| scroll.x);
        x.for_each_cloned({
            let scroll = scroll.clone();
            move |x| {
                if let Ok(mut scroll) = scroll.try_lock() {
                    if scroll.x != x {
                        scroll.x = x;
                    }
                }
            }
        })
        .persist();
        let y = scroll.map_each_cloned(|scroll| scroll.y);
        y.for_each_cloned({
            let scroll = scroll.clone();
            move |y| {
                if let Ok(mut scroll) = scroll.try_lock() {
                    if scroll.y != y {
                        scroll.y = y;
                    }
                }
            }
        })
        .persist();
        let horizontal = ScrollBar::new(content_size.map_each_cloned(|size| size.width), x, false);
        let mut vertical =
            ScrollBar::new(content_size.map_each_cloned(|size| size.height), y, true);
        vertical.synchronize_visibility_with(&horizontal);
        let max_scroll = (&horizontal.max_scroll(), &vertical.max_scroll())
            .map_each_cloned(|(x, y)| Point::new(x, y))
            .into_reader();

        let contents = Watcher::default();
        let contents_generation = contents.get();

        Self {
            make_row,
            contents,
            contents_generation,
            vertical_scroll: OwnedWidget::new(vertical),
            horizontal_scroll: OwnedWidget::new(horizontal),
            items: VecDeque::new(),
            control_size: Dynamic::new(Size::default()),
            content_size,
            max_scroll,
            scroll,

            item_size,
            item_count,
            visible_range: Dynamic::default(),
        }
    }

    /// Returns a [`Watcher`] that when notified will force this list to reload
    /// its contents, including the currently visible rows.
    pub const fn content_watcher(&self) -> &Watcher {
        &self.contents
    }

    /// Returns a reader for the maximum scroll value.
    ///
    /// This represents the maximum amount that the scroll can be moved by.
    #[must_use]
    pub const fn max_scroll(&self) -> &DynamicReader<Point<UPx>> {
        &self.max_scroll
    }

    /// Returns a reader for the size of the scrollable area.
    #[must_use]
    pub fn content_size(&self) -> DynamicReader<Size<UPx>> {
        self.content_size.create_reader()
    }

    /// Returns a reader for the size of this Scroll widget.
    #[must_use]
    pub fn control_size(&self) -> DynamicReader<Size<UPx>> {
        self.control_size.create_reader()
    }

    /// Returns a reader for number of visible items. 0 indexed.
    #[must_use]
    pub fn visible_range(&self) -> DynamicReader<Range<usize>> {
        self.visible_range.create_reader()
    }

    fn show_scrollbars(&mut self, context: &mut EventContext<'_>) {
        let mut vertical = self.vertical_scroll.expect_made_mut().widget().lock();
        vertical
            .downcast_mut::<ScrollBar>()
            .expect("a ScrollBar")
            .show(context);
    }

    fn hide_scrollbars(&mut self, context: &mut EventContext<'_>) {
        let mut vertical = self.vertical_scroll.expect_made_mut().widget().lock();
        vertical
            .downcast_mut::<ScrollBar>()
            .expect("a ScrollBar")
            .hide(context);
    }

    fn clear(&mut self, context: &mut LayoutContext<'_, '_, '_, '_>) {
        for item in self.items.drain(..) {
            context.remove_child(&item.mounted);
        }
    }

    fn layout_scrollbars(
        &mut self,
        available_space: Size<ConstraintLimit>,
        new_control_size: Size<UPx>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) {
        let horizontal = self
            .horizontal_scroll
            .make_if_needed()
            .mounted(&mut context.as_event_context());
        let scrollbar_layout = context.for_other(&horizontal).layout(available_space);
        context.set_child_layout(
            &horizontal,
            Rect::new(
                Point::new(
                    Px::ZERO,
                    available_space
                        .height
                        .fit_measured(new_control_size.height)
                        .saturating_sub(scrollbar_layout.height)
                        .into_signed(),
                ),
                scrollbar_layout.into_signed(),
            ),
        );
        let vertical = self
            .vertical_scroll
            .make_if_needed()
            .mounted(&mut context.as_event_context());
        let scrollbar_layout = context.for_other(&vertical).layout(available_space);
        context.set_child_layout(
            &vertical,
            Rect::new(
                Point::new(
                    available_space
                        .width
                        .fit_measured(new_control_size.width)
                        .saturating_sub(scrollbar_layout.width)
                        .into_signed(),
                    Px::ZERO,
                ),
                scrollbar_layout.into_signed(),
            ),
        );
    }

    fn layout_rows(
        &mut self,
        item_count: usize,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let generation = self.contents.get_tracking_redraw(context);
        if generation != self.contents_generation {
            self.contents_generation = generation;
            self.clear(context);
        }
        let mut item_size = self.calculate_item_size(available_space, context).ceil();

        let content_height = item_size.height * u32::try_from(item_count).unwrap_or(u32::MAX);
        let content_height = content_height.into_unsigned();

        let new_control_size = Size::new(
            available_space.width.fill_or_fit(item_size.width),
            available_space.height.fill_or_fit(content_height),
        )
        .ceil();
        if item_size.width < new_control_size.width {
            item_size.width = new_control_size.width;
        }

        self.layout_scrollbars(available_space, new_control_size, context);
        let scroll = self.scroll.get_tracking_invalidate(context);

        let max_scroll_x = item_size.width.saturating_sub(new_control_size.width);
        let max_scroll_y = content_height.saturating_sub(new_control_size.height);
        let scroll = scroll.min(Point::new(max_scroll_x, max_scroll_y));

        let start_item = (scroll.y.floor() / item_size.height).floor().get() as usize;
        let end_item = ((scroll.y.ceil() + new_control_size.height) / item_size.height)
            .ceil()
            .get() as usize;
        let end_item = end_item.min(item_count - 1);

        self.visible_range.set(start_item..end_item);

        let first = self.items.front().map(|t| t.index);
        let last = self.items.back().map(|t| t.index);

        if self.items.is_empty() || first.unwrap() > end_item || last.unwrap() < start_item {
            self.clear(context);
            self.items.extend(
                (start_item..=end_item).map(|index| self.make_row.make_row(index, context)),
            );
        } else {
            let first = first.expect("List is not empty");
            let last = last.expect("List is not empty");
            while self
                .items
                .front()
                .map_or(false, |item| item.index < start_item)
            {
                context.remove_child(&self.items.pop_front().expect("at least one item").mounted);
            }
            while self
                .items
                .back()
                .map_or(false, |item| item.index > end_item)
            {
                context.remove_child(&self.items.pop_back().expect("at least one item").mounted);
            }
            // no extend front :(
            for item in (start_item..first).rev() {
                self.items.push_front(self.make_row.make_row(item, context));
            }
            self.items.extend(
                ((last + 1)..=end_item).map(|index| self.make_row.make_row(index, context)),
            );
        }

        let x = -scroll.x.into_signed();
        let mut y = -(scroll.y % item_size.height).into_signed();
        let constraint = item_size.map(ConstraintLimit::Fill);
        for item in &self.items {
            let child_size = context.for_other(&item.mounted).layout(constraint);

            context.set_child_layout(
                &item.mounted,
                Rect::new(Point::new(x, y), item_size.min(child_size).into_signed()),
            );
            y += item_size.height.into_signed();
        }

        self.control_size.set(new_control_size);
        self.content_size
            .set(Size::new(item_size.width, content_height));
        self.item_size.set(item_size);

        new_control_size
    }

    fn calculate_item_size(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        if self.items.is_empty() {
            self.items.push_front(self.make_row.make_row(0, context));
        }

        context
            .for_other(
                &self
                    .items
                    .front()
                    .expect("at least one mounted item")
                    .mounted,
            )
            .layout(available_space.map(|space| ConstraintLimit::SizeToFit(space.max())))
    }
}

impl Widget for VirtualList {
    fn hit_test(&mut self, _location: Point<Px>, _context: &mut EventContext<'_>) -> bool {
        true
    }

    fn hover(
        &mut self,
        _location: Point<Px>,
        context: &mut EventContext<'_>,
    ) -> Option<CursorIcon> {
        self.show_scrollbars(context);

        None
    }

    fn unhover(&mut self, context: &mut EventContext<'_>) {
        self.hide_scrollbars(context);
    }

    fn mounted(&mut self, context: &mut EventContext<'_>) {
        for child in &mut self.items {
            child.mounted.remount_if_needed(context);
        }
    }

    fn redraw(&mut self, context: &mut cushy::context::GraphicsContext<'_, '_, '_, '_>) {
        self.item_count.invalidate_when_changed(context);
        self.contents.invalidate_when_changed(context);
        for child in &mut self.items {
            context.for_other(&child.mounted).redraw();
        }
        let vertical = self
            .vertical_scroll
            .expect_made_mut()
            .mounted(&mut context.as_event_context());
        context.for_other(&vertical).redraw();
        let horizontal = self
            .horizontal_scroll
            .expect_made_mut()
            .mounted(&mut context.as_event_context());
        context.for_other(&horizontal).redraw();
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let item_count = self.item_count.get_tracking_invalidate(context);
        if item_count == 0 {
            return available_space.map(ConstraintLimit::min);
        }

        self.layout_rows(item_count, available_space, context)
    }

    fn mouse_wheel(
        &mut self,
        _device_id: DeviceId,
        delta: MouseScrollDelta,
        _phase: TouchPhase,
        context: &mut EventContext<'_>,
    ) -> EventHandling {
        let mut handled = false;
        {
            let mut vertical = self.vertical_scroll.expect_made().widget().lock();
            handled |= vertical
                .downcast_mut::<ScrollBar>()
                .expect("a ScrollBar")
                .mouse_wheel(delta, context)
                .is_break();
            let mut horizontal = self.horizontal_scroll.expect_made().widget().lock();
            handled |= horizontal
                .downcast_mut::<ScrollBar>()
                .expect("a ScrollBar")
                .mouse_wheel(delta, context)
                .is_break();
        }
        if handled {
            self.show_scrollbars(context);
            context.set_needs_redraw();

            HANDLED
        } else {
            IGNORED
        }
    }
}
