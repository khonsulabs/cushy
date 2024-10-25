use std::{collections::VecDeque, fmt::Debug, ops::Range};

use crate::{context::{AsEventContext, EventContext}, figures::{units::{Px, UPx}, IntoSigned, Point, Rect, Round, ScreenScale, Size, Zero}, kludgine::app::winit::{event::{ MouseScrollDelta, TouchPhase}, window::CursorIcon}, styles::Dimension, value::{Destination, Dynamic, DynamicReader, IntoDynamic, IntoValue, Source}, widget::{EventHandling, MakeWidget, MountedWidget, Widget, HANDLED, IGNORED}, widgets::scroll::ScrollBar, window::DeviceId, ConstraintLimit};

use super::scroll::OwnedWidget;

/// A virtual list contents.
/// This simple virtual list assumes that all items have the same height, width and that the item count is known.
/// All the values are dynamic, so the list will update when the values change.
pub trait VirtualListContent: Debug {
    /// Single item height
    fn item_height(&self) -> impl IntoValue<Dimension>;
    /// Width of the items
    fn width(&self) -> impl IntoValue<Dimension>;
    /// Number of items
    fn item_count(&self) -> impl IntoValue<usize>;
    /// Create a widget for the item at the given index.
    /// This is called when the widget comes into view. The widget may be removed at any moment (by scrolling it out of view) and recreated later.
    fn widget_at(&self, index: usize) -> impl MakeWidget;
}

#[derive(Debug)]
struct VirtualListItem {
    index: usize,
    mounted: MountedWidget,
}

#[derive(Debug)]
/// A virtual list widget.
/// Requires a [VirtualListContent] trait implementation to render the items.
/// Items are lazily recreated as they go in and out of view.
pub struct VirtualList<T: VirtualListContent + Send + 'static> {
    virtual_list: T,
    vertical_scroll: OwnedWidget<ScrollBar>,
    items: VecDeque<VirtualListItem>,
    content_size: Dynamic<Size<UPx>>,
    /// Maximum scroll value - max_scroll.y + control_size.height should be the height of the content.
    pub max_scroll: DynamicReader<Point<UPx>>,
    /// Current scroll value. The x value is always 0. Change the value to scroll the widget programmatically.
    pub scroll: Dynamic<Point<UPx>>,
    control_size: Dynamic<Size<UPx>>,

    /// Height of an item. Based on [VirtualListContent::item_height].
    pub item_height: DynamicReader<Dimension>,
    /// Width of the items. Based on [VirtualListContent::width].
    pub width: DynamicReader<Dimension>,
    /// Number of items. Based on [VirtualListContent::item_count].
    pub item_count: DynamicReader<usize>,

    visible_range: Dynamic<Range<usize>>
}

impl<T: VirtualListContent + Send + 'static> VirtualList<T> {
    /// Creates a new [VirtualList] based on the given [VirtualListContent].
    pub fn new(virtual_list: T) -> Self {
        let scroll = Dynamic::<Point<UPx>>::default();
        let item_height = virtual_list.item_height().into_value().into_dynamic().create_reader();
        let width = virtual_list.width().into_value().into_dynamic().create_reader();
        let item_count = virtual_list.item_count().into_value().into_dynamic().create_reader();
        let content_size = Dynamic::new(Size::default());

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
        let vertical =
            ScrollBar::new(content_size.map_each_cloned(|size| size.height), y, true);
        let max_scroll = (&vertical.max_scroll())
            .map_each_cloned(|y| Point::new(UPx::ZERO, y))
            .into_reader();

        Self {
            virtual_list,
            vertical_scroll: OwnedWidget::new(vertical),
            items: VecDeque::new(),
            control_size: Dynamic::new(Size::default()),
            content_size,
            max_scroll,
            scroll,

            item_height,
            width,
            item_count,
            visible_range: Default::default()
        }
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
}

impl<T: VirtualListContent + Send + 'static> Widget for VirtualList<T> {
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
        for child in &mut self.items {
            context.for_other(&child.mounted).redraw();
        }
        let vertical = self
            .vertical_scroll
            .expect_made_mut()
            .mounted(&mut context.as_event_context());
        context.for_other(&vertical).redraw();
    }

    fn layout(
            &mut self,
            available_space: Size<cushy::ConstraintLimit>,
            context: &mut cushy::context::LayoutContext<'_, '_, '_, '_>,
        ) -> Size<UPx> {
        let item_height = self.item_height.get_tracking_invalidate(context);
        let item_height_upx = item_height.into_upx(context.gfx.scale());
        let item_count = self.item_count.get_tracking_invalidate(context);
        let content_height = item_height * item_count as i32;
        let content_height = content_height.into_upx(context.gfx.scale());
        let width = self.width.get_tracking_invalidate(context);
        let width = width.into_upx(context.gfx.scale());

        let new_control_size = Size::new(
            width,
            constrain_child(available_space.height, content_height),
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
                    available_space.width
                        .fit_measured(new_control_size.width)
                        .saturating_sub(scrollbar_layout.width)
                        .into_signed(),
                    Px::ZERO,
                ),
                scrollbar_layout.into_signed(),
            ),
        );
        let scroll = self.scroll.get_tracking_invalidate(context);

        let start_item = (scroll.y / item_height_upx).floor().get() as usize;
        let end_item = ((scroll.y + new_control_size.height) / item_height_upx).ceil().get() as usize;
        let end_item = end_item.min(item_count-1);

        self.visible_range.set(start_item..end_item);
        
        let first = self.items.front().map(|t| t.index);
        let last = self.items.back().map(|t| t.index);
        let mut closure = |index| {
            let widget = self.virtual_list.widget_at(index);
            let mut widget = widget.widget_ref();
            let mounted = widget.mounted(&mut context.as_event_context());
            VirtualListItem { index, mounted }
        };
        if self.items.is_empty() || first.unwrap() > end_item || last.unwrap() < start_item {
            self.items.clear();
            self.items.extend((start_item..=end_item).map(closure));
        } else {
            let first = first.expect("List is not empty");
            let last = last.expect("List is not empty");
            if first < start_item {
                while self.items.front().is_some() && self.items.front().expect("Checked is some").index < start_item {
                    self.items.pop_front();
                }
            }
            if last > end_item {
                while self.items.back().is_some() && self.items.back().expect("Checked is some").index > end_item {
                    self.items.pop_back();
                }
            }
            // no extend front :(
            for item in (start_item..first).rev() {
                self.items.push_front(closure(item));
            }
            self.items.extend(((last+1)..=end_item).map(closure));
        }

        let item_size = Size::new(width, item_height_upx);
        let constraint = item_size.map(ConstraintLimit::Fill);

        for item in &self.items {
            context.for_other(&item.mounted).layout(constraint);
        }

        let item_size = item_size.into_signed();
        let scroll = self.scroll.get_tracking_invalidate(context).into_signed();

        for item in &self.items {
            context.set_child_layout(
                &item.mounted,
                Rect::new(
                    Point::new(Px::ZERO, (item_height_upx * item.index as f32).into_signed() - scroll.y),
                    item_size,
                )
            );
        }
        
        self.control_size.set(new_control_size);
        self.content_size.set(Size::new(width, content_height));

        new_control_size
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

fn constrain_child(constraint: ConstraintLimit, measured: UPx) -> UPx {
    match constraint {
        ConstraintLimit::Fill(size) => size.min(measured),
        // change from Scroll widget: returning just measured here would break the functionality (render too many items)
        ConstraintLimit::SizeToFit(size) => size.min(measured),
    }
}