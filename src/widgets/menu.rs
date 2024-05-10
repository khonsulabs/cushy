//! Overlay menu widgets.

use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

use alot::LotId;
use figures::units::{Px, UPx};
use figures::{Angle, IntoSigned, Point, Rect, Round, ScreenScale, Size, Zero};
use kludgine::shapes::{PathBuilder, Shape};
use kludgine::DrawableExt;
use parking_lot::Mutex;

use self::sealed::{SharedMenuState, SubmenuFactory};
use super::button::{ButtonClick, ButtonColors, ButtonKind, VisualState};
use super::container::{self, ContainerShadow};
use super::disclose::IndicatorSize;
use super::layers::{OverlayBuilder, OverlayHandle, OverlayLayer, Overlayable};
use super::Button;
use crate::animation::{AnimationHandle, AnimationTarget, Spawn};
use crate::context::{EventContext, GraphicsContext, LayoutContext};
use crate::styles::components::{
    CornerRadius, Easing, IntrinsicPadding, OpaqueWidgetColor, TextColor,
};
use crate::styles::Styles;
use crate::value::{Dynamic, Source};
use crate::widget::{
    Callback, EventHandling, MakeWidget, MakeWidgetWithTag, Widget, WidgetId, WidgetInstance,
    WidgetRef, WidgetTag, HANDLED,
};
use crate::ConstraintLimit;

/// An overlayable menu of selectable items.
///
/// This widget is designed to implement Cushy's contextual menu system. When
/// used with an [`OverlayLayer`], this widget can be shown above other widgets
/// or at a specific location.
#[derive(Debug, Clone)]
pub struct Menu<T, Handler = MenuHandler<T>> {
    items: Vec<MenuItem<T>>,
    on_click: Handler,
}

impl<T> Default for Menu<T, ()>
where
    T: Debug + Send + Clone + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Menu<T, ()>
where
    T: Debug + Send + Clone + 'static,
{
    /// Returns a new, empty menu.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: Vec::new(),
            on_click: (),
        }
    }

    /// Sets the selected handler to `selected`, causing it to be invoked when
    /// an item is chosen.
    #[must_use]
    pub fn on_selected<F>(self, selected: F) -> Menu<T>
    where
        F: FnMut(ChosenMenuItem<T>) + Send + 'static,
    {
        Menu {
            items: self.items,
            on_click: MenuHandler(Arc::new(Mutex::new(Callback::new(selected)))),
        }
    }
}

impl<T, Handler> Menu<T, Handler>
where
    T: Debug + Send + Clone + 'static,
{
    /// Adds another menu `item` that is displayed using `widget`.
    #[must_use]
    pub fn with(mut self, item: impl Into<MenuItem<T>>) -> Self {
        self.items.push(item.into());
        self
    }
}

impl<T> Menu<T>
where
    T: Debug + Send + Clone + 'static,
{
    /// Presents this menu in `overlay`, returning an [`Overlayable`] that can
    /// be positioned relative or absolutely within `overlay`.
    #[must_use]
    pub fn overlay_in<'overlay>(&self, overlay: &'overlay OverlayLayer) -> MenuOverlay<'overlay> {
        self.overlay_in_shared(overlay, Dynamic::default())
    }

    fn overlay_in_shared<'overlay>(
        &self,
        overlay: &'overlay OverlayLayer,
        shared: Dynamic<SharedMenuState>,
    ) -> MenuOverlay<'overlay> {
        let Self { items, on_click } = self;
        let handle = OpenMenuHandle(Dynamic::new(None));
        let items = items
            .iter()
            .map(|item| {
                let MenuItem {
                    value,
                    widget,
                    submenu,
                } = item;
                OpenItem {
                    value: value.clone(),
                    contents: WidgetRef::new(widget.clone().align_left()),
                    y: UPx::ZERO,
                    height: UPx::ZERO,
                    submenu: submenu.clone(),
                    colors: None,
                    color_animation: AnimationHandle::default(),
                    state: VisualState::Normal,
                }
            })
            .collect();

        let root_menu = shared.lock().open_menus.push(handle.clone());

        let (menu_tag, menu_id) = WidgetTag::new();
        MenuOverlay(
            overlay.build_overlay(
                OpenMenu {
                    on_click: on_click.clone(),
                    items,
                    open_id: root_menu,
                    padding: UPx::ZERO,
                    selecting: None,
                    mouse_down: false,
                    layer: overlay.clone(),
                    open_submenu: None,
                    menu_id,
                    disclosure_size: UPx::ZERO,
                    shared,
                }
                .vertical_scroll()
                .make_with_tag(menu_tag),
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

    fn parent(self, id: crate::widget::WidgetId) -> Self {
        Self(self.0.parent(id), self.1)
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
    submenu: Option<Arc<dyn SubmenuFactory>>,
    contents: Contents,
}

impl<T> MenuItemBuilder<T, ()> {
    /// Sets the text of this menu item to `text` and returns self.
    pub fn text(self, text: impl Into<String>) -> MenuItemBuilder<T, String> {
        let Self {
            value,
            submenu,
            contents: (),
        } = self;

        MenuItemBuilder {
            value,
            submenu,
            contents: text.into(),
        }
    }

    /// Sets the contents of this menu item to `widget` and returns self.
    pub fn widget(self, widget: impl MakeWidget) -> MenuItemBuilder<T, WidgetInstance> {
        let Self {
            value,
            submenu,
            contents: (),
        } = self;

        MenuItemBuilder {
            value,
            submenu,
            contents: widget.make_widget(),
        }
    }
}

/// A type that can be used inside of a [`MenuItemBuilder`] as a menu item's
/// contents.
pub trait MenuItemContents<T>: sealed::MenuItemContentsSealed<T> {}

mod sealed {
    use std::sync::Arc;

    use alot::OrderedLots;
    use kempt::Set;

    use super::{MenuOverlay, OpenMenuHandle};
    use crate::value::Dynamic;
    use crate::widget::WidgetId;
    use crate::widgets::layers::OverlayLayer;

    pub trait SubmenuFactory: Send + Sync + 'static {
        fn overlay_submenu_in<'overlay>(
            &self,
            overlay: &'overlay OverlayLayer,
            shared_state: Dynamic<SharedMenuState>,
        ) -> MenuOverlay<'overlay>;
    }

    pub trait MenuItemContentsSealed<T> {
        fn make_item(
            self,
            value: T,
            submenu: Option<Arc<dyn SubmenuFactory>>,
        ) -> super::MenuItem<T>;
    }

    #[derive(Debug, Default)]
    pub struct SharedMenuState {
        pub open_menus: OrderedLots<OpenMenuHandle>,
        pub hovering: Set<WidgetId>,
    }
}

