//! Overlay menu widgets.

use std::fmt::Debug;
use std::sync::Arc;

use figures::units::{Lp, Px, UPx};
use figures::{IntoSigned, Point, Rect, Size, Zero};
use parking_lot::Mutex;

use super::button::{ButtonClick, ButtonKind};
use super::container::ContainerShadow;
use super::layers::{OverlayBuilder, OverlayHandle, OverlayLayer, Overlayable};
use crate::context::{GraphicsContext, LayoutContext};
use crate::value::Dynamic;
use crate::widget::{Callback, MakeWidget, Widget, WidgetInstance, WidgetRef};
use crate::ConstraintLimit;

/// An overlayable menu of selectable items.
///
/// This widget is designed to implement Cushy's contextual menu system. When
/// used with an [`OverlayLayer`], this widget can be shown above other widgets
/// or at a specific location.
#[derive(Debug, Clone)]
pub struct Menu<T> {
    items: Vec<MenuItem<T>>,
    on_click: Option<Arc<Mutex<Callback<ChosenMenuItem<T>>>>>,
}

impl<T> Default for Menu<T>
where
    T: Debug + Send + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Menu<T>
where
    T: Debug + Send + Clone + 'static,
{
    /// Returns a new, empty menu.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            on_click: None,
        }
    }

    /// Adds another menu `item` that is displayed using `widget`.
    #[must_use]
    pub fn with(mut self, item: impl Into<MenuItem<T>>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Sets the selected handler to `selected`, causing it to be invoked when
    /// an item is chosen.
    #[must_use]
    pub fn on_selected<F>(mut self, selected: F) -> Self
    where
        F: FnMut(ChosenMenuItem<T>) + Send + 'static,
    {
        self.on_click = Some(Arc::new(Mutex::new(Callback::new(selected))));
        self
    }

    /// Presents this menu in `overlay`, returning an [`Overlayable`] that can
    /// be positioned relative or absolutely within `overlay`.
    #[must_use]
    pub fn overlay_in<'overlay>(&self, overlay: &'overlay OverlayLayer) -> MenuOverlay<'overlay> {
        let Self { items, on_click } = self;
        let handle = OpenMenuHandle(Dynamic::new(None));
        let items = items
            .iter()
            .map(|item| {
                let MenuItem { value, widget } = item;
                let handle = handle.clone();
                OpenItem {
                    contents: WidgetRef::new(
                        widget
                            .clone()
                            .into_button()
                            .kind(ButtonKind::Transparent)
                            .on_click({
                                let on_click = on_click.clone();
                                let value = value.clone();
                                move |click| {
                                    if let Some(on_click) = &on_click {
                                        let mut on_click = on_click.lock();
                                        on_click.invoke(ChosenMenuItem {
                                            item: value.clone(),
                                            click,
                                        });
                                    }
                                    handle.dismiss();
                                }
                            }),
                    ),
                    height: UPx::ZERO,
                }
            })
            .collect();
        MenuOverlay(
            overlay.build_overlay(
                OpenMenu {
                    items,
                    handle: handle.clone(),
                }
                .contain()
                .shadow(ContainerShadow::drop(Lp::mm(1), Lp::mm(2)))
                .vertical_scroll(),
            ),
            handle,
        )
    }
}

/// A [`Menu`] that is preparing to be shown in an [`OverlayLayer`].
pub struct MenuOverlay<'a>(OverlayBuilder<'a>, OpenMenuHandle);

impl<'a> Overlayable for MenuOverlay<'a> {
    type Handle = OpenMenuHandle;

    fn hide_on_unhover(self) -> Self {
        Self(self.0.hide_on_unhover(), self.1)
    }

    fn left_of(self, id: crate::widget::WidgetId) -> Self {
        Self(self.0.left_of(id), self.1)
    }

    fn right_of(self, id: crate::widget::WidgetId) -> Self {
        Self(self.0.right_of(id), self.1)
    }

    fn below(self, id: crate::widget::WidgetId) -> Self {
        Self(self.0.below(id), self.1)
    }

    fn above(self, id: crate::widget::WidgetId) -> Self {
        Self(self.0.above(id), self.1)
    }

    fn near(self, id: crate::widget::WidgetId, direction: super::layers::Direction) -> Self {
        Self(self.0.near(id, direction), self.1)
    }

    fn at(self, location: Point<Px>) -> Self {
        Self(self.0.at(location), self.1)
    }

    fn on_dismiss(self, callback: Callback) -> Self {
        Self(self.0.on_dismiss(callback), self.1)
    }

    fn show(self) -> Self::Handle {
        let handle = self.0.show();
        *self.1 .0.lock() = Some(handle);
        self.1
    }
}

/// A handle to a [`Menu`] that was shown.
#[derive(Clone, Debug)]
pub struct OpenMenuHandle(Dynamic<Option<OverlayHandle>>);