impl<T> MenuItemContents<T> for String {}
impl<T> MenuItemContents<T> for WidgetInstance {}
impl<T> sealed::MenuItemContentsSealed<T> for String {
    fn make_item(self, value: T, submenu: Option<Arc<dyn SubmenuFactory>>) -> MenuItem<T> {
        MenuItem {
            value,
            widget: self.make_widget(),
            submenu,
        }
    }
}

impl<T> sealed::MenuItemContentsSealed<T> for WidgetInstance {
    fn make_item(self, value: T, submenu: Option<Arc<dyn SubmenuFactory>>) -> MenuItem<T> {
        MenuItem {
            value,
            widget: self,
            submenu,
        }
    }
}

impl<T> sealed::SubmenuFactory for Menu<T>
where
    T: Clone + std::fmt::Debug + Send + Sync + 'static,
{
    fn overlay_submenu_in<'overlay>(
        &self,
        overlay: &'overlay OverlayLayer,
        shared_state: Dynamic<SharedMenuState>,
    ) -> MenuOverlay<'overlay> {
        self.overlay_in_shared(overlay, shared_state)
    }
}

impl<T, Contents> MenuItemBuilder<T, Contents>
where
    Contents: MenuItemContents<T>,
{
    /// Attaches a submenu to this item and returns self.
    #[must_use]
    pub fn submenu<U>(mut self, submenu: Menu<U>) -> Self
    where
        U: Clone + Debug + Send + Sync + 'static,
    {
        self.submenu = Some(Arc::new(submenu));
        self
    }

    /// Returns the finished menu item.
    pub fn finish(self) -> MenuItem<T> {
        self.contents.make_item(self.value, self.submenu)
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
#[derive(Clone)]
pub struct MenuItem<T> {
    value: T,
    widget: WidgetInstance,
    submenu: Option<Arc<dyn SubmenuFactory>>,
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
            submenu: None,
            contents: (),
        }
    }
}

impl<T> Debug for MenuItem<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuItem")
            .field("value", &self.value)
            .field("widget", &self.widget)
            .field("submenu", &self.submenu.is_some())
            .finish()
    }
}

/// A handler for a [`ChosenMenuItem<T>`].
#[derive(Debug, Clone)]
pub struct MenuHandler<T>(Arc<Mutex<Callback<ChosenMenuItem<T>>>>);

#[derive(Debug)]
struct OpenMenu<T> {
    items: Vec<OpenItem<T>>,
    on_click: MenuHandler<T>,
    open_id: LotId,
    padding: UPx,
    selecting: Option<usize>,
    mouse_down: bool,
    layer: OverlayLayer,
    open_submenu: Option<(usize, OpenMenuHandle)>,
    menu_id: WidgetId,
    disclosure_size: UPx,
    shared: Dynamic<SharedMenuState>,
}
impl<T> OpenMenu<T> {
    fn handle_mouse_movement(&mut self, location: Point<Px>, context: &mut EventContext<'_>) {
        self.selecting = None;
        for (index, item) in self.items.iter_mut().enumerate() {
            let hovered = location.y >= item.y - self.padding
                && location.y < item.y + item.height + self.padding;
            let new_state = if hovered {
                self.selecting = Some(index);
                if let Some((submenu_index, handle)) = &self.open_submenu {
                    if *submenu_index != index {
                        context.focus();
                        handle.dismiss();
                        self.open_submenu = None;
                    }
                } else if let Some(factory) = &item.submenu {
                    let last_layout = context.last_layout().expect("must have rendered");
                    let menu_location = Point::new(
                        last_layout.origin.x + last_layout.size.width
                            - self.padding.into_signed() * 2,
                        last_layout.origin.y + (item.y - self.padding).into_signed(),
                    );
                    self.open_submenu = Some((
                        index,
                        factory
                            .overlay_submenu_in(&self.layer, self.shared.clone())
                            .parent(self.menu_id)
                            .at(menu_location)
                            .show(),
                    ));
                }
                if self.mouse_down {
                    VisualState::Active
                } else {
                    VisualState::Hovered
                }
            } else {
                VisualState::Normal
            };
            if item.state != new_state {
                item.state = new_state;
                let new_colors = if hovered {
                    ButtonKind::Solid.colors_for_default(new_state, context)
                } else {
                    Button::colors_for_transparent(new_state, context)
                };
                if let Some(colors) = &item.colors {
                    item.color_animation = colors
                        .transition_to(new_colors)
                        .over(Duration::from_millis(150))
                        .with_easing(context.get(&Easing))
                        .spawn();
                } else {
                    item.colors = Some(Dynamic::new(new_colors));
                    context.set_needs_redraw();
                }
            }
        }
    }
}