impl OpenMenuHandle {
    /// Closes the menu, if it is still shown.
    pub fn dismiss(&self) {
        *self.0.lock() = None;
    }
}

/// The selected item of a shown [`Menu<T>`].
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ChosenMenuItem<T> {
    /// The item that was chosen.
    pub item: T,
    /// Information about the button click that caused this item to be chosen,
    /// if present.
    pub click: Option<ButtonClick>,
}

/// A builder of a [`MenuItem<T>`].
pub struct MenuItemBuilder<T, Contents = ()> {
    value: T,
    contents: Contents,
}

impl<T> MenuItemBuilder<T, ()> {
    /// Sets the text of this menu item to `text` and returns self.
    pub fn text(self, text: impl Into<String>) -> MenuItemBuilder<T, String> {
        let Self {
            value,
            contents: (),
        } = self;

        MenuItemBuilder {
            value,
            contents: text.into(),
        }
    }

    /// Sets the contents of this menu item to `widget` and returns self.
    pub fn widget(self, widget: impl MakeWidget) -> MenuItemBuilder<T, WidgetInstance> {
        let Self {
            value,
            contents: (),
        } = self;

        MenuItemBuilder {
            value,
            contents: widget.make_widget(),
        }
    }
}

/// A type that can be used inside of a [`MenuItemBuilder`] as a menu item's
/// contents.
pub trait MenuItemContents<T>: sealed::MenuItemContentsSealed<T> {}

mod sealed {
    pub trait MenuItemContentsSealed<T> {
        fn make_item(self, value: T) -> super::MenuItem<T>;
    }
}

impl<T> MenuItemContents<T> for String {}
impl<T> MenuItemContents<T> for WidgetInstance {}
impl<T> sealed::MenuItemContentsSealed<T> for String {
    fn make_item(self, value: T) -> MenuItem<T> {
        MenuItem {
            value,
            widget: self.make_widget(),
        }
    }
}

impl<T> sealed::MenuItemContentsSealed<T> for WidgetInstance {
    fn make_item(self, value: T) -> MenuItem<T> {
        MenuItem {
            value,
            widget: self,
        }
    }
}

impl<T, Contents> MenuItemBuilder<T, Contents>
where
    Contents: MenuItemContents<T>,
{
    /// Returns the finished menu item.
    pub fn finish(self) -> MenuItem<T> {
        self.contents.make_item(self.value)
    }
}

impl<T, Contents> From<MenuItemBuilder<T, Contents>> for MenuItem<T>
where
    Contents: MenuItemContents<T>,
{
    fn from(builder: MenuItemBuilder<T, Contents>) -> Self {
        builder.finish()
    }
}

/// An item in a [`Menu<T>`].
#[derive(Debug, Clone)]
pub struct MenuItem<T> {
    value: T,
    widget: WidgetInstance,
    // submenu: Option<Menu<T>>,
}

impl<T> MenuItem<T> {
    /// Returns a new menu item with the given value and contents.
    pub fn new(value: T, contents: impl MakeWidget) -> Self {
        Self::build(value).widget(contents).finish()
    }

    /// Returns a builder for a menu item with the given value.
    pub fn build(value: T) -> MenuItemBuilder<T, ()> {
        MenuItemBuilder {
            value,
            contents: (),
        }
    }
}

#[derive(Debug)]
struct OpenMenu {
    items: Vec<OpenItem>,
    handle: OpenMenuHandle,
}

#[derive(Debug)]
struct OpenItem {
    contents: WidgetRef,
    height: UPx,
}

impl Widget for OpenMenu {
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        for item in &mut self.items {
            let mounted = item.contents.mounted(context);
            context.for_other(&mounted).redraw();
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let mut maximum_item_width = UPx::ZERO;
        let mut remaining_height = available_space.height.max();

        for item in &mut self.items {
            let mounted = item.contents.mounted(context);
            let size = context.for_other(&mounted).layout(Size::new(
                ConstraintLimit::SizeToFit(available_space.width.max()),
                ConstraintLimit::SizeToFit(remaining_height),
            ));
            item.height = size.height;

            remaining_height = remaining_height.saturating_sub(item.height);
            maximum_item_width = maximum_item_width.max(size.width);
        }

        let mut y = UPx::ZERO;
        for item in &mut self.items {
            let mounted = item.contents.mounted(context);
            context.set_child_layout(
                &mounted,
                Rect::new(
                    Point::new(Px::ZERO, y.into_signed()),
                    Size::new(maximum_item_width, item.height).into_signed(),
                ),
            );
            y += item.height;
        }

        Size::new(maximum_item_width, y)
    }

    fn accept_focus(&mut self, _context: &mut crate::context::EventContext<'_>) -> bool {
        true
    }

    fn mounted(&mut self, context: &mut crate::context::EventContext<'_>) {
        context.focus();
    }

    fn blur(&mut self, _context: &mut crate::context::EventContext<'_>) {
        self.handle.dismiss();
    }
}