impl<T> Widget for OpenMenu<T>
where
    T: Clone + Debug + Send + 'static,
{
    fn redraw(&mut self, context: &mut GraphicsContext<'_, '_, '_, '_>) {
        let radii = context.get(&CornerRadius);
        let radii = radii.map(|r| r.into_px(context.gfx.scale()));
        let bg = context.get(&OpaqueWidgetColor);
        let full_size = context.gfx.size();
        let content_rect = Rect::new(
            Point::new(self.padding, UPx::ZERO),
            Size::new(
                full_size.width - self.padding * 2,
                full_size.height - self.padding,
            ),
        )
        .into_signed();
        container::render_shadow(
            &content_rect,
            radii,
            &ContainerShadow::new(Point::ZERO)
                .blur_radius(self.padding.into_signed())
                .spread(self.padding.into_signed()),
            bg,
            context,
        );
        let bg_shape = if radii.is_zero() {
            Shape::filled_rect(content_rect, bg)
        } else {
            Shape::filled_round_rect(content_rect, radii, bg)
        };
        context.gfx.draw_shape(&bg_shape);
        let disclosure_size = (self.disclosure_size.into_signed() / 2).round();
        let pt1 = Point::new(disclosure_size, Px::ZERO).rotate_by(Angle::degrees(0));
        let pt2 = Point::new(disclosure_size, Px::ZERO).rotate_by(Angle::degrees(120));
        let pt3 = Point::new(disclosure_size, Px::ZERO).rotate_by(Angle::degrees(240));

        let submenu = PathBuilder::new(pt1).line_to(pt2).line_to(pt3).close();
        for item in &mut self.items {
            let mounted = item.contents.mounted(context);

            if let Some(colors) = &item.colors {
                let colors = colors.get_tracking_redraw(context);
                let child_rect = Rect::new(
                    Point::new(self.padding, item.y - self.padding),
                    Size::new(
                        full_size.width - self.padding * 2,
                        item.height + self.padding * 2,
                    ),
                )
                .into_signed();

                let bg_shape = if radii.is_zero() {
                    Shape::filled_rect(child_rect, colors.background)
                } else {
                    Shape::filled_round_rect(child_rect, radii, colors.background)
                };
                context.gfx.draw_shape(&bg_shape);

                if item.submenu.is_some() {
                    let disclosure_offset = Point::new(
                        full_size.width - self.disclosure_size / 2 - self.padding * 2,
                        item.y + item.height / 2,
                    )
                    .into_signed();
                    context.gfx.draw_shape(
                        submenu
                            .fill(colors.foreground)
                            .translate_by(disclosure_offset),
                    );
                }

                let mut context = context.for_other(&mounted);
                context.attach_styles(Styles::new().with(&TextColor, colors.foreground));
                context.redraw();
            }
        }
    }

    fn layout(
        &mut self,
        available_space: Size<ConstraintLimit>,
        context: &mut LayoutContext<'_, '_, '_, '_>,
    ) -> Size<UPx> {
        let mut maximum_item_width = UPx::ZERO;
        let mut remaining_height = available_space.height.max();
        self.padding = context.get(&IntrinsicPadding).into_upx(context.gfx.scale());
        self.disclosure_size =
            (context.get(&IndicatorSize).into_upx(context.gfx.scale()) / 2).round();
        let double_padding = self.padding * 2;
        let submenu_space = if self.items.iter().any(|i| i.submenu.is_some()) {
            self.padding + self.disclosure_size
        } else {
            UPx::ZERO
        };
        let available_width = available_space.width.max() - double_padding;

        let mut y = self.padding;
        for item in &mut self.items {
            let mounted = item.contents.mounted(context);
            let available_width = available_width - submenu_space;
            let size = context.for_other(&mounted).layout(Size::new(
                ConstraintLimit::SizeToFit(available_width),
                ConstraintLimit::SizeToFit(remaining_height),
            ));
            item.y = y;
            item.height = size.height;
            let full_height = size.height + double_padding;
            y += full_height;

            remaining_height = remaining_height.saturating_sub(full_height);
            maximum_item_width = maximum_item_width.max(size.width);
        }

        for item in &mut self.items {
            let mounted = item.contents.mounted(context);
            context.set_child_layout(
                &mounted,
                Rect::new(
                    Point::new(double_padding, item.y),
                    Size::new(maximum_item_width, item.height),
                )
                .into_signed(),
            );
        }

        Size::new(maximum_item_width + double_padding * 2 + submenu_space, y)
    }

    fn hit_test(
        &mut self,
        _location: Point<Px>,
        _context: &mut crate::context::EventContext<'_>,
    ) -> bool {
        true
    }

    fn hover(
        &mut self,
        location: Point<Px>,
        context: &mut crate::context::EventContext<'_>,
    ) -> Option<kludgine::app::winit::window::CursorIcon> {
        self.handle_mouse_movement(location, context);
        self.shared.lock().hovering.insert(context.widget().id());
        None
    }

    fn unhover(&mut self, context: &mut EventContext<'_>) {
        let mut shared = self.shared.lock();
        shared.hovering.remove(&context.widget().id());
        if shared.hovering.is_empty() {
            drop(shared);
            self.handle_mouse_movement(Point::squared(Px::new(-1)), context);
        }
    }

    fn mouse_down(
        &mut self,
        location: Point<Px>,
        _device_id: crate::window::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut crate::context::EventContext<'_>,
    ) -> EventHandling {
        self.mouse_down = true;
        self.handle_mouse_movement(location, context);

        HANDLED
    }

    fn mouse_drag(
        &mut self,
        location: Point<Px>,
        _device_id: crate::window::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        context: &mut crate::context::EventContext<'_>,
    ) {
        self.handle_mouse_movement(location, context);
    }

    fn mouse_up(
        &mut self,
        _location: Option<Point<Px>>,
        _device_id: crate::window::DeviceId,
        _button: kludgine::app::winit::event::MouseButton,
        _context: &mut crate::context::EventContext<'_>,
    ) {
        if let Some(index) = self.selecting {
            self.on_click.0.lock().invoke(ChosenMenuItem {
                item: self.items[index].value.clone(),
                click: None,
            });
            let mut shared = self.shared.lock();
            for handle in shared.open_menus.drain() {
                handle.dismiss();
            }
        }
        self.mouse_down = false;
    }

    fn accept_focus(&mut self, _context: &mut crate::context::EventContext<'_>) -> bool {
        true
    }

    fn mounted(&mut self, context: &mut crate::context::EventContext<'_>) {
        context.focus();

        let colors = Button::colors_for_transparent(VisualState::Normal, context);
        for item in &mut self.items {
            item.colors = Some(Dynamic::new(colors));
        }
    }

    fn blur(&mut self, _context: &mut crate::context::EventContext<'_>) {
        if self.open_submenu.is_none() {
            let mut shared = self.shared.lock();
            if let Some(index) = shared.open_menus.index_of_id(self.open_id) {
                while shared.open_menus.len() > index {
                    let Some(handle) = shared.open_menus.pop() else {
                        unreachable!()
                    };
                    handle.dismiss();
                }
            }
        }
    }
}

struct OpenItem<T> {
    value: T,
    contents: WidgetRef,
    submenu: Option<Arc<dyn SubmenuFactory>>,
    y: UPx,
    height: UPx,
    colors: Option<Dynamic<ButtonColors>>,
    color_animation: AnimationHandle,
    state: VisualState,
}

impl<T> Debug for OpenItem<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenItem")
            .field("value", &self.value)
            .field("contents", &self.contents)
            .field("submenu", &self.submenu.is_some())
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}
