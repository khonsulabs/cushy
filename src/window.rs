//! Types for displaying a [`Widget`](crate::widget::Widget) inside of a desktop
//! window.

use std::cell::RefCell;
use std::collections::hash_map;
use std::ffi::OsStr;
use std::hash::Hash;
use std::io;
use std::marker::PhantomData;
use std::num::{NonZeroU32, TryFromIntError};
use std::ops::{Deref, DerefMut, Not};
use std::path::Path;
use std::string::ToString;
use std::sync::{mpsc, Arc, OnceLock};
use std::time::{Duration, Instant};

use ahash::AHashMap;
use alot::LotId;
use arboard::Clipboard;
use figures::units::{Px, UPx};
use figures::{
    FloatConversion, Fraction, IntoSigned, IntoUnsigned, Point, Ranged, Rect, Round, ScreenScale,
    Size, UPx2D, Zero,
};
use image::{DynamicImage, RgbImage, RgbaImage};
use intentional::{Assert, Cast};
use kludgine::app::winit::dpi::{PhysicalPosition, PhysicalSize};
use kludgine::app::winit::event::{
    ElementState, Ime, Modifiers, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::app::winit::keyboard::{
    Key, KeyLocation, NamedKey, NativeKeyCode, PhysicalKey, SmolStr,
};
use kludgine::app::winit::window::{self, Cursor, Fullscreen, Icon, WindowButtons, WindowLevel};
use kludgine::app::{winit, WindowAttributes, WindowBehavior as _};
use kludgine::cosmic_text::{fontdb, Family, FamilyOwned};
use kludgine::drawing::Drawing;
use kludgine::shapes::Shape;
use kludgine::wgpu::{self, CompositeAlphaMode, COPY_BYTES_PER_ROW_ALIGNMENT};
use kludgine::{Color, DrawableExt, Kludgine, KludgineId, Origin, Texture};
use parking_lot::{Mutex, MutexGuard};
use sealed::Ize;
use tracing::Level;
use unicode_segmentation::UnicodeSegmentation;

use crate::animation::{
    AnimationTarget, Easing, LinearInterpolate, PercentBetween, Spawn, ZeroToOne,
};
use crate::app::{Application, Cushy, Open, PendingApp, Run};
use crate::context::sealed::{InvalidationStatus, Trackable as _};
use crate::context::{
    AsEventContext, EventContext, Exclusive, GraphicsContext, LayoutContext, Trackable,
    WidgetContext,
};
use crate::fonts::FontCollection;
use crate::graphics::{FontState, Graphics};
use crate::styles::{Edges, FontFamilyList, ThemePair};
use crate::tree::Tree;
use crate::utils::ModifiersExt;
use crate::value::{
    Destination, Dynamic, DynamicReader, IntoDynamic, IntoValue, Source, Tracked, Value,
};
use crate::widget::{
    EventHandling, MakeWidget, MountedWidget, OnceCallback, RootBehavior, SharedCallback, WidgetId,
    WidgetInstance, HANDLED, IGNORED,
};
use crate::window::sealed::WindowCommand;
use crate::{initialize_tracing, App, ConstraintLimit};

/// A platform-dependent window implementation.
pub trait PlatformWindowImplementation {
    /// Marks the window to close as soon as possible.
    fn close(&mut self);
    /// Returns the underlying `winit` window, if one exists.
    fn winit(&self) -> Option<&winit::window::Window>;
    /// Sets the window to redraw as soon as possible.
    fn set_needs_redraw(&mut self);
    /// Sets the window to redraw after a `duration`.
    fn redraw_in(&mut self, duration: Duration);
    /// Sets the window to redraw at a specified instant.
    fn redraw_at(&mut self, moment: Instant);
    /// Returns the current keyboard modifiers.
    fn modifiers(&self) -> Modifiers;
    /// Returns the amount of time that has elapsed since the last redraw.
    fn elapsed(&self) -> Duration;
    /// Sets the current cursor icon to `cursor`.
    fn set_cursor(&mut self, cursor: Cursor);
    /// Returns a handle for the window.
    fn handle(&self, redraw_status: InvalidationStatus) -> WindowHandle;
    /// Returns the current outer position of the window.
    fn outer_position(&self) -> Point<Px> {
        self.winit().map_or_else(Point::default, |w| {
            w.outer_position().unwrap_or_default().into()
        })
    }
    /// Returns the current inner position of the window.
    fn inner_position(&self) -> Point<Px> {
        self.winit().map_or_else(Point::default, |w| {
            w.inner_position().unwrap_or_default().into()
        })
    }
    /// Returns the current inner size of the window.
    fn inner_size(&self) -> Size<UPx>;
    /// Returns the current outer size of the window.
    fn outer_size(&self) -> Size<UPx> {
        self.winit()
            .map_or_else(|| self.inner_size(), |w| w.outer_size().into())
    }

    /// Returns true if the window can have its size changed.
    ///
    /// The provided implementation returns
    /// [`winit::window::Window::is_resizable`], or true if this window has no
    /// winit window.
    fn is_resizable(&self) -> bool {
        self.winit()
            .map_or(true, winit::window::Window::is_resizable)
    }

    /// Returns true if the window can have its size changed.
    ///
    /// The provided implementation returns [`winit::window::Window::theme`], or
    /// dark if this window has no winit window.
    fn theme(&self) -> winit::window::Theme {
        self.winit()
            .and_then(winit::window::Window::theme)
            .unwrap_or(winit::window::Theme::Dark)
    }

    /// Requests that the window change its inner size.
    ///
    /// The provided implementation forwards the request onto the winit window,
    /// if present.
    fn request_inner_size(&mut self, inner_size: Size<UPx>) {
        self.winit()
            .map(|winit| winit.request_inner_size(PhysicalSize::from(inner_size)));
    }

    /// Sets whether [`Ime`] events should be enabled.
    ///
    /// The provided implementation forwards the request onto the winit window,
    /// if present.
    fn set_ime_allowed(&self, allowed: bool) {
        if let Some(winit) = self.winit() {
            winit.set_ime_allowed(allowed);
        }
    }
    /// Sets the location of the cursor.
    fn set_ime_location(&self, location: Rect<Px>) {
        if let Some(winit) = self.winit() {
            winit.set_ime_cursor_area(
                PhysicalPosition::from(location.origin),
                PhysicalSize::from(location.size),
            );
        }
    }

    /// Sets the current [`Ime`] purpose.
    ///
    /// The provided implementation forwards the request onto the winit window,
    /// if present.
    fn set_ime_purpose(&self, purpose: winit::window::ImePurpose) {
        if let Some(winit) = self.winit() {
            winit.set_ime_purpose(purpose);
        }
    }

    /// Sets the window's minimum inner size.
    fn set_min_inner_size(&self, min_size: Option<Size<UPx>>) {
        if let Some(winit) = self.winit() {
            winit.set_min_inner_size::<PhysicalSize<u32>>(min_size.map(Into::into));
        }
    }

    /// Sets the window's maximum inner size.
    fn set_max_inner_size(&self, max_size: Option<Size<UPx>>) {
        if let Some(winit) = self.winit() {
            winit.set_max_inner_size::<PhysicalSize<u32>>(max_size.map(Into::into));
        }
    }

    /// Ensures that this window will be redrawn when `value` has been updated.
    fn redraw_when_changed(&self, value: &impl Trackable, invalidation_status: &InvalidationStatus)
    where
        Self: Sized,
    {
        value.inner_redraw_when_changed(self.handle(invalidation_status.clone()));
    }
}

impl PlatformWindowImplementation for kludgine::app::Window<'_, WindowCommand> {
    fn set_cursor(&mut self, cursor: Cursor) {
        self.winit().set_cursor(cursor);
    }

    fn inner_size(&self) -> Size<UPx> {
        self.winit().inner_size().into()
    }

    fn close(&mut self) {
        self.close();
    }

    fn winit(&self) -> Option<&winit::window::Window> {
        Some(self.winit())
    }

    fn set_needs_redraw(&mut self) {
        self.set_needs_redraw();
    }

    fn redraw_in(&mut self, duration: Duration) {
        self.redraw_in(duration);
    }

    fn redraw_at(&mut self, moment: Instant) {
        self.redraw_at(moment);
    }

    fn modifiers(&self) -> Modifiers {
        self.modifiers()
    }

    fn elapsed(&self) -> Duration {
        self.elapsed()
    }

    fn handle(&self, redraw_status: InvalidationStatus) -> WindowHandle {
        WindowHandle::new(self.handle(), redraw_status)
    }
}

/// A platform-dependent window.
pub trait PlatformWindow {
    /// Marks the window to close as soon as possible.
    fn close(&mut self);
    /// Returns a handle for the window.
    fn handle(&self) -> WindowHandle;
    /// Returns the unique id of the [`Kludgine`] instance used by this window.
    fn kludgine_id(&self) -> KludgineId;
    /// Returns the dynamic that is synchrnoized with the window's focus.
    fn focused(&self) -> &Dynamic<bool>;
    /// Returns the dynamic that is synchronized with the window's occlusion
    /// status.
    fn occluded(&self) -> &Dynamic<bool>;
    /// Returns the current inner size of the window.
    fn inner_size(&self) -> &Dynamic<Size<UPx>>;
    /// Returns the shared application resources.
    fn cushy(&self) -> &Cushy;
    /// Sets the window to redraw as soon as possible.
    fn set_needs_redraw(&mut self);
    /// Sets the window to redraw after a `duration`.
    fn redraw_in(&mut self, duration: Duration);
    /// Sets the window to redraw at a specified instant.
    fn redraw_at(&mut self, moment: Instant);
    /// Returns the current keyboard modifiers.
    fn modifiers(&self) -> Modifiers;
    /// Returns the amount of time that has elapsed since the last redraw.
    fn elapsed(&self) -> Duration;
    /// Sets the current cursor icon to `cursor`.
    fn set_cursor(&mut self, cursor: Cursor);

    /// Sets the location of the cursor.
    fn set_ime_location(&self, location: Rect<Px>);
    /// Sets whether [`Ime`] events should be enabled.
    fn set_ime_allowed(&self, allowed: bool);
    /// Sets the current [`Ime`] purpose.
    fn set_ime_purpose(&self, purpose: winit::window::ImePurpose);

    /// Requests that the window change its inner size.
    fn request_inner_size(&mut self, inner_size: Size<UPx>);
    /// Sets the window's minimum inner size.
    fn set_min_inner_size(&self, min_size: Option<Size<UPx>>);
    /// Sets the window's maximum inner size.
    fn set_max_inner_size(&self, max_size: Option<Size<UPx>>);

    /// Returns a handle to the underlying winit window, if available.
    fn winit(&self) -> Option<&winit::window::Window>;
}

/// A currently running Cushy window.
pub struct RunningWindow<W> {
    window: W,
    kludgine_id: KludgineId,
    invalidation_status: InvalidationStatus,
    cushy: Cushy,
    focused: Dynamic<bool>,
    occluded: Dynamic<bool>,
    inner_size: Dynamic<Size<UPx>>,
    close_requested: Option<SharedCallback<(), bool>>,
}

impl<W> RunningWindow<W>
where
    W: PlatformWindowImplementation,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        window: W,
        kludgine_id: KludgineId,
        invalidation_status: &InvalidationStatus,
        cushy: &Cushy,
        focused: &Dynamic<bool>,
        occluded: &Dynamic<bool>,
        inner_size: &Dynamic<Size<UPx>>,
        close_requested: &Option<SharedCallback<(), bool>>,
    ) -> Self {
        Self {
            window,
            kludgine_id,
            invalidation_status: invalidation_status.clone(),
            cushy: cushy.clone(),
            focused: focused.clone(),
            occluded: occluded.clone(),
            inner_size: inner_size.clone(),
            close_requested: close_requested.clone(),
        }
    }

    /// Returns the [`KludgineId`] of this window.
    ///
    /// Each window has its own unique `KludgineId`.
    #[must_use]
    pub const fn kludgine_id(&self) -> KludgineId {
        self.kludgine_id
    }

    /// Returns a dynamic that is updated whenever this window's focus status
    /// changes.
    #[must_use]
    pub const fn focused(&self) -> &Dynamic<bool> {
        &self.focused
    }

    /// Returns a dynamic that is updated whenever this window's occlusion
    /// status changes.
    #[must_use]
    pub const fn occluded(&self) -> &Dynamic<bool> {
        &self.occluded
    }

    /// Request that the window closes.
    ///
    /// A window may disallow itself from being closed by customizing
    /// [`WindowBehavior::close_requested`].
    pub fn request_close(&self) {
        self.handle().request_close();
    }

    /// Returns a handle to this window.
    #[must_use]
    pub fn handle(&self) -> WindowHandle {
        self.window.handle(self.invalidation_status.clone())
    }

    /// Returns a dynamic that is synchronized with this window's inner size.
    ///
    /// Whenever the window is resized, this dynamic will be updated with the
    /// new inner size. Setting a new value will request the new size from the
    /// operating system, but resize requests may be altered or ignored by the
    /// operating system.
    #[must_use]
    pub const fn inner_size(&self) -> &Dynamic<Size<UPx>> {
        &self.inner_size
    }

    /// Returns a locked mutex guard to the OS's clipboard, if one was able to be
    /// initialized when the window opened.
    #[must_use]
    pub fn clipboard_guard(&self) -> Option<MutexGuard<'_, Clipboard>> {
        self.cushy.clipboard_guard()
    }
}

impl<W> Deref for RunningWindow<W>
where
    W: PlatformWindowImplementation + 'static,
{
    type Target = dyn PlatformWindowImplementation;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl<W> DerefMut for RunningWindow<W>
where
    W: PlatformWindowImplementation + 'static,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}

impl<W> PlatformWindow for RunningWindow<W>
where
    W: PlatformWindowImplementation,
{
    fn close(&mut self) {
        self.window.close();
    }

    fn handle(&self) -> WindowHandle {
        self.window.handle(self.invalidation_status.clone())
    }

    fn kludgine_id(&self) -> KludgineId {
        self.kludgine_id
    }

    fn focused(&self) -> &Dynamic<bool> {
        &self.focused
    }

    fn occluded(&self) -> &Dynamic<bool> {
        &self.occluded
    }

    fn inner_size(&self) -> &Dynamic<Size<UPx>> {
        &self.inner_size
    }

    fn cushy(&self) -> &Cushy {
        &self.cushy
    }

    fn set_needs_redraw(&mut self) {
        self.window.set_needs_redraw();
    }

    fn redraw_in(&mut self, duration: Duration) {
        self.window.redraw_in(duration);
    }

    fn redraw_at(&mut self, moment: Instant) {
        self.window.redraw_at(moment);
    }

    fn modifiers(&self) -> Modifiers {
        self.window.modifiers()
    }

    fn elapsed(&self) -> Duration {
        self.window.elapsed()
    }

    fn set_ime_allowed(&self, allowed: bool) {
        self.window.set_ime_allowed(allowed);
    }

    fn set_ime_purpose(&self, purpose: winit::window::ImePurpose) {
        self.window.set_ime_purpose(purpose);
    }

    fn set_cursor(&mut self, cursor: Cursor) {
        self.window.set_cursor(cursor);
    }

    fn set_min_inner_size(&self, min_size: Option<Size<UPx>>) {
        self.window.set_min_inner_size(min_size);
    }

    fn set_max_inner_size(&self, max_size: Option<Size<UPx>>) {
        self.window.set_max_inner_size(max_size);
    }

    fn request_inner_size(&mut self, inner_size: Size<UPx>) {
        self.window.request_inner_size(inner_size);
    }

    fn set_ime_location(&self, location: Rect<Px>) {
        self.window.set_ime_location(location);
    }

    fn winit(&self) -> Option<&winit::window::Window> {
        self.window.winit()
    }
}

/// A Cushy window that is not yet running.
#[must_use]
pub struct Window<Behavior = WidgetInstance>
where
    Behavior: WindowBehavior,
{
    /// The title to display in the title bar of the window.
    pub title: Value<String>,
    /// The colors to use to theme the user interface.
    pub theme: Value<ThemePair>,
    /// When true, the system fonts will be loaded into the font database. This
    /// is on by default.
    pub load_system_fonts: bool,
    /// The list of font families to try to find when a [`FamilyOwned::Serif`]
    /// font is requested.
    pub serif_font_family: FontFamilyList,
    /// The list of font families to try to find when a
    /// [`FamilyOwned::SansSerif`] font is requested.
    pub sans_serif_font_family: FontFamilyList,
    /// The list of font families to try to find when a [`FamilyOwned::Fantasy`]
    /// font is requested.
    pub fantasy_font_family: FontFamilyList,
    /// The list of font families to try to find when a
    /// [`FamilyOwned::Monospace`] font is requested.
    pub monospace_font_family: FontFamilyList,
    /// The list of font families to try to find when a [`FamilyOwned::Cursive`]
    /// font is requested.
    pub cursive_font_family: FontFamilyList,
    /// A collection of fonts that this window will load.
    pub fonts: FontCollection,
    /// When true, Cushy will try to use "vertical sync" to try to eliminate
    /// graphical tearing that can occur if the graphics card has a new frame
    /// presented while the monitor is currently rendering another frame.
    ///
    /// Under the hood, Cushy uses `wgpu::PresentMode::AutoVsync` when true and
    /// `wgpu::PresentMode::AutoNoVsync` when false.
    pub vsync: bool,
    /// The number of samples to perform for each pixel rendered to the screen.
    /// When 1, multisampling is disabled.
    pub multisample_count: NonZeroU32,
    /// Resizes the window to fit the contents if true.
    pub resize_to_fit: Value<bool>,

    context: Behavior::Context,
    pending: PendingWindow,
    attributes: WindowAttributes,
    on_closed: Option<OnceCallback>,
    on_open: Option<OnceCallback<WindowHandle>>,
    inner_size: Option<Dynamic<Size<UPx>>>,
    zoom: Option<Dynamic<Fraction>>,
    occluded: Option<Dynamic<bool>>,
    focused: Option<Dynamic<bool>>,
    theme_mode: Option<Value<ThemeMode>>,
    content_protected: Option<Value<bool>>,
    cursor_hittest: Option<Value<bool>>,
    cursor_visible: Option<Value<bool>>,
    cursor_position: Option<Dynamic<Point<Px>>>,
    window_level: Option<Value<WindowLevel>>,
    decorated: Option<Value<bool>>,
    maximized: Option<Dynamic<bool>>,
    minimized: Option<Dynamic<bool>>,
    resizable: Option<Value<bool>>,
    resize_increments: Option<Value<Size<UPx>>>,
    visible: Option<Dynamic<bool>>,
    outer_size: Option<Dynamic<Size<UPx>>>,
    inner_position: Option<Dynamic<Point<Px>>>,
    outer_position: Option<Dynamic<Point<Px>>>,
    close_requested: Option<SharedCallback<(), bool>>,
    icon: Option<Value<Option<RgbaImage>>>,
    modifiers: Option<Dynamic<Modifiers>>,
    enabled_buttons: Option<Value<WindowButtons>>,
    fullscreen: Option<Value<Option<Fullscreen>>>,
}

impl<Behavior> Default for Window<Behavior>
where
    Behavior: WindowBehavior,
    Behavior::Context: Default,
{
    fn default() -> Self {
        Self::new(Behavior::Context::default())
    }
}

impl Window {
    /// Returns a new instance using `widget` as its contents.
    pub fn for_widget<W>(widget: W) -> Self
    where
        W: MakeWidget,
    {
        Self::new(widget.make_widget())
    }
}

impl<Behavior> Window<Behavior>
where
    Behavior: WindowBehavior,
{
    /// Returns a new instance using `context` to initialize the window upon
    /// opening.
    pub fn new(context: Behavior::Context) -> Self {
        Self::new_with_pending(context, PendingWindow::default())
    }

    fn new_with_pending(context: Behavior::Context, pending: PendingWindow) -> Self {
        static EXECUTABLE_NAME: OnceLock<String> = OnceLock::new();

        let title = EXECUTABLE_NAME
            .get_or_init(|| {
                std::env::args_os()
                    .next()
                    .and_then(|path| {
                        Path::new(&path)
                            .file_name()
                            .and_then(OsStr::to_str)
                            .map(ToString::to_string)
                    })
                    .unwrap_or_else(|| String::from("Cushy App"))
            })
            .clone();
        Self {
            pending,
            title: Value::Constant(title),
            attributes: WindowAttributes::default(),
            on_open: None,
            on_closed: None,
            context,
            load_system_fonts: true,
            theme: Value::default(),
            occluded: None,
            focused: None,
            theme_mode: None,
            inner_size: None,
            serif_font_family: FontFamilyList::default(),
            sans_serif_font_family: FontFamilyList::default(),
            fantasy_font_family: FontFamilyList::default(),
            monospace_font_family: FontFamilyList::default(),
            cursive_font_family: FontFamilyList::default(),
            fonts: {
                let fonts = FontCollection::default();
                #[cfg(feature = "roboto-flex")]
                fonts.push(include_bytes!("../assets/RobotoFlex.ttf").to_vec());
                fonts
            },
            multisample_count: NonZeroU32::new(4).assert("not 0"),
            vsync: true,
            close_requested: None,
            zoom: None,
            resize_to_fit: Value::Constant(false),
            content_protected: None,
            cursor_hittest: None,
            cursor_visible: None,
            cursor_position: None,
            window_level: None,
            decorated: None,
            maximized: None,
            minimized: None,
            resizable: None,
            resize_increments: None,
            visible: None,
            outer_size: None,
            inner_position: None,
            outer_position: None,
            icon: None,
            modifiers: None,
            enabled_buttons: None,
            fullscreen: None,
        }
    }

    /// Returns the handle to this window.
    pub const fn handle(&self) -> &WindowHandle {
        &self.pending.0
    }

    fn center_on_open(&mut self, app: App) {
        // We want to ensure that if the user has customized any of these
        // properties that we keep their dynamic.
        let outer_position = self.outer_position.clone().unwrap_or_else(|| {
            let outer_position = Dynamic::new(Point::default());
            self.outer_position = Some(outer_position.clone());
            outer_position
        });
        let outer_size = self.outer_size.clone().unwrap_or_else(|| {
            let outer_size = Dynamic::new(Size::default());
            self.outer_size = Some(outer_size.clone());
            outer_size
        });
        let visible = self.visible.clone().unwrap_or_else(|| {
            let visible = Dynamic::new(false);
            self.visible = Some(visible.clone());
            visible
        });
        visible.set(false);

        let callback_handle = Dynamic::new(None);
        callback_handle.set(Some(outer_size.for_each_subsequent({
            let visible = visible.clone();
            let callback_handle = callback_handle.clone();
            move |new_size| {
                if let Some(monitor) = app.monitors().and_then(|monitors| {
                    let initial_position = outer_position.get();
                    monitors
                        .available
                        .into_iter()
                        .find(|m| m.region().contains(initial_position))
                        .or(monitors.primary)
                }) {
                    let region = monitor.region();
                    let margin = region.size - new_size.into_signed();
                    outer_position.set(region.origin + margin / 2);
                }
                visible.set(true);
                // Uninstall this callback to ensure it doesn't fire again.
                let _ = callback_handle.take();
            }
        })));
    }

    /// Opens `self` in the center of the monitor the window initially appears
    /// on.
    pub fn open_centered<App>(mut self, app: &mut App) -> crate::Result<WindowHandle>
    where
        App: Application + ?Sized,
    {
        self.center_on_open(app.as_app());

        self.open(app)
    }

    /// Sets `focused` to be the dynamic updated when this window's focus status
    /// is changed.
    ///
    /// When the window is focused for user input, the dynamic will contain
    /// `true`.
    ///
    /// The current value of `focused` will inform the OS whether the window
    /// should be activated upon opening. To prevent state mismatches, if
    /// `focused` is a dynamic, it will be initialized with `false` so that the
    /// transition to focused can be observed.
    pub fn focused(mut self, focused: impl IntoValue<bool>) -> Self {
        let focused = focused.into_value();
        self.attributes.active = focused.get();
        if let Value::Dynamic(focused) = focused {
            focused.set(false);
            self.focused = Some(focused);
        }
        self
    }

    /// Sets `occluded` to be the dynamic updated when this window's occlusion
    /// status is changed.
    ///
    /// When the window is occluded (completely hidden/offscreen/minimized), the
    /// dynamic will contain `true`. If the window is at least partially
    /// visible, this value will contain `true`.
    ///
    /// `occluded` will be initialized with an initial state of `false`.
    pub fn occluded(mut self, occluded: impl IntoDynamic<bool>) -> Self {
        let occluded = occluded.into_dynamic();
        occluded.set(false);
        self.occluded = Some(occluded);
        self
    }

    /// Sets the full screen mode for this window.
    pub fn fullscreen(mut self, fullscreen: impl IntoValue<Option<Fullscreen>>) -> Self {
        let fullscreen = fullscreen.into_value();
        self.attributes.fullscreen = fullscreen.get();
        self.fullscreen = Some(fullscreen);
        self
    }

    /// Sets `inner_size` to be the dynamic synchronized with this window's
    /// inner size.
    ///
    /// When the window is resized, the dynamic will contain its new size. When
    /// the dynamic is updated with a new value, a resize request will be made
    /// with the new inner size.
    pub fn inner_size(mut self, inner_size: impl IntoDynamic<Size<UPx>>) -> Self {
        let inner_size = inner_size.into_dynamic();
        let initial_size = inner_size.get();
        if initial_size.width > 0 && initial_size.height > 0 {
            self.attributes.inner_size = Some(winit::dpi::Size::Physical(initial_size.into()));
        }
        self.inner_size = Some(inner_size);
        self
    }

    /// Sets `outer_size` to be a dynamic synchronized with this window's size,
    /// including decorations.
    ///
    /// When the window is resized, the dynamic will contain its new size.
    /// Setting this dynamic with a new value does not change the window in any
    /// way. To resize the window, use [`inner_size`](Self::inner_size).
    pub fn outer_size(mut self, outer_size: impl IntoDynamic<Size<UPx>>) -> Self {
        self.outer_size = Some(outer_size.into_dynamic());
        self
    }

    /// Sets `position`  to be a dynamic synchronized with this window's outer
    /// position.
    ///
    /// If `automatic_layout` is true, the initial value of `position` will be
    /// ignored and the window server will control the window's initial
    /// position.
    ///
    /// When the window is moved, this dynamic will contain its new position.
    /// Setting this dynamic will attempt to move the window to the provided
    /// location.
    pub fn outer_position(
        mut self,
        position: impl IntoValue<Point<Px>>,
        automatic_layout: bool,
    ) -> Self {
        let position = position.into_value();

        if let Some(initial_position) = automatic_layout.then(|| position.get()) {
            self.attributes.position =
                Some(winit::dpi::Position::Physical(initial_position.into()));
        }

        if let Value::Dynamic(position) = position {
            self.outer_position = Some(position);
        }

        self
    }

    /// Sets `position`  to be a dynamic synchronized with this window's inner
    /// position.
    ///
    /// When the window is moved, this dynamic will contain its new position.
    /// Setting this dynamic to a new value has no effect. To move a window, use
    /// [`outer_position`](Self::outer_position).
    pub fn inner_position(mut self, position: impl IntoDynamic<Point<Px>>) -> Self {
        self.inner_position = Some(position.into_dynamic());
        self
    }

    /// Resizes this window to fit the contents when `resize_to_fit` is true.
    pub fn resize_to_fit(mut self, resize_to_fit: impl IntoValue<bool>) -> Self {
        self.resize_to_fit = resize_to_fit.into_value();
        self
    }

    /// Prevents the window contents from being captured by other apps.
    pub fn content_protected(mut self, protected: impl IntoValue<bool>) -> Self {
        let protected = protected.into_value();
        self.attributes.content_protected = protected.get();
        self.content_protected = Some(protected);
        self
    }

    /// Controls whether the cursor should interact with this window or not.
    pub fn cursor_hittest(mut self, hittest: impl IntoValue<bool>) -> Self {
        self.cursor_hittest = Some(hittest.into_value());
        self
    }

    /// Sets whether the cursor is visible when above this window.
    pub fn cursor_visible(mut self, visible: impl IntoValue<bool>) -> Self {
        self.cursor_visible = Some(visible.into_value());
        self
    }

    /// A dynamic providing access to the window coordinate of the cursor, or
    /// -1, -1 if the cursor is not currently hovering the window.
    ///
    /// In the future, this dynamic will also support setting the position of
    /// the cursor within the window.
    pub fn cursor_position(mut self, window_position: impl IntoDynamic<Point<Px>>) -> Self {
        self.cursor_position = Some(window_position.into_dynamic());
        self
    }

    /// Controls whether window decorations are shown around this window.
    pub fn decorated(mut self, decorated: impl IntoValue<bool>) -> Self {
        let decorated = decorated.into_value();
        self.attributes.decorations = decorated.get();
        self.decorated = Some(decorated);
        self
    }

    /// Sets the enabled buttons for this window.
    pub fn enabled_buttons(mut self, buttons: impl IntoValue<WindowButtons>) -> Self {
        let buttons = buttons.into_value();
        self.attributes.enabled_buttons = buttons.get();
        self.enabled_buttons = Some(buttons);
        self
    }

    /// Controls the level of this window.
    pub fn window_level(mut self, window_level: impl IntoValue<WindowLevel>) -> Self {
        let window_level = window_level.into_value();
        self.attributes.window_level = window_level.get();
        self.window_level = Some(window_level);
        self
    }

    /// Provides a dynamic that is updated with the minimized status of this
    /// window.
    pub fn minimized(mut self, minimized: impl IntoDynamic<bool>) -> Self {
        self.minimized = Some(minimized.into_dynamic());
        self
    }

    /// Provides a dynamic that is updated with the maximized status of this
    /// window.
    pub fn maximized(mut self, maximized: impl IntoDynamic<bool>) -> Self {
        let maximized = maximized.into_dynamic();
        self.attributes.maximized = maximized.get();
        self.maximized = Some(maximized);
        self
    }

    /// Controls whether the window is resizable by the user or not.
    pub fn resizable(mut self, resizable: impl IntoValue<bool>) -> Self {
        let resizable = resizable.into_value();
        self.attributes.resizable = resizable.get();
        self.resizable = Some(resizable);
        self
    }

    /// Controls the increments in which the window can be resized.
    pub fn resize_increments(mut self, resize_increments: impl IntoValue<Size<UPx>>) -> Self {
        self.resize_increments = Some(resize_increments.into_value());
        self
    }

    /// Sets this window to render with a transparent background.
    pub fn transparent(mut self) -> Self {
        self.attributes.transparent = true;
        self
    }

    /// Controls the visibility of this window.
    pub fn visible(mut self, visible: impl IntoDynamic<bool>) -> Self {
        let visible = visible.into_dynamic();
        self.attributes.visible = visible.get();
        self.visible = Some(visible);
        self
    }

    /// Sets this window's `zoom` factor.
    ///
    /// The zoom factor is multiplied with the DPI scaling from the window
    /// server to allow an additional scaling factor to be applied.
    pub fn zoom(mut self, zoom: impl IntoDynamic<Fraction>) -> Self {
        self.zoom = Some(zoom.into_dynamic().map_each_into());
        self
    }

    /// Sets the [`ThemeMode`] for this window.
    ///
    /// If a [`ThemeMode`] is provided, the window will be set to this theme
    /// mode upon creation and will not be updated while the window is running.
    ///
    /// If a [`Dynamic`] is provided, the initial value will be ignored and the
    /// dynamic will be updated when the window opens with the user's current
    /// theme mode. The dynamic will also be updated any time the user's theme
    /// mode changes.
    ///
    /// Setting the [`Dynamic`]'s value will also update the window with the new
    /// mode until a mode change is detected, upon which the new mode will be
    /// stored.
    pub fn themed_mode(mut self, theme_mode: impl IntoValue<ThemeMode>) -> Self {
        self.theme_mode = Some(theme_mode.into_value());
        self
    }

    /// Applies `theme` to the widgets in this window.
    pub fn themed(mut self, theme: impl IntoValue<ThemePair>) -> Self {
        self.theme = theme.into_value();
        self
    }

    /// Adds `font_data` to the list of fonts to load for availability when
    /// rendering.
    ///
    /// All font families contained in `font_data` will be loaded.
    pub fn loading_font(self, font_data: Vec<u8>) -> Self {
        self.fonts.push(font_data);
        self
    }

    /// Invokes `on_open` when this window is first opened, even if it is not
    /// visible.
    pub fn on_open<Function>(mut self, on_open: Function) -> Self
    where
        Function: FnOnce(WindowHandle) + Send + 'static,
    {
        self.on_open = Some(OnceCallback::new(on_open));
        self
    }

    /// Invokes `on_close` when this window is closed.
    pub fn on_close<Function>(mut self, on_close: Function) -> Self
    where
        Function: FnOnce() + Send + 'static,
    {
        self.on_closed = Some(OnceCallback::new(|()| on_close()));
        self
    }

    /// Invokes `on_close_requested` when the window is requested to be closed.
    ///
    /// If the function returns true, the window is allowed to be closed,
    /// otherwise the window remains open.
    pub fn on_close_requested<Function>(mut self, on_close_requested: Function) -> Self
    where
        Function: FnMut(()) -> bool + Send + 'static,
    {
        self.close_requested = Some(SharedCallback::new(on_close_requested));
        self
    }

    /// Sets the window's title.
    pub fn titled(mut self, title: impl IntoValue<String>) -> Self {
        self.title = title.into_value();
        self
    }

    /// Sets the window's icon.
    pub fn icon(mut self, icon: impl IntoValue<Option<RgbaImage>>) -> Self {
        self.icon = Some(icon.into_value());
        self
    }

    /// Sets `modifiers` to contain the state of the keyboard modifiers when
    /// this window has keyboard focus.
    pub fn modifiers(mut self, modifiers: impl IntoDynamic<Modifiers>) -> Self {
        self.modifiers = Some(modifiers.into_dynamic());
        self
    }

    /// Sets the name of the application.
    ///
    /// - `WM_CLASS` on X11
    /// - application ID on wayland
    /// - class name on windows
    pub fn app_name(mut self, name: String) -> Self {
        self.attributes.app_name = Some(name);
        self
    }
}

impl<Behavior> Run for Window<Behavior>
where
    Behavior: WindowBehavior,
{
    fn run(self) -> crate::Result {
        initialize_tracing();
        let mut app = PendingApp::default();
        self.open(&mut app)?;
        app.run()
    }
}

impl<T> Open for T
where
    T: MakeWindow,
{
    fn open<App>(self, app: &mut App) -> crate::Result<WindowHandle>
    where
        App: Application + ?Sized,
    {
        let this = self.make_window();
        let cushy = app.cushy().clone();
        let handle = this.pending.handle();
        OpenWindow::<T::Behavior>::open_with(
            app,
            sealed::Context {
                user: this.context,
                settings: RefCell::new(sealed::WindowSettings {
                    cushy,
                    title: this.title,
                    redraw_status: this.pending.0.redraw_status.clone(),
                    on_open: this.on_open,
                    on_closed: this.on_closed,
                    transparent: this.attributes.transparent,
                    attributes: Some(this.attributes),
                    occluded: this.occluded.unwrap_or_default(),
                    focused: this.focused.unwrap_or_default(),
                    inner_size: this.inner_size.unwrap_or_default(),
                    theme: Some(this.theme),
                    theme_mode: this.theme_mode,
                    font_data_to_load: this.fonts,
                    serif_font_family: this.serif_font_family,
                    sans_serif_font_family: this.sans_serif_font_family,
                    fantasy_font_family: this.fantasy_font_family,
                    monospace_font_family: this.monospace_font_family,
                    cursive_font_family: this.cursive_font_family,
                    vsync: this.vsync,
                    multisample_count: this.multisample_count,
                    close_requested: this.close_requested,
                    zoom: this.zoom.unwrap_or_else(|| Dynamic::new(Fraction::ONE)),
                    resize_to_fit: this.resize_to_fit,
                    content_protected: this.content_protected.unwrap_or_default(),
                    cursor_hittest: this.cursor_hittest.unwrap_or_else(|| Value::Constant(true)),
                    cursor_visible: this.cursor_visible.unwrap_or_else(|| Value::Constant(true)),
                    cursor_position: this.cursor_position.unwrap_or_default(),
                    window_level: this.window_level.unwrap_or_default(),
                    decorated: this.decorated.unwrap_or_else(|| Value::Constant(true)),
                    maximized: this.maximized.unwrap_or_default(),
                    minimized: this.minimized.unwrap_or_default(),
                    resizable: this.resizable.unwrap_or_else(|| Value::Constant(true)),
                    resize_increments: this.resize_increments.unwrap_or_default(),
                    visible: this.visible.unwrap_or_default(),
                    inner_position: this.inner_position.unwrap_or_default(),
                    outer_position: this.outer_position.unwrap_or_default(),
                    outer_size: this.outer_size.unwrap_or_default(),
                    window_icon: this.icon.unwrap_or_default(),
                    modifiers: this.modifiers.unwrap_or_default(),
                    enabled_buttons: this
                        .enabled_buttons
                        .unwrap_or(Value::Constant(WindowButtons::all())),
                    fullscreen: this.fullscreen.unwrap_or_default(),
                }),
                pending: this.pending,
            },
        )?;

        Ok(handle)
    }

    fn run_in(self, mut app: PendingApp) -> crate::Result {
        self.open(&mut app)?;
        app.run()
    }
}

/// A type that can be made into a [`Window`].
pub trait MakeWindow {
    /// The behavior associated with this window.
    type Behavior: WindowBehavior;

    /// Returns a new window from `self`.
    fn make_window(self) -> Window<Self::Behavior>;

    /// Opens `self` in the center of the monitor the window initially appears
    /// on.
    fn open_centered<App>(self, app: &mut App) -> crate::Result<WindowHandle>
    where
        Self: Sized,
        App: Application + ?Sized,
    {
        self.make_window().open_centered(app)
    }

    /// Runs `self` in the center of the monitor the window
    /// initially appears on.
    fn run_centered(self) -> crate::Result
    where
        Self: Sized,
    {
        self.make_window().run()
    }

    /// Runs `app` after opening `self` in the center of the monitor the window
    /// initially appears on.
    fn run_centered_in(self, mut app: PendingApp) -> crate::Result
    where
        Self: Sized,
    {
        self.make_window().open_centered(&mut app)?;
        app.run()
    }
}

impl<Behavior> MakeWindow for Window<Behavior>
where
    Behavior: WindowBehavior,
{
    type Behavior = Behavior;

    fn make_window(self) -> Window<Self::Behavior> {
        self
    }
}

impl<T> MakeWindow for T
where
    T: MakeWidget,
{
    type Behavior = WidgetInstance;

    fn make_window(self) -> Window<Self::Behavior> {
        Window::for_widget(self.make_widget())
    }
}

/// The behavior of a Cushy window.
pub trait WindowBehavior: Sized + 'static {
    /// The type that is provided when initializing this window.
    type Context: Send + 'static;

    /// Return a new instance of this behavior using `context`.
    fn initialize(
        window: &mut RunningWindow<kludgine::app::Window<'_, WindowCommand>>,
        context: Self::Context,
    ) -> Self;

    /// Create the window's root widget. This function is only invoked once.
    fn make_root(&mut self) -> WidgetInstance;

    /// The window has been requested to close. If this function returns true,
    /// the window will be closed. Returning false prevents the window from
    /// closing.
    #[allow(unused_variables)]
    fn close_requested<W>(&self, window: &mut W) -> bool
    where
        W: PlatformWindow,
    {
        true
    }

    /// Runs this behavior as an application.
    fn run() -> crate::Result
    where
        Self::Context: Default,
    {
        Self::run_with(<Self::Context>::default())
    }

    /// Runs this behavior as an application, initialized with `context`.
    fn run_with(context: Self::Context) -> crate::Result {
        Window::<Self>::new(context).run()
    }
}

#[allow(clippy::struct_excessive_bools)]
struct OpenWindow<T> {
    behavior: T,
    tree: Tree,
    root: MountedWidget,
    contents: Drawing,
    should_close: bool,
    cursor: CursorState,
    mouse_buttons: AHashMap<DeviceId, AHashMap<MouseButton, WidgetId>>,
    redraw_status: InvalidationStatus,
    initial_frame: bool,
    occluded: Dynamic<bool>,
    focused: Dynamic<bool>,
    inner_size: Tracked<Dynamic<Size<UPx>>>,
    outer_size: Dynamic<Size<UPx>>,
    keyboard_activated: Option<WidgetId>,
    min_inner_size: Option<Size<UPx>>,
    max_inner_size: Option<Size<UPx>>,
    resize_to_fit: Value<bool>,
    theme: Option<DynamicReader<ThemePair>>,
    current_theme: ThemePair,
    theme_mode: Value<ThemeMode>,
    transparent: bool,
    fonts: FontState,
    cushy: Cushy,
    on_closed: Option<OnceCallback>,
    vsync: bool,
    dpi_scale: Dynamic<Fraction>,
    zoom: Tracked<Dynamic<Fraction>>,
    close_requested: Option<SharedCallback<(), bool>>,
    content_protected: Tracked<Value<bool>>,
    cursor_hittest: Tracked<Value<bool>>,
    cursor_visible: Tracked<Value<bool>>,
    cursor_position: Tracked<Dynamic<Point<Px>>>,
    window_level: Tracked<Value<WindowLevel>>,
    decorated: Tracked<Value<bool>>,
    maximized: Tracked<Dynamic<bool>>,
    minimized: Tracked<Dynamic<bool>>,
    resizable: Tracked<Value<bool>>,
    resize_increments: Tracked<Value<Size<UPx>>>,
    visible: Tracked<Dynamic<bool>>,
    outer_position: Tracked<Dynamic<Point<Px>>>,
    inner_position: Dynamic<Point<Px>>,
    window_icon: Tracked<Value<Option<RgbaImage>>>,
    enabled_buttons: Tracked<Value<WindowButtons>>,
    fullscreen: Tracked<Value<Option<Fullscreen>>>,
    modifiers: Dynamic<Modifiers>,
}

impl<T> OpenWindow<T>
where
    T: WindowBehavior,
{
    fn request_close(
        should_close: &mut bool,
        behavior: &mut T,
        window: &mut RunningWindow<kludgine::app::Window<'_, WindowCommand>>,
    ) -> bool {
        *should_close |= behavior.close_requested(window)
            && window
                .close_requested
                .as_ref()
                .map_or(true, |close| close.invoke(()));

        *should_close
    }

    fn keyboard_activate_widget<W>(
        &mut self,
        is_pressed: bool,
        widget: Option<LotId>,
        window: &mut W,
        kludgine: &mut Kludgine,
    ) where
        W: PlatformWindow,
    {
        if is_pressed {
            if let Some(default) = widget.and_then(|id| self.tree.widget_from_node(id)) {
                if let Some(previously_active) = self
                    .keyboard_activated
                    .take()
                    .and_then(|id| self.tree.widget(id))
                {
                    EventContext::new(
                        WidgetContext::new(
                            previously_active,
                            &self.current_theme,
                            window,
                            &mut self.fonts,
                            self.theme_mode.get(),
                            &mut self.cursor,
                        ),
                        kludgine,
                    )
                    .deactivate();
                }
                EventContext::new(
                    WidgetContext::new(
                        default.clone(),
                        &self.current_theme,
                        window,
                        &mut self.fonts,
                        self.theme_mode.get(),
                        &mut self.cursor,
                    ),
                    kludgine,
                )
                .activate();
                self.keyboard_activated = Some(default.id());
            }
        } else if let Some(keyboard_activated) = self
            .keyboard_activated
            .take()
            .and_then(|id| self.tree.widget(id))
        {
            EventContext::new(
                WidgetContext::new(
                    keyboard_activated,
                    &self.current_theme,
                    window,
                    &mut self.fonts,
                    self.theme_mode.get(),
                    &mut self.cursor,
                ),
                kludgine,
            )
            .deactivate();
        }
    }

    fn constrain_window_resizing<W>(
        &mut self,
        resizable: bool,
        window: &mut RunningWindow<W>,
        graphics: &mut kludgine::Graphics<'_>,
    ) -> RootMode
    where
        W: PlatformWindowImplementation,
    {
        let mut root_or_child = self.root.widget.clone();
        let mut root_mode = None;
        let mut padding = Edges::<Px>::default();

        loop {
            let Some(managed) = self.tree.widget(root_or_child.id()) else {
                break;
            };

            let mut context = EventContext::new(
                WidgetContext::new(
                    managed,
                    &self.current_theme,
                    window,
                    &mut self.fonts,
                    self.theme_mode.get(),
                    &mut self.cursor,
                ),
                graphics,
            );
            let mut widget = root_or_child.lock();
            match widget.as_widget().root_behavior(&mut context) {
                Some((behavior, child)) => {
                    let child = child.clone();
                    match behavior {
                        RootBehavior::PassThrough => {}
                        RootBehavior::Expand => {
                            root_mode = root_mode.or(Some(RootMode::Expand));
                        }
                        RootBehavior::Align => {
                            root_mode = root_mode.or(Some(RootMode::Align));
                        }
                        RootBehavior::Pad(edges) => {
                            padding += edges.into_px(context.kludgine.scale());
                        }
                        RootBehavior::Resize(range) => {
                            let padding = padding.size();
                            let min_width = range
                                .width
                                .minimum()
                                .map_or(Px::ZERO, |width| width.into_px(context.kludgine.scale()))
                                .saturating_add(padding.width);
                            let max_width = range
                                .width
                                .maximum()
                                .map_or(Px::MAX, |width| width.into_px(context.kludgine.scale()))
                                .saturating_add(padding.width);
                            let min_height = range
                                .height
                                .minimum()
                                .map_or(Px::ZERO, |height| height.into_px(context.kludgine.scale()))
                                .saturating_add(padding.height);
                            let max_height = range
                                .height
                                .maximum()
                                .map_or(Px::MAX, |height| height.into_px(context.kludgine.scale()))
                                .saturating_add(padding.height);

                            let new_min_size = (min_width > 0 || min_height > 0)
                                .then_some(Size::new(min_width, min_height).into_unsigned());

                            if new_min_size != self.min_inner_size && resizable {
                                context.set_min_inner_size(new_min_size);
                                self.min_inner_size = new_min_size;
                            }
                            let new_max_size = (max_width > 0 || max_height > 0)
                                .then_some(Size::new(max_width, max_height).into_unsigned());

                            if new_max_size != self.max_inner_size && resizable {
                                context.set_max_inner_size(new_max_size);
                            }
                            self.max_inner_size = new_max_size;

                            break;
                        }
                    }
                    drop(widget);

                    root_or_child = child.clone();
                }
                None => break,
            }
        }

        root_mode.unwrap_or(RootMode::Fit)
    }

    fn load_fonts(
        settings: &mut sealed::WindowSettings,
        app_fonts: FontCollection,
        fontdb: &mut fontdb::Database,
    ) -> FontState {
        let fonts = FontState::new(fontdb, settings.font_data_to_load.clone(), app_fonts);
        fonts.apply_font_family_list(
            &settings.serif_font_family,
            || default_family(Family::Serif),
            |name| fontdb.set_serif_family(name),
        );

        fonts.apply_font_family_list(
            &settings.sans_serif_font_family,
            || {
                let bundled_font_name;
                #[cfg(feature = "roboto-flex")]
                {
                    bundled_font_name = Some(String::from("Roboto Flex"));
                }
                #[cfg(not(feature = "roboto-flex"))]
                {
                    bundled_font_name = None;
                }

                bundled_font_name.map_or_else(
                    || default_family(Family::SansSerif),
                    |name| Some(FamilyOwned::Name(name)),
                )
            },
            |name| fontdb.set_sans_serif_family(name),
        );
        fonts.apply_font_family_list(
            &settings.fantasy_font_family,
            || default_family(Family::Fantasy),
            |name| fontdb.set_fantasy_family(name),
        );
        fonts.apply_font_family_list(
            &settings.monospace_font_family,
            || default_family(Family::Monospace),
            |name| fontdb.set_monospace_family(name),
        );
        fonts.apply_font_family_list(
            &settings.cursive_font_family,
            || default_family(Family::Cursive),
            |name| fontdb.set_cursive_family(name),
        );
        fonts
    }

    fn handle_window_keyboard_input<W>(
        &mut self,
        window: &mut W,
        kludgine: &mut Kludgine,
        input: KeyEvent,
    ) -> EventHandling
    where
        W: PlatformWindow,
    {
        match input.logical_key {
            Key::Character(ch) if ch == "w" && window.modifiers().primary() => {
                if !input.repeat
                    && input.state.is_pressed()
                    && self.behavior.close_requested(window)
                {
                    self.should_close = true;
                    window.set_needs_redraw();
                }
                HANDLED
            }
            Key::Named(NamedKey::Space) if !window.modifiers().possible_shortcut() => {
                let target = self.tree.focused_widget().unwrap_or(self.root.node_id);
                let target = self.tree.widget_from_node(target).expect("missing widget");
                let mut target = EventContext::new(
                    WidgetContext::new(
                        target,
                        &self.current_theme,
                        window,
                        &mut self.fonts,
                        self.theme_mode.get(),
                        &mut self.cursor,
                    ),
                    kludgine,
                );

                match input.state {
                    ElementState::Pressed => {
                        if target.active() {
                            target.deactivate();
                            target.apply_pending_state();
                        }
                        target.activate();
                    }
                    ElementState::Released => {
                        target.deactivate();
                    }
                }
                HANDLED
            }

            Key::Named(NamedKey::Tab) if !window.modifiers().possible_shortcut() => {
                if input.state.is_pressed() {
                    let reverse = window.modifiers().state().shift_key();

                    let target = self.tree.focused_widget().unwrap_or(self.root.node_id);
                    let target = self.tree.widget_from_node(target).expect("missing widget");
                    let mut target = EventContext::new(
                        WidgetContext::new(
                            target,
                            &self.current_theme,
                            window,
                            &mut self.fonts,
                            self.theme_mode.get(),
                            &mut self.cursor,
                        ),
                        kludgine,
                    );

                    if reverse {
                        target.return_focus();
                    } else {
                        target.advance_focus();
                    }
                }
                HANDLED
            }
            Key::Named(NamedKey::Enter) => {
                self.keyboard_activate_widget(
                    input.state.is_pressed(),
                    self.tree.default_widget(),
                    window,
                    kludgine,
                );
                HANDLED
            }
            Key::Named(NamedKey::Escape) => {
                self.keyboard_activate_widget(
                    input.state.is_pressed(),
                    self.tree.escape_widget(),
                    window,
                    kludgine,
                );
                HANDLED
            }
            _ => {
                tracing::event!(
                    Level::DEBUG,
                    logical = ?input.logical_key,
                    physical = ?input.physical_key,
                    state = ?input.state,
                    "Ignored Keyboard Input",
                );
                IGNORED
            }
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn new<W>(
        mut behavior: T,
        mut window: W,
        graphics: &mut kludgine::Graphics<'_>,
        mut settings: sealed::WindowSettings,
    ) -> Self
    where
        W: PlatformWindowImplementation,
    {
        let redraw_status = settings.redraw_status.clone();
        if let Value::Dynamic(title) = &settings.title {
            let handle = window.handle(redraw_status.clone());
            title
                .for_each_cloned(move |title| {
                    handle.inner.send(WindowCommand::SetTitle(title));
                })
                .persist();
        }

        let cushy = settings.cushy.clone();
        let fonts = Self::load_fonts(
            &mut settings,
            cushy.fonts.clone(),
            graphics.font_system().db_mut(),
        );

        let dpi_scale = Dynamic::new(graphics.dpi_scale());
        settings.inner_position.set(window.inner_position());
        settings.outer_position.set(window.outer_position());

        let theme_mode = match settings.theme_mode.take() {
            Some(Value::Dynamic(dynamic)) => {
                dynamic.set(window.theme().into());
                Value::Dynamic(dynamic)
            }
            Some(Value::Constant(mode)) => Value::Constant(mode),
            None => Value::dynamic(window.theme().into()),
        };

        let tree = Tree::default();
        let root = tree.push_boxed(behavior.make_root(), None);

        let theme = settings.theme.unwrap_or_default();
        let (current_theme, theme) = match theme {
            Value::Constant(theme) => (theme, None),
            Value::Dynamic(dynamic) => (dynamic.get(), Some(dynamic.into_reader())),
        };

        if let Some(on_open) = settings.on_open {
            let handle = window.handle(redraw_status.clone());
            on_open.invoke(handle);
        }

        let mut this = Self {
            behavior,
            root,
            tree,
            contents: Drawing::default(),
            should_close: false,
            cursor: CursorState {
                location: None,
                widget: None,
            },
            mouse_buttons: AHashMap::default(),
            redraw_status,
            initial_frame: true,
            occluded: settings.occluded,
            focused: settings.focused,
            inner_size: Tracked::from(settings.inner_size).ignoring_first(),
            keyboard_activated: None,
            min_inner_size: None,
            max_inner_size: None,
            resize_to_fit: settings.resize_to_fit,
            current_theme,
            theme,
            theme_mode,
            transparent: settings.transparent,
            fonts,
            cushy,
            on_closed: settings.on_closed,
            vsync: settings.vsync,
            close_requested: settings.close_requested,
            dpi_scale,
            zoom: Tracked::from(settings.zoom),
            content_protected: Tracked::from(settings.content_protected).ignoring_first(),
            cursor_hittest: Tracked::from(settings.cursor_hittest),
            cursor_visible: Tracked::from(settings.cursor_visible),
            cursor_position: Tracked::from(settings.cursor_position),
            window_level: Tracked::from(settings.window_level).ignoring_first(),
            decorated: Tracked::from(settings.decorated).ignoring_first(),
            maximized: Tracked::from(settings.maximized),
            minimized: Tracked::from(settings.minimized),
            resizable: Tracked::from(settings.resizable).ignoring_first(),
            resize_increments: Tracked::from(settings.resize_increments),
            visible: Tracked::from(settings.visible).ignoring_first(),
            outer_size: settings.outer_size,
            inner_position: settings.inner_position,
            outer_position: Tracked::from(settings.outer_position).ignoring_first(),
            window_icon: Tracked::from(settings.window_icon),
            modifiers: settings.modifiers,
            enabled_buttons: Tracked::from(settings.enabled_buttons).ignoring_first(),
            fullscreen: Tracked::from(settings.fullscreen).ignoring_first(),
        };

        this.synchronize_platform_window(&mut window);
        this.prepare(window, graphics);

        this
    }

    fn new_frame(&mut self, graphics: &mut kludgine::Graphics<'_>) {
        if let Some(theme) = &mut self.theme {
            if theme.has_updated() {
                self.current_theme = theme.get();
                self.root.invalidate();
            }
        }

        self.redraw_status.refresh_received();
        graphics.reset_text_attributes();
        if let Some(zoom) = self.zoom.updated() {
            graphics.set_zoom(*zoom);
            self.redraw_status.invalidate(self.root.id());
        }

        self.tree
            .new_frame(self.redraw_status.invalidations().drain());
    }

    fn prepare<W>(&mut self, mut window: W, graphics: &mut kludgine::Graphics<'_>)
    where
        W: PlatformWindowImplementation,
    {
        let cushy = self.cushy.clone();
        let _guard = cushy.enter_runtime();

        self.synchronize_platform_window(&mut window);
        self.new_frame(graphics);

        let resize_to_fit = self.resize_to_fit.get();
        let resizable = window.is_resizable() || resize_to_fit;
        let mut window = RunningWindow::new(
            window,
            graphics.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            self.inner_size.source(),
            &self.close_requested,
        );
        let root_mode = self.constrain_window_resizing(resizable, &mut window, graphics);

        let fonts_changed = self.fonts.next_frame(graphics.font_system().db_mut());
        if fonts_changed {
            graphics.rebuild_font_system();
        }
        let graphics = self.contents.new_frame(graphics);
        let mut context = GraphicsContext {
            widget: WidgetContext::new(
                self.root.clone(),
                &self.current_theme,
                &mut window,
                &mut self.fonts,
                self.theme_mode.get(),
                &mut self.cursor,
            ),
            gfx: Exclusive::Owned(Graphics::new(graphics)),
        };
        if self.initial_frame {
            self.root
                .lock()
                .as_widget()
                .mounted(&mut context.as_event_context());
        }
        self.theme_mode.redraw_when_changed(&context);
        let mut layout_context = LayoutContext::new(&mut context);
        let window_size = layout_context.gfx.size();

        if !self.transparent {
            let background_color = layout_context.theme().surface.color;
            layout_context.graphics.gfx.fill(background_color);
        }

        let layout_size =
            layout_context.layout(if matches!(root_mode, RootMode::Expand | RootMode::Align) {
                window_size.map(ConstraintLimit::Fill)
            } else {
                window_size.map(ConstraintLimit::SizeToFit)
            });
        let actual_size = if root_mode == RootMode::Align {
            window_size.max(layout_size)
        } else {
            layout_size
        };
        let render_size = actual_size.min(window_size);
        layout_context.invalidate_when_changed(&self.inner_size);
        layout_context.invalidate_when_changed(&self.resize_to_fit);
        if let Some(new_size) = self.inner_size.updated() {
            layout_context.request_inner_size(*new_size);
        } else if actual_size != window_size && !resizable {
            let mut new_size = actual_size;
            if let Some(min_size) = self.min_inner_size {
                new_size = new_size.max(min_size);
            }
            if let Some(max_size) = self.max_inner_size {
                new_size = new_size.min(max_size);
            }
            layout_context.request_inner_size(new_size);
        } else if resize_to_fit && window_size != layout_size {
            layout_context.request_inner_size(layout_size);
        }
        self.root.set_layout(Rect::from(render_size.into_signed()));

        if self.initial_frame {
            self.initial_frame = false;
            self.root
                .lock()
                .as_widget()
                .mounted(&mut layout_context.as_event_context());
            layout_context.focus();
            layout_context.as_event_context().apply_pending_state();
        }

        if render_size.width < window_size.width || render_size.height < window_size.height {
            layout_context
                .clipped_to(Rect::from(render_size.into_signed()))
                .redraw();
        } else {
            layout_context.redraw();
        }
    }

    fn close_requested<W>(&mut self, window: W, kludgine: &mut Kludgine) -> bool
    where
        W: PlatformWindowImplementation,
    {
        let cushy = self.cushy.clone();
        let _guard = cushy.enter_runtime();
        if self.behavior.close_requested(&mut RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            self.inner_size.source(),
            &self.close_requested,
        )) {
            self.should_close = true;
            true
        } else {
            false
        }
    }

    fn resized<W>(&mut self, new_size: Size<UPx>, window: &W)
    where
        W: PlatformWindowImplementation,
    {
        self.inner_size.set_and_read(new_size);
        self.outer_size.set(window.outer_size());
        self.update_ized(window);
        self.root.invalidate();
    }

    fn moved(&mut self, new_inner_position: Point<Px>, new_outer_position: Point<Px>) {
        self.outer_position.set_and_read(new_outer_position);
        self.inner_position.set(new_inner_position);
    }

    fn update_ized<W>(&mut self, window: &W)
    where
        W: PlatformWindowImplementation,
    {
        if let Some(winit) = window.winit() {
            // TODO should these be supported outside of winit? Put in a feature
            // request if you read this and need them.
            self.maximized.set_and_read(winit.is_maximized());
            if let Some(minimized) = winit.is_minimized() {
                self.minimized.set_and_read(minimized);
            }
            self.decorated.set_and_read(winit.is_decorated());
        }
    }

    fn synchronize_platform_window<W>(&mut self, window: &mut W)
    where
        W: PlatformWindowImplementation,
    {
        macro_rules! when_updated {
            ($prop:ident, $handle:ident, $block:expr) => {
                self.$prop.inner_sync_when_changed($handle.clone());
                if let Some($prop) = self.$prop.updated() {
                    $block
                }
            };
        }
        self.redraw_status.sync_received();
        self.update_ized(window);
        if let Some(winit) = window.winit() {
            let mut redraw = false;
            let handle = window.handle(self.redraw_status.clone());

            when_updated!(outer_position, handle, {
                winit.set_outer_position(PhysicalPosition::<i32>::from(*outer_position));
            });
            when_updated!(content_protected, handle, {
                winit.set_content_protected(*content_protected);
            });
            when_updated!(cursor_hittest, handle, {
                let _ = winit.set_cursor_hittest(*cursor_hittest);
            });
            when_updated!(cursor_visible, handle, {
                winit.set_cursor_visible(*cursor_visible);
            });
            when_updated!(window_level, handle, {
                winit.set_window_level(*window_level);
            });
            when_updated!(decorated, handle, {
                winit.set_decorations(*decorated);
            });
            when_updated!(resize_increments, handle, {
                let increments: Option<PhysicalSize<f32>> =
                    if resize_increments.width > 0 || resize_increments.height > 0 {
                        Some(PhysicalSize::new(
                            resize_increments.width.into_float(),
                            resize_increments.height.into_float(),
                        ))
                    } else {
                        None
                    };
                winit.set_resize_increments(increments);
            });
            when_updated!(visible, handle, {
                winit.set_visible(*visible);
            });
            when_updated!(resizable, handle, {
                winit.set_resizable(*resizable);
                redraw = true;
            });
            when_updated!(window_icon, handle, {
                let icon = window_icon.as_ref().map(|icon| {
                    Icon::from_rgba(icon.as_raw().clone(), icon.width(), icon.height())
                        .expect("valid image")
                });
                winit.set_window_icon(icon);
            });
            when_updated!(enabled_buttons, handle, {
                winit.set_enabled_buttons(*enabled_buttons);
            });
            when_updated!(fullscreen, handle, {
                winit.set_fullscreen(fullscreen.clone());
            });

            if redraw {
                window.set_needs_redraw();
            }
        }
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused.set(focused);
    }

    pub fn set_occluded<W>(&mut self, window: &W, occluded: bool)
    where
        W: PlatformWindowImplementation,
    {
        self.occluded.set(occluded);
        self.update_ized(window);
    }

    pub fn keyboard_input<W>(
        &mut self,
        window: W,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
    ) -> EventHandling
    where
        W: PlatformWindowImplementation,
    {
        let cushy = self.cushy.clone();
        let _guard = cushy.enter_runtime();
        let mut window = RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            self.inner_size.source(),
            &self.close_requested,
        );
        let target = self.tree.focused_widget().unwrap_or(self.root.node_id);
        let Some(target) = self.tree.widget_from_node(target) else {
            return IGNORED;
        };
        let mut target = EventContext::new(
            WidgetContext::new(
                target,
                &self.current_theme,
                &mut window,
                &mut self.fonts,
                self.theme_mode.get(),
                &mut self.cursor,
            ),
            kludgine,
        );

        if recursively_handle_event(&mut target, |widget| {
            widget.keyboard_input(device_id, input.clone(), is_synthetic)
        })
        .is_some()
        {
            return HANDLED;
        }
        drop(target);

        self.handle_window_keyboard_input(&mut window, kludgine, input)
    }

    pub fn mouse_wheel<W>(
        &mut self,
        window: W,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    ) -> EventHandling
    where
        W: PlatformWindowImplementation,
    {
        let cushy = self.cushy.clone();
        let _guard = cushy.enter_runtime();
        let mut window = RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            self.inner_size.source(),
            &self.close_requested,
        );
        let widget = self
            .tree
            .hovered_widget()
            .and_then(|hovered| self.tree.widget_from_node(hovered))
            .unwrap_or_else(|| self.tree.widget(self.root.id()).expect("missing widget"));

        let mut widget = EventContext::new(
            WidgetContext::new(
                widget,
                &self.current_theme,
                &mut window,
                &mut self.fonts,
                self.theme_mode.get(),
                &mut self.cursor,
            ),
            kludgine,
        );
        if recursively_handle_event(&mut widget, |widget| {
            widget.mouse_wheel(device_id, delta, phase)
        })
        .is_some()
        {
            HANDLED
        } else {
            IGNORED
        }
    }

    fn ime<W>(&mut self, window: W, kludgine: &mut Kludgine, ime: &Ime) -> EventHandling
    where
        W: PlatformWindowImplementation,
    {
        let cushy = self.cushy.clone();
        let _guard = cushy.enter_runtime();
        let mut window = RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            self.inner_size.source(),
            &self.close_requested,
        );
        let widget = self
            .tree
            .focused_widget()
            .and_then(|hovered| self.tree.widget_from_node(hovered))
            .unwrap_or_else(|| self.tree.widget(self.root.id()).expect("missing widget"));
        let mut target = EventContext::new(
            WidgetContext::new(
                widget,
                &self.current_theme,
                &mut window,
                &mut self.fonts,
                self.theme_mode.get(),
                &mut self.cursor,
            ),
            kludgine,
        );

        if recursively_handle_event(&mut target, |widget| widget.ime(ime.clone())).is_some() {
            HANDLED
        } else {
            IGNORED
        }
    }

    fn cursor_moved<W>(
        &mut self,
        window: W,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        position: impl Into<Point<Px>>,
    ) where
        W: PlatformWindowImplementation,
    {
        let cushy = self.cushy.clone();
        let _guard = cushy.enter_runtime();
        let mut window = RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            self.inner_size.source(),
            &self.close_requested,
        );

        let location = position.into();
        self.cursor.location = Some(location);
        self.cursor_position.set_and_read(location);

        EventContext::new(
            WidgetContext::new(
                self.root.clone(),
                &self.current_theme,
                &mut window,
                &mut self.fonts,
                self.theme_mode.get(),
                &mut self.cursor,
            ),
            kludgine,
        )
        .update_hovered_widget();

        if let Some(state) = self.mouse_buttons.get(&device_id) {
            // Mouse Drag
            for (button, handler) in state {
                let Some(handler) = self.tree.widget(*handler) else {
                    continue;
                };
                let mut context = EventContext::new(
                    WidgetContext::new(
                        handler.clone(),
                        &self.current_theme,
                        &mut window,
                        &mut self.fonts,
                        self.theme_mode.get(),
                        &mut self.cursor,
                    ),
                    kludgine,
                );
                let Some(last_rendered_at) = context.last_layout() else {
                    continue;
                };
                context.mouse_drag(location - last_rendered_at.origin, device_id, *button);
            }
        }
    }

    fn cursor_left<W>(&mut self, window: W, kludgine: &mut Kludgine)
    where
        W: PlatformWindowImplementation,
    {
        let cushy = self.cushy.clone();
        let _guard = cushy.enter_runtime();
        self.cursor.location = None;
        self.cursor_position
            .set_and_read(Point::squared(Px::new(-1)));
        if self.cursor.widget.take().is_some() {
            let mut window = RunningWindow::new(
                window,
                kludgine.id(),
                &self.redraw_status,
                &self.cushy,
                &self.focused,
                &self.occluded,
                self.inner_size.source(),
                &self.close_requested,
            );

            let mut context = EventContext::new(
                WidgetContext::new(
                    self.root.clone(),
                    &self.current_theme,
                    &mut window,
                    &mut self.fonts,
                    self.theme_mode.get(),
                    &mut self.cursor,
                ),
                kludgine,
            );
            context.clear_hover();
        }
    }

    fn mouse_input<W>(
        &mut self,
        window: W,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) -> EventHandling
    where
        W: PlatformWindowImplementation,
    {
        let cushy = self.cushy.clone();
        let _guard = cushy.enter_runtime();
        let mut window = RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            self.inner_size.source(),
            &self.close_requested,
        );
        match state {
            ElementState::Pressed => {
                if let (ElementState::Pressed, Some(location), Some(hovered)) = (
                    state,
                    self.cursor.location,
                    self.cursor.widget.and_then(|id| self.tree.widget(id)),
                ) {
                    if let Some(handler) = recursively_handle_event(
                        &mut EventContext::new(
                            WidgetContext::new(
                                hovered.clone(),
                                &self.current_theme,
                                &mut window,
                                &mut self.fonts,
                                self.theme_mode.get(),
                                &mut self.cursor,
                            ),
                            kludgine,
                        ),
                        |context| {
                            let Some(layout) = context.last_layout() else {
                                return IGNORED;
                            };
                            let relative = location - layout.origin;
                            context.mouse_down(relative, device_id, button)
                        },
                    ) {
                        self.mouse_buttons
                            .entry(device_id)
                            .or_default()
                            .insert(button, handler.id());
                        return HANDLED;
                    }
                } else {
                    EventContext::new(
                        WidgetContext::new(
                            self.root.clone(),
                            &self.current_theme,
                            &mut window,
                            &mut self.fonts,
                            self.theme_mode.get(),
                            &mut self.cursor,
                        ),
                        kludgine,
                    )
                    .clear_focus();
                }
                IGNORED
            }
            ElementState::Released => {
                let Some(device_buttons) = self.mouse_buttons.get_mut(&device_id) else {
                    return IGNORED;
                };
                let Some(handler) = device_buttons.remove(&button) else {
                    return IGNORED;
                };
                if device_buttons.is_empty() {
                    self.mouse_buttons.remove(&device_id);
                }
                let Some(handler) = self.tree.widget(handler) else {
                    return IGNORED;
                };
                let cursor_location = self.cursor.location;
                let mut context = EventContext::new(
                    WidgetContext::new(
                        handler,
                        &self.current_theme,
                        &mut window,
                        &mut self.fonts,
                        self.theme_mode.get(),
                        &mut self.cursor,
                    ),
                    kludgine,
                );

                let relative = if let (Some(last_rendered), Some(location)) =
                    (context.last_layout(), cursor_location)
                {
                    Some(location - last_rendered.origin)
                } else {
                    None
                };

                context.mouse_up(relative, device_id, button);
                HANDLED
            }
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum RootMode {
    Fit,
    Expand,
    Align,
}

impl<T> kludgine::app::WindowBehavior<WindowCommand> for OpenWindow<T>
where
    T: WindowBehavior,
{
    type Context = sealed::Context<T::Context>;

    fn initialize(
        window: kludgine::app::Window<'_, WindowCommand>,
        graphics: &mut kludgine::Graphics<'_>,
        context: Self::Context,
    ) -> Self {
        context.pending.opened(window.handle());
        let settings = context.settings.borrow_mut();
        let cushy = settings.cushy.clone();
        let _guard = cushy.enter_runtime();
        let mut window = RunningWindow::new(
            window,
            graphics.id(),
            &settings.redraw_status,
            &settings.cushy,
            &settings.focused,
            &settings.occluded,
            &settings.inner_size,
            &settings.close_requested,
        );
        drop(settings);

        let behavior = T::initialize(&mut window, context.user);
        Self::new(
            behavior,
            window.window,
            graphics,
            context.settings.into_inner(),
        )
    }

    fn prepare(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        graphics: &mut kludgine::Graphics<'_>,
    ) {
        self.prepare(window, graphics);
    }

    fn present_mode(&self) -> wgpu::PresentMode {
        if self.vsync {
            wgpu::PresentMode::AutoVsync
        } else {
            wgpu::PresentMode::AutoNoVsync
        }
    }

    fn multisample_count(context: &Self::Context) -> std::num::NonZeroU32 {
        context.settings.borrow().multisample_count
    }

    fn memory_hints(_context: &Self::Context) -> wgpu::MemoryHints {
        wgpu::MemoryHints::MemoryUsage
    }

    fn focus_changed(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        _kludgine: &mut Kludgine,
    ) {
        self.set_focused(window.focused());
    }

    fn occlusion_changed(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        _kludgine: &mut Kludgine,
    ) {
        self.set_occluded(&window, window.occluded());
    }

    fn render<'pass>(
        &'pass mut self,
        _window: kludgine::app::Window<'_, WindowCommand>,
        graphics: &mut kludgine::RenderingGraphics<'_, 'pass>,
    ) -> bool {
        self.contents.render(1., graphics);

        !self.should_close
    }

    fn initial_window_attributes(context: &Self::Context) -> kludgine::app::WindowAttributes {
        let mut settings = context.settings.borrow_mut();
        let mut attrs = settings.attributes.take().expect("called more than once");
        if let Some(Value::Constant(theme_mode)) = &settings.theme_mode {
            attrs.preferred_theme = Some((*theme_mode).into());
        }
        attrs.title = settings.title.get();
        if attrs.inner_size.is_none() {
            let dynamic_inner = settings.inner_size.get();
            if !dynamic_inner.is_zero() {
                attrs.inner_size = Some(winit::dpi::Size::Physical(dynamic_inner.into()));
            }
        }
        attrs
    }

    fn close_requested(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
    ) -> bool {
        let cushy = self.cushy.clone();
        let _guard = cushy.enter_runtime();
        Self::request_close(
            &mut self.should_close,
            &mut self.behavior,
            &mut RunningWindow::new(
                window,
                kludgine.id(),
                &self.redraw_status,
                &self.cushy,
                &self.focused,
                &self.occluded,
                self.inner_size.source(),
                &self.close_requested,
            ),
        )
    }

    // fn power_preference() -> wgpu::PowerPreference {
    //     wgpu::PowerPreference::default()
    // }

    // fn limits(adapter_limits: wgpu::Limits) -> wgpu::Limits {
    //     wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter_limits)
    // }

    fn clear_color(&self) -> Option<kludgine::Color> {
        Some(if self.transparent {
            kludgine::Color::CLEAR_BLACK
        } else {
            kludgine::Color::BLACK
        })
    }

    fn composite_alpha_mode(&self, supported_modes: &[CompositeAlphaMode]) -> CompositeAlphaMode {
        if self.transparent && supported_modes.contains(&CompositeAlphaMode::PreMultiplied) {
            CompositeAlphaMode::PreMultiplied
        } else {
            CompositeAlphaMode::Auto
        }
    }

    // fn focus_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn occlusion_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    fn scale_factor_changed(
        &mut self,
        mut window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
    ) {
        self.dpi_scale.set(kludgine.dpi_scale());
        window.set_needs_redraw();
    }

    fn resized(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        _kludgine: &mut Kludgine,
    ) {
        self.resized(window.inner_size(), &window);
    }

    fn moved(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        _kludgine: &mut Kludgine,
    ) {
        self.moved(window.inner_position(), window.outer_position());
    }

    // fn theme_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn dropped_file(&mut self, window: kludgine::app::Window<'_, ()>, path: std::path::PathBuf) {}

    // fn hovered_file(&mut self, window: kludgine::app::Window<'_, ()>, path: std::path::PathBuf) {}

    // fn hovered_file_cancelled(&mut self, window: kludgine::app::Window<'_, ()>) {}

    // fn received_character(&mut self, window: kludgine::app::Window<'_, ()>, char: char) {}

    fn keyboard_input(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        device_id: winit::event::DeviceId,
        input: winit::event::KeyEvent,
        is_synthetic: bool,
    ) {
        let event = KeyEvent::from_winit(input, window.modifiers());
        self.keyboard_input(window, kludgine, device_id.into(), event, is_synthetic);
    }

    fn mouse_wheel(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        device_id: winit::event::DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    ) {
        self.mouse_wheel(window, kludgine, device_id.into(), delta, phase);
    }

    fn modifiers_changed(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        _kludgine: &mut Kludgine,
    ) {
        self.modifiers.set(window.modifiers());
    }

    fn ime(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        ime: Ime,
    ) {
        self.ime(window, kludgine, &ime);
    }

    fn cursor_moved(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        device_id: winit::event::DeviceId,
        position: PhysicalPosition<f64>,
    ) {
        self.cursor_moved(window, kludgine, device_id.into(), position);
    }

    fn cursor_left(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        _device_id: winit::event::DeviceId,
    ) {
        self.cursor_left(window, kludgine);
    }

    fn mouse_input(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        device_id: winit::event::DeviceId,
        state: ElementState,
        button: MouseButton,
    ) {
        self.mouse_input(window, kludgine, device_id.into(), state, button);
    }

    fn theme_changed(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        _kludgine: &mut Kludgine,
    ) {
        if let Value::Dynamic(theme_mode) = &self.theme_mode {
            theme_mode.set(window.theme().into());
        }
    }

    fn event(
        &mut self,
        mut window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        event: WindowCommand,
    ) {
        match event {
            WindowCommand::Redraw => {
                window.set_needs_redraw();
            }
            WindowCommand::Sync => {
                self.synchronize_platform_window(&mut window);
            }
            WindowCommand::RequestClose => {
                let mut window = RunningWindow::new(
                    window,
                    kludgine.id(),
                    &self.redraw_status,
                    &self.cushy,
                    &self.focused,
                    &self.occluded,
                    self.inner_size.source(),
                    &self.close_requested,
                );
                if self.behavior.close_requested(&mut window) {
                    window.close();
                }
            }
            WindowCommand::SetTitle(new_title) => {
                window.set_title(&new_title);
            }
            WindowCommand::ResetDeadKeys => {
                window.winit().reset_dead_keys();
            }
            WindowCommand::RequestUserAttention(request_type) => {
                window.winit().request_user_attention(request_type);
            }
            WindowCommand::Focus => {
                window.winit().focus_window();
            }
            WindowCommand::Ize(ize) => {
                let (minimize, maximize) = match ize {
                    Some(Ize::Maximize) => (false, true),
                    Some(Ize::Minimize) => (true, false),
                    None => (false, false),
                };
                if window
                    .winit()
                    .is_minimized()
                    .map_or(true, |minimized| minimized != minimize)
                {
                    window.winit().set_minimized(minimize);
                }
                if window.winit().is_maximized() != maximize {
                    window.winit().set_maximized(maximize);
                }
            }
        }
    }

    // fn dropped_file(
    //     &mut self,
    //     window: kludgine::app::Window<'_, WindowCommand>,
    //     kludgine: &mut Kludgine,
    //     path: std::path::PathBuf,
    // ) {
    // }

    // fn hovered_file(
    //     &mut self,
    //     window: kludgine::app::Window<'_, WindowCommand>,
    //     kludgine: &mut Kludgine,
    //     path: std::path::PathBuf,
    // ) {
    // }

    // fn hovered_file_cancelled(
    //     &mut self,
    //     window: kludgine::app::Window<'_, WindowCommand>,
    //     kludgine: &mut Kludgine,
    // ) {
    // }

    // fn received_character(
    //     &mut self,
    //     window: kludgine::app::Window<'_, WindowCommand>,
    //     kludgine: &mut Kludgine,
    //     char: char,
    // ) {
    // }

    // fn modifiers_changed(
    //     &mut self,
    //     window: kludgine::app::Window<'_, WindowCommand>,
    //     kludgine: &mut Kludgine,
    // ) {
    // }
}

impl<Behavior> Drop for OpenWindow<Behavior> {
    fn drop(&mut self) {
        if let Some(on_closed) = self.on_closed.take() {
            on_closed.invoke(());
        }
    }
}

fn recursively_handle_event(
    context: &mut EventContext<'_>,
    mut each_widget: impl FnMut(&mut EventContext<'_>) -> EventHandling,
) -> Option<MountedWidget> {
    match each_widget(context) {
        HANDLED => Some(context.widget().clone()),
        IGNORED => context.parent().and_then(|parent| {
            recursively_handle_event(&mut context.for_other(&parent), each_widget)
        }),
    }
}

#[derive(Default)]
pub(crate) struct CursorState {
    pub(crate) location: Option<Point<Px>>,
    pub(crate) widget: Option<WidgetId>,
}

pub(crate) mod sealed {
    use std::cell::RefCell;
    use std::num::NonZeroU32;

    use figures::units::{Px, UPx};
    use figures::{Fraction, Point, Size};
    use image::{DynamicImage, RgbaImage};
    use kludgine::app::winit::event::Modifiers;
    use kludgine::app::winit::window::{Fullscreen, UserAttentionType, WindowButtons, WindowLevel};
    use kludgine::Color;

    use super::{PendingWindow, WindowHandle};
    use crate::app::Cushy;
    use crate::context::sealed::InvalidationStatus;
    use crate::fonts::FontCollection;
    use crate::styles::{FontFamilyList, ThemePair};
    use crate::value::{Dynamic, Value};
    use crate::widget::{OnceCallback, SharedCallback};
    use crate::window::{ThemeMode, WindowAttributes};

    pub struct Context<C> {
        pub user: C,
        pub pending: PendingWindow,
        pub settings: RefCell<WindowSettings>,
    }

    pub struct WindowSettings {
        pub cushy: Cushy,
        pub redraw_status: InvalidationStatus,
        pub title: Value<String>,
        pub attributes: Option<WindowAttributes>,
        pub occluded: Dynamic<bool>,
        pub focused: Dynamic<bool>,
        pub inner_size: Dynamic<Size<UPx>>,
        pub zoom: Dynamic<Fraction>,
        pub theme: Option<Value<ThemePair>>,
        pub theme_mode: Option<Value<ThemeMode>>,
        pub transparent: bool,
        pub serif_font_family: FontFamilyList,
        pub sans_serif_font_family: FontFamilyList,
        pub fantasy_font_family: FontFamilyList,
        pub monospace_font_family: FontFamilyList,
        pub cursive_font_family: FontFamilyList,
        pub font_data_to_load: FontCollection,
        pub on_open: Option<OnceCallback<WindowHandle>>,
        pub on_closed: Option<OnceCallback>,
        pub vsync: bool,
        pub multisample_count: NonZeroU32,
        pub resize_to_fit: Value<bool>,
        pub close_requested: Option<SharedCallback<(), bool>>,
        pub content_protected: Value<bool>,
        pub cursor_hittest: Value<bool>,
        pub cursor_visible: Value<bool>,
        pub cursor_position: Dynamic<Point<Px>>,
        pub window_level: Value<WindowLevel>,
        pub decorated: Value<bool>,
        pub maximized: Dynamic<bool>,
        pub minimized: Dynamic<bool>,
        pub resizable: Value<bool>,
        pub resize_increments: Value<Size<UPx>>,
        pub visible: Dynamic<bool>,
        pub inner_position: Dynamic<Point<Px>>,
        pub outer_position: Dynamic<Point<Px>>,
        pub outer_size: Dynamic<Size<UPx>>,
        pub window_icon: Value<Option<RgbaImage>>,
        pub modifiers: Dynamic<Modifiers>,
        pub enabled_buttons: Value<WindowButtons>,
        pub fullscreen: Value<Option<Fullscreen>>,
    }

    #[derive(Debug, Clone)]
    pub enum WindowCommand {
        Redraw,
        Sync,
        RequestClose,
        ResetDeadKeys,
        RequestUserAttention(Option<UserAttentionType>),
        Focus,
        Ize(Option<Ize>),
        SetTitle(String),
    }

    #[derive(Debug, Clone)]
    pub enum Ize {
        Maximize,
        Minimize,
    }

    pub trait CaptureFormat {
        const HAS_ALPHA: bool;

        fn convert_rgba(data: &mut Vec<u8>, width: u32, bytes_per_row: u32);
        fn load_image(data: &[u8], size: Size<UPx>) -> DynamicImage;
        fn pixel_color(location: Point<UPx>, data: &[u8], size: Size<UPx>) -> Color;
    }
}

/// Controls whether the light or dark theme is applied.
#[derive(Default, Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, LinearInterpolate)]
pub enum ThemeMode {
    /// Applies the light theme
    Light,
    /// Applies the dark theme
    #[default]
    Dark,
}

impl ThemeMode {
    /// Returns the opposite mode of `self`.
    #[must_use]
    pub const fn inverse(self) -> Self {
        match self {
            ThemeMode::Light => Self::Dark,
            ThemeMode::Dark => Self::Light,
        }
    }

    /// Updates `self` with its [inverse](Self::inverse).
    pub fn toggle(&mut self) {
        *self = !*self;
    }
}

impl Not for ThemeMode {
    type Output = Self;

    fn not(self) -> Self::Output {
        self.inverse()
    }
}

impl From<window::Theme> for ThemeMode {
    fn from(value: window::Theme) -> Self {
        match value {
            window::Theme::Light => Self::Light,
            window::Theme::Dark => Self::Dark,
        }
    }
}

impl From<ThemeMode> for window::Theme {
    fn from(value: ThemeMode) -> Self {
        match value {
            ThemeMode::Light => Self::Light,
            ThemeMode::Dark => Self::Dark,
        }
    }
}

impl PercentBetween for ThemeMode {
    fn percent_between(&self, min: &Self, max: &Self) -> ZeroToOne {
        if *min == *max || *self == *min {
            ZeroToOne::ZERO
        } else {
            ZeroToOne::ONE
        }
    }
}

impl Ranged for ThemeMode {
    const MAX: Self = Self::Dark;
    const MIN: Self = Self::Light;
}

#[cfg(any(target_os = "macos", target_os = "ios", target_os = "windows"))]
fn default_family(_query: Family<'_>) -> Option<FamilyOwned> {
    // fontdb uses system APIs to determine these defaults.
    None
}

#[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "windows")))]
fn default_family(query: Family<'_>) -> Option<FamilyOwned> {
    // fontdb does not yet support configuring itself automatically. We will try
    // to use `fc-match` to query font config. Once this is supported, we can
    // remove this functionality.
    // <https://github.com/RazrFalcon/fontdb/issues/24>
    let query = match query {
        Family::Serif => "serif",
        Family::SansSerif => "sans",
        Family::Cursive => "cursive",
        Family::Fantasy => "fantasy",
        Family::Monospace => "monospace",
        Family::Name(_) => return None,
    };

    std::process::Command::new("fc-match")
        .arg("-f")
        .arg("%{family}")
        .arg(query)
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(FamilyOwned::Name)
}

/// A handle to an open Cushy window.
#[derive(Debug, Clone)]
pub struct WindowHandle {
    inner: InnerWindowHandle,
    pub(crate) redraw_status: InvalidationStatus,
}

impl WindowHandle {
    pub(crate) fn new(
        kludgine: kludgine::app::WindowHandle<WindowCommand>,
        redraw_status: InvalidationStatus,
    ) -> Self {
        Self {
            inner: InnerWindowHandle::Known(kludgine),
            redraw_status,
        }
    }

    fn pending() -> Self {
        Self {
            inner: InnerWindowHandle::Pending(Arc::default()),
            redraw_status: InvalidationStatus::default(),
        }
    }

    /// Request that the window closes.
    ///
    /// A window may disallow itself from being closed by customizing
    /// [`WindowBehavior::close_requested`].
    pub fn request_close(&self) {
        self.inner.send(sealed::WindowCommand::RequestClose);
    }

    /// Requests that the window redraws.
    pub fn redraw(&self) {
        if self.redraw_status.should_send_refresh() {
            self.inner.send(WindowCommand::Redraw);
        }
    }

    pub(crate) fn sync(&self) {
        if self.redraw_status.should_send_sync() {
            self.inner.send(WindowCommand::Sync);
        }
    }

    /// Marks `widget` as invalidated, and if needed, refreshes the window.
    pub fn invalidate(&self, widget: WidgetId) {
        if self.redraw_status.invalidate(widget) {
            self.redraw();
        }
    }
}

impl Eq for WindowHandle {}

impl PartialEq for WindowHandle {
    fn eq(&self, other: &Self) -> bool {
        self.redraw_status == other.redraw_status
    }
}

impl Hash for WindowHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.redraw_status.hash(state);
    }
}

#[derive(Debug, Clone)]
enum InnerWindowHandle {
    Pending(Arc<PendingWindowHandle>),
    Known(kludgine::app::WindowHandle<WindowCommand>),
    Virtual(WindowDynamicState),
}

impl InnerWindowHandle {
    fn send(&self, message: WindowCommand) {
        match self {
            InnerWindowHandle::Pending(pending) => {
                if let Some(handle) = pending.handle.get() {
                    let _result = handle.send(message);
                } else {
                    pending.commands.lock().push(message);
                }
            }
            InnerWindowHandle::Known(handle) => {
                let _result = handle.send(message);
            }
            InnerWindowHandle::Virtual(state) => match message {
                WindowCommand::Redraw => state.redraw_target.set(RedrawTarget::Now),
                WindowCommand::RequestClose => state.close_requested.set(true),
                WindowCommand::SetTitle(title) => state.title.set(title),
                WindowCommand::ResetDeadKeys
                | WindowCommand::RequestUserAttention(_)
                | WindowCommand::Focus
                | WindowCommand::Ize(_)
                | WindowCommand::Sync => {}
            },
        };
    }
}

/// A [`Window`] that doesn't have its root widget yet.
///
/// [`PendingWindow::handle()`] returns a handle that allows code to interact
/// with a window before it has had its contents initialized. This is useful,
/// for example, for a button's `on_click` to be able to close the window that
/// contains it.
pub struct PendingWindow(WindowHandle);

impl Default for PendingWindow {
    fn default() -> Self {
        Self(WindowHandle::pending())
    }
}

impl PendingWindow {
    /// Returns a [`Window`] using `context` to initialize its contents.
    pub fn with<Behavior>(self, context: Behavior::Context) -> Window<Behavior>
    where
        Behavior: WindowBehavior,
    {
        Window::new_with_pending(context, self)
    }

    /// Returns a [`Window`] containing `root`.
    pub fn with_root(self, root: impl MakeWidget) -> Window {
        Window::new_with_pending(root.make_widget(), self)
    }

    /// Returns a [`Window`] using the default context to initialize its
    /// contents.
    pub fn using<Behavior>(self) -> Window<Behavior>
    where
        Behavior: WindowBehavior,
        Behavior::Context: Default,
    {
        self.with(<Behavior::Context>::default())
    }

    /// Returns a handle for this window.
    #[must_use]
    pub fn handle(&self) -> WindowHandle {
        self.0.clone()
    }

    fn opened(self, handle: kludgine::app::WindowHandle<WindowCommand>) -> WindowHandle {
        let InnerWindowHandle::Pending(pending) = &self.0.inner else {
            unreachable!("always pending")
        };

        let initialized = pending.handle.set(handle.clone());
        assert!(initialized.is_ok());

        for command in pending.commands.lock().drain(..) {
            let _result = handle.send(command);
        }

        WindowHandle::new(handle, self.0.redraw_status.clone())
    }
}

#[derive(Debug, Default)]
struct PendingWindowHandle {
    handle: OnceLock<kludgine::app::WindowHandle<WindowCommand>>,
    commands: Mutex<Vec<WindowCommand>>,
}

/// A collection that stores an instance of `T` per window.
///
/// This is a convenience wrapper around a `HashMap<KludgineId, T>`.
#[derive(Debug, Clone)]
pub struct WindowLocal<T> {
    by_window: AHashMap<KludgineId, T>,
}

impl<T> WindowLocal<T> {
    /// Looks up the entry for this window.
    ///
    /// Internally this API uses [`HashMap::entry`](hash_map::HashMap::entry).
    pub fn entry(&mut self, context: &WidgetContext<'_>) -> hash_map::Entry<'_, KludgineId, T> {
        self.by_window.entry(context.kludgine_id())
    }

    /// Sets `value` as the local value for `context`'s window.
    pub fn set(&mut self, context: &WidgetContext<'_>, value: T) {
        self.by_window.insert(context.kludgine_id(), value);
    }

    /// Looks up the value for this window, returning None if not found.
    ///
    /// Internally this API uses [`HashMap::get`](hash_map::HashMap::get).
    #[must_use]
    pub fn get(&self, context: &WidgetContext<'_>) -> Option<&T> {
        self.by_window.get(&context.kludgine_id())
    }

    /// Looks up an exclusive reference to the value for this window, returning
    /// None if not found.
    ///
    /// Internally this API uses [`HashMap::get`](hash_map::HashMap::get).
    #[must_use]
    pub fn get_mut(&mut self, context: &WidgetContext<'_>) -> Option<&mut T> {
        self.by_window.get_mut(&context.kludgine_id())
    }

    /// Removes any stored value for this window.
    pub fn clear_for(&mut self, context: &WidgetContext<'_>) -> Option<T> {
        self.by_window.remove(&context.kludgine_id())
    }

    /// Returns an iterator over the per-window values stored in this
    /// collection.
    #[must_use]
    pub fn iter(&self) -> hash_map::Iter<'_, KludgineId, T> {
        self.into_iter()
    }
}

impl<T> Default for WindowLocal<T> {
    fn default() -> Self {
        Self {
            by_window: AHashMap::default(),
        }
    }
}

impl<T> IntoIterator for WindowLocal<T> {
    type IntoIter = hash_map::IntoIter<KludgineId, T>;
    type Item = (KludgineId, T);

    fn into_iter(self) -> Self::IntoIter {
        self.by_window.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a WindowLocal<T> {
    type IntoIter = hash_map::Iter<'a, KludgineId, T>;
    type Item = (&'a KludgineId, &'a T);

    fn into_iter(self) -> Self::IntoIter {
        self.by_window.iter()
    }
}

/// The state of a [`VirtualWindow`].
pub struct VirtualState {
    /// State that may be updated outside of the window's event callbacks.
    pub dynamic: WindowDynamicState,
    /// When true, this window should be closed.
    pub closed: bool,
    /// The current keyboard modifers.
    pub modifiers: Modifiers,
    /// The amount of time elapsed since the last redraw call.
    pub elapsed: Duration,
    /// The currently set cursor.
    pub cursor: Cursor,
    /// The inner size of the virtual window.
    pub size: Size<UPx>,
}

impl VirtualState {
    fn new() -> Self {
        Self {
            dynamic: WindowDynamicState::default(),
            closed: false,
            modifiers: Modifiers::default(),
            elapsed: Duration::ZERO,
            cursor: Cursor::default(),
            size: Size::upx(800, 600),
        }
    }
}

/// Window state that is able to be updated outside of event handling,
/// potentially via other threads depending on the application.
#[derive(Clone, Debug, Default)]
pub struct WindowDynamicState {
    /// The target of the next frame to draw.
    pub redraw_target: Dynamic<RedrawTarget>,
    /// When true, the window has been asked to close. To ensure full Cushy
    /// functionality, upon detecting this, [`VirtualWindow::request_close`]
    /// should be invoked.
    pub close_requested: Dynamic<bool>,
    /// The current title of the window.
    pub title: Dynamic<String>,
}

/// A target for the next redraw of a window.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedrawTarget {
    /// The window should not redraw.
    #[default]
    Never,
    /// The window should redraw as soon as possible.
    Now,
    /// The window should try to redraw at the given instant.
    At(Instant),
}

impl PlatformWindowImplementation for &mut VirtualState {
    fn close(&mut self) {
        self.closed = true;
    }

    fn winit(&self) -> Option<&winit::window::Window> {
        None
    }

    fn handle(&self, redraw_status: InvalidationStatus) -> WindowHandle {
        WindowHandle {
            inner: InnerWindowHandle::Virtual(self.dynamic.clone()),
            redraw_status,
        }
    }

    fn set_needs_redraw(&mut self) {
        self.dynamic.redraw_target.set(RedrawTarget::Now);
    }

    fn redraw_in(&mut self, duration: Duration) {
        self.redraw_at(Instant::now() + duration);
    }

    fn redraw_at(&mut self, moment: Instant) {
        self.dynamic.redraw_target.map_mut(|mut redraw_at| {
            if match *redraw_at {
                RedrawTarget::At(instant) => moment < instant,
                RedrawTarget::Never => true,
                RedrawTarget::Now => false,
            } {
                *redraw_at = RedrawTarget::At(moment);
            }
        });
    }

    fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    fn elapsed(&self) -> Duration {
        self.elapsed
    }

    fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
    }

    fn inner_size(&self) -> Size<UPx> {
        self.size
    }

    fn request_inner_size(&mut self, inner_size: Size<UPx>) {
        self.size = inner_size;
        self.set_needs_redraw();
    }
}

/// A builder that creates either a [`VirtualWindow`] or a [`CushyWindow`].
pub struct StandaloneWindowBuilder {
    widget: WidgetInstance,
    multisample_count: NonZeroU32,
    initial_size: Size<UPx>,
    scale: f32,
    transparent: bool,
    zoom: Dynamic<Fraction>,
    resize_to_fit: Value<bool>,
}

impl StandaloneWindowBuilder {
    /// Returns a new builder for a standalone window that contains `contents`.
    #[must_use]
    pub fn new(contents: impl MakeWidget) -> Self {
        Self {
            widget: contents.make_widget(),
            multisample_count: NonZeroU32::new(4).assert("not 0"),
            initial_size: Size::upx(800, 600),
            scale: 1.,
            zoom: Dynamic::new(Fraction::ONE),
            transparent: false,
            resize_to_fit: Value::Constant(false),
        }
    }

    /// Sets this window's multi-sample count.
    ///
    /// By default, 4 samples are taken. When 1 sample is used, multisampling is
    /// fully disabled.
    #[must_use]
    pub fn multisample_count(mut self, count: NonZeroU32) -> Self {
        self.multisample_count = count;
        self
    }

    /// Sets the size of the window.
    #[must_use]
    pub fn size<Unit>(mut self, size: Size<Unit>) -> Self
    where
        Unit: Into<UPx>,
    {
        self.initial_size = size.map(Into::into);
        self
    }

    /// Sets the DPI scaling factor of the window.
    #[must_use]
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Sets the window not fill its background before rendering its contents.
    #[must_use]
    pub fn transparent(mut self) -> Self {
        self.transparent = true;
        self
    }

    /// Resizes this window to fit the contents when `resize_to_fit` is true.
    #[must_use]
    pub fn resize_to_fit(mut self, resize_to_fit: impl IntoValue<bool>) -> Self {
        self.resize_to_fit = resize_to_fit.into_value();
        self
    }

    /// Returns the initialized window.
    #[must_use]
    pub fn finish<W>(self, window: W, device: &wgpu::Device, queue: &wgpu::Queue) -> CushyWindow
    where
        W: PlatformWindowImplementation,
    {
        let mut kludgine = Kludgine::new(
            device,
            queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::MultisampleState {
                count: self.multisample_count.get(),
                ..Default::default()
            },
            self.initial_size,
            self.scale,
        );
        let window = OpenWindow::<WidgetInstance>::new(
            self.widget,
            window,
            &mut kludgine::Graphics::new(&mut kludgine, device, queue),
            sealed::WindowSettings {
                cushy: Cushy::default(),
                redraw_status: InvalidationStatus::default(),
                title: Value::default(),
                attributes: None,
                occluded: Dynamic::default(),
                focused: Dynamic::default(),
                inner_size: Dynamic::default(),
                theme: None,
                theme_mode: None,
                transparent: self.transparent,
                serif_font_family: FontFamilyList::default(),
                sans_serif_font_family: FontFamilyList::default(),
                fantasy_font_family: FontFamilyList::default(),
                monospace_font_family: FontFamilyList::default(),
                cursive_font_family: FontFamilyList::default(),
                font_data_to_load: FontCollection::default(),
                on_open: None,
                on_closed: None,
                vsync: false,
                multisample_count: self.multisample_count,
                close_requested: None,
                zoom: self.zoom,
                resize_to_fit: self.resize_to_fit,
                content_protected: Value::Constant(false),
                cursor_hittest: Value::Constant(true),
                cursor_visible: Value::Constant(true),
                cursor_position: Dynamic::default(),
                window_level: Value::default(),
                decorated: Value::Constant(true),
                maximized: Dynamic::new(false),
                minimized: Dynamic::new(false),
                resizable: Value::Constant(true),
                resize_increments: Value::default(),
                visible: Dynamic::new(true),
                inner_position: Dynamic::default(),
                outer_position: Dynamic::default(),
                outer_size: Dynamic::default(),
                window_icon: Value::Constant(None),
                modifiers: Dynamic::default(),
                enabled_buttons: Value::dynamic(WindowButtons::all()),
                fullscreen: Value::default(),
            },
        );

        CushyWindow { window, kludgine }
    }

    /// Returns an initialized [`VirtualWindow`].
    #[must_use]
    pub fn finish_virtual(self, device: &wgpu::Device, queue: &wgpu::Queue) -> VirtualWindow {
        let mut state = VirtualState::new();
        state.size = self.initial_size;
        let mut cushy = self.finish(&mut state, device, queue);
        cushy.set_focused(true);

        VirtualWindow {
            cushy,
            state,
            last_rendered_at: None,
        }
    }
}

/// A standalone Cushy window.
///
/// This type allows rendering Cushy applications directly into any wgpu
/// application.
pub struct CushyWindow {
    window: OpenWindow<WidgetInstance>,
    kludgine: Kludgine,
}

impl CushyWindow {
    /// Prepares all necessary resources and operations necessary to render the
    /// next frame.
    pub fn prepare<W>(&mut self, window: W, device: &wgpu::Device, queue: &wgpu::Queue)
    where
        W: PlatformWindowImplementation,
    {
        self.window.prepare(
            window,
            &mut kludgine::Graphics::new(&mut self.kludgine, device, queue),
        );
    }

    /// Renders this window in a wgpu render pass created from `pass`.
    ///
    /// Returns the submission index of the last command submission, if any
    /// commands were submitted.
    pub fn render(
        &mut self,
        pass: &wgpu::RenderPassDescriptor<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<wgpu::SubmissionIndex> {
        self.render_with(pass, device, queue, None)
    }

    /// Renders this window in a wgpu render pass created from `pass`.
    ///
    /// Returns the submission index of the last command submission, if any
    /// commands were submitted.
    pub fn render_with(
        &mut self,
        pass: &wgpu::RenderPassDescriptor<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        additional_drawing: Option<&Drawing>,
    ) -> Option<wgpu::SubmissionIndex> {
        let mut frame = self.kludgine.next_frame();
        let mut gfx = frame.render(pass, device, queue);
        self.window.contents.render(1., &mut gfx);
        if let Some(additional) = additional_drawing {
            additional.render(1., &mut gfx);
        }
        drop(gfx);
        frame.submit(queue)
    }

    /// Renders this window into `texture` after performing `load_op`.
    pub fn render_into(
        &mut self,
        texture: &kludgine::Texture,
        load_op: wgpu::LoadOp<Color>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<wgpu::SubmissionIndex> {
        let mut frame = self.kludgine.next_frame();
        let mut gfx = frame.render_into(texture, load_op, device, queue);
        self.window.contents.render(1., &mut gfx);
        drop(gfx);
        frame.submit(queue)
    }

    /// Returns a new [`kludgine::Graphics`] context for this window.
    #[must_use]
    pub fn graphics<'gfx>(
        &'gfx mut self,
        device: &'gfx wgpu::Device,
        queue: &'gfx wgpu::Queue,
    ) -> kludgine::Graphics<'gfx> {
        kludgine::Graphics::new(&mut self.kludgine, device, queue)
    }

    /// Sets the window's focused status.
    ///
    /// Being focused means that the window is expecting to be able to receive
    /// user input.
    pub fn set_focused(&mut self, focused: bool) {
        self.window.set_focused(focused);
    }

    /// Sets the window's occlusion status.
    ///
    /// This should only be set to true if the window is not visible at all to
    /// the end user due to being offscreen, minimized, or fully hidden behind
    /// other windows.
    pub fn set_occluded<W>(&mut self, window: &W, occluded: bool)
    where
        W: PlatformWindowImplementation,
    {
        self.window.set_occluded(window, occluded);
    }

    /// Requests that the window close.
    ///
    /// Returns true if the request should be honored.
    pub fn request_close<W>(&mut self, window: W) -> bool
    where
        W: PlatformWindowImplementation,
    {
        self.window.close_requested(window, &mut self.kludgine)
    }

    /// Returns the current size of the window.
    pub const fn size(&self) -> Size<UPx> {
        self.kludgine.size()
    }

    /// Returns the current DPI scale of the window.
    pub const fn dpi_scale(&self) -> Fraction {
        self.kludgine.dpi_scale()
    }

    /// Returns the effective scale of the window.
    pub fn effective_scale(&self) -> Fraction {
        self.kludgine.scale()
    }

    /// Updates the dimensions and DPI scaling of the window.
    pub fn resize<W>(
        &mut self,
        window: &W,
        new_size: Size<UPx>,
        new_scale: impl Into<Fraction>,
        new_zoom: impl Into<Fraction>,
        queue: &wgpu::Queue,
    ) where
        W: PlatformWindowImplementation,
    {
        self.kludgine.resize(new_size, new_scale, new_zoom, queue);
        self.window.resized(new_size, window);
    }

    /// Sets the window's position.
    pub fn set_position(&mut self, new_position: Point<Px>) {
        self.window.moved(new_position, new_position);
    }

    /// Provide keyboard input to this virtual window.
    ///
    /// Returns whether the event was [`HANDLED`] or [`IGNORED`].
    pub fn keyboard_input<W>(
        &mut self,
        window: W,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
    ) -> EventHandling
    where
        W: PlatformWindowImplementation,
    {
        self.window
            .keyboard_input(window, &mut self.kludgine, device_id, input, is_synthetic)
    }

    /// Provides mouse wheel input to this window.
    ///
    /// Returns whether the event was [`HANDLED`] or [`IGNORED`].
    pub fn mouse_wheel<W>(
        &mut self,
        window: W,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    ) -> EventHandling
    where
        W: PlatformWindowImplementation,
    {
        self.window
            .mouse_wheel(window, &mut self.kludgine, device_id, delta, phase)
    }

    /// Provides input manager events to this window.
    ///
    /// Returns whether the event was [`HANDLED`] or [`IGNORED`].
    pub fn ime<W>(&mut self, window: W, ime: &Ime) -> EventHandling
    where
        W: PlatformWindowImplementation,
    {
        self.window.ime(window, &mut self.kludgine, ime)
    }

    /// Provides cursor movement events to this window.
    pub fn cursor_moved<W>(
        &mut self,
        window: W,
        device_id: DeviceId,
        position: impl Into<Point<Px>>,
    ) where
        W: PlatformWindowImplementation,
    {
        self.window
            .cursor_moved(window, &mut self.kludgine, device_id, position);
    }

    /// Notifies the window that the cursor is no longer within the window.
    pub fn cursor_left<W>(&mut self, window: W)
    where
        W: PlatformWindowImplementation,
    {
        self.window.cursor_left(window, &mut self.kludgine);
    }

    /// Provides mouse input events to tihs window.
    ///
    /// Returns whether the event was [`HANDLED`] or [`IGNORED`].
    pub fn mouse_input<W>(
        &mut self,
        window: W,
        device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) -> EventHandling
    where
        W: PlatformWindowImplementation,
    {
        self.window
            .mouse_input(window, &mut self.kludgine, device_id, state, button)
    }
}

/// A virtual Cushy window.
///
/// This type allows rendering Cushy applications directly into any wgpu
/// application.
pub struct VirtualWindow {
    cushy: CushyWindow,
    state: VirtualState,
    last_rendered_at: Option<Instant>,
}

impl VirtualWindow {
    /// Prepares all necessary resources and operations necessary to render the
    /// next frame.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let now = Instant::now();
        self.state.elapsed = self
            .last_rendered_at
            .map(|i| now.duration_since(i))
            .unwrap_or_default();
        self.last_rendered_at = Some(now);
        self.state.dynamic.redraw_target.set(RedrawTarget::Never);
        self.cushy.prepare(&mut self.state, device, queue);
    }

    /// Renders this window in a wgpu render pass created from `pass`.
    ///
    /// Returns the submission index of the last command submission, if any
    /// commands were submitted.
    pub fn render(
        &mut self,
        pass: &wgpu::RenderPassDescriptor<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<wgpu::SubmissionIndex> {
        self.render_with(pass, device, queue, None)
    }

    /// Renders this window in a wgpu render pass created from `pass`.
    ///
    /// Returns the submission index of the last command submission, if any
    /// commands were submitted.
    pub fn render_with(
        &mut self,
        pass: &wgpu::RenderPassDescriptor<'_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        additional_drawing: Option<&Drawing>,
    ) -> Option<wgpu::SubmissionIndex> {
        self.cushy
            .render_with(pass, device, queue, additional_drawing)
    }

    /// Renders this window into `texture` after performing `load_op`.
    pub fn render_into(
        &mut self,
        texture: &kludgine::Texture,
        load_op: wgpu::LoadOp<Color>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<wgpu::SubmissionIndex> {
        self.cushy.render_into(texture, load_op, device, queue)
    }

    /// Returns a new [`kludgine::Graphics`] context for this window.
    #[must_use]
    pub fn graphics<'gfx>(
        &'gfx mut self,
        device: &'gfx wgpu::Device,
        queue: &'gfx wgpu::Queue,
    ) -> kludgine::Graphics<'gfx> {
        self.cushy.graphics(device, queue)
    }

    /// Requests that the window close.
    ///
    /// Returns true if the request should be honored.
    pub fn request_close(&mut self) -> bool {
        if self.cushy.request_close(&mut self.state) {
            self.state.closed = true;
            true
        } else {
            self.state.dynamic.close_requested.set(false);
            false
        }
    }

    /// Sets the window's focused status.
    ///
    /// Being focused means that the window is expecting to be able to receive
    /// user input.
    pub fn set_focused(&mut self, focused: bool) {
        self.cushy.set_focused(focused);
    }

    /// Sets the window's occlusion status.
    ///
    /// This should only be set to true if the window is not visible at all to
    /// the end user due to being offscreen, minimized, or fully hidden behind
    /// other windows.
    pub fn set_occluded(&mut self, occluded: bool) {
        self.cushy.set_occluded(&&mut self.state, occluded);
    }

    /// Returns true if this window should no longer be open.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.state.closed
    }

    /// Returns a reference to the window's state.
    #[must_use]
    pub const fn state(&self) -> &VirtualState {
        &self.state
    }

    /// Returns the current size of the window.
    pub const fn size(&self) -> Size<UPx> {
        self.cushy.size()
    }

    /// Returns the current DPI scale of the window.
    pub const fn dpi_scale(&self) -> Fraction {
        self.cushy.dpi_scale()
    }

    /// Updates the dimensions and DPI scaling of the window.
    pub fn resize(
        &mut self,
        new_size: Size<UPx>,
        new_scale: impl Into<Fraction>,
        queue: &wgpu::Queue,
    ) {
        self.cushy.resize(
            &&mut self.state,
            new_size,
            new_scale,
            self.cushy.kludgine.zoom(),
            queue,
        );
    }

    /// Sets the window's position.
    pub fn set_position(&mut self, new_position: Point<Px>) {
        self.cushy.set_position(new_position);
    }

    /// Provide keyboard input to this virtual window.
    ///
    /// Returns whether the event was [`HANDLED`] or [`IGNORED`].
    pub fn keyboard_input(
        &mut self,
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
    ) -> EventHandling {
        self.cushy
            .keyboard_input(&mut self.state, device_id, input, is_synthetic)
    }

    /// Provides mouse wheel input to this window.
    ///
    /// Returns whether the event was [`HANDLED`] or [`IGNORED`].
    pub fn mouse_wheel(
        &mut self,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    ) -> EventHandling {
        self.cushy
            .mouse_wheel(&mut self.state, device_id, delta, phase)
    }

    /// Provides input manager events to this window.
    ///
    /// Returns whether the event was [`HANDLED`] or [`IGNORED`].
    pub fn ime(&mut self, ime: &Ime) -> EventHandling {
        self.cushy.ime(&mut self.state, ime)
    }

    /// Provides cursor movement events to this window.
    pub fn cursor_moved(&mut self, device_id: DeviceId, position: impl Into<Point<Px>>) {
        self.cushy
            .cursor_moved(&mut self.state, device_id, position);
    }

    /// Notifies the window that the cursor is no longer within the window.
    pub fn cursor_left(&mut self) {
        self.cushy.cursor_left(&mut self.state);
    }

    /// Provides mouse input events to tihs window.
    ///
    /// Returns whether the event was [`HANDLED`] or [`IGNORED`].
    pub fn mouse_input(
        &mut self,
        device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) -> EventHandling {
        self.cushy
            .mouse_input(&mut self.state, device_id, state, button)
    }
}

/// A color format containing 8-bit red, green, and blue channels.
pub struct Rgb8;

/// A color format containing 8-bit red, green, blue, and alpha channels.
pub struct Rgba8;

/// A format that can be captured in a [`VirtualRecorder`].
pub trait CaptureFormat: sealed::CaptureFormat {}

impl CaptureFormat for Rgb8 {}

impl sealed::CaptureFormat for Rgb8 {
    const HAS_ALPHA: bool = false;

    fn convert_rgba(data: &mut Vec<u8>, width: u32, bytes_per_row: u32) {
        let packed_width = width * 4;
        // Tightly pack the rgb data, discarding the alpha and extra padding.q
        let mut index = 0;
        data.retain(|_| {
            let retain = index % bytes_per_row < packed_width && index % 4 < 3;
            index += 1;
            retain
        });
    }

    fn load_image(data: &[u8], size: Size<UPx>) -> DynamicImage {
        DynamicImage::ImageRgb8(
            RgbImage::from_vec(size.width.get(), size.height.get(), data.to_vec())
                .expect("incorrect dimensions"),
        )
    }

    fn pixel_color(location: Point<UPx>, data: &[u8], size: Size<UPx>) -> Color {
        let pixel_offset = pixel_offset(data, location, size, 3);
        Color::new(pixel_offset[0], pixel_offset[1], pixel_offset[2], 255)
    }
}

fn pixel_offset(
    data: &[u8],
    location: Point<UPx>,
    size: Size<UPx>,
    bytes_per_component: u32,
) -> &[u8] {
    assert!(location.x < size.width && location.y < size.height);

    let width = size.width.get();
    let index = location.y.get() * width + location.x.get();
    &data[usize::try_from(index * bytes_per_component).expect("offset out of bounds")..]
}

impl CaptureFormat for Rgba8 {}

impl sealed::CaptureFormat for Rgba8 {
    const HAS_ALPHA: bool = true;

    fn convert_rgba(data: &mut Vec<u8>, width: u32, bytes_per_row: u32) {
        let packed_width = width * 4;
        if packed_width != bytes_per_row {
            // Tightly pack the rgba data
            let mut index = 0;
            data.retain(|_| {
                let retain = index % bytes_per_row < packed_width;
                index += 1;
                retain
            });
        }
    }

    fn load_image(data: &[u8], size: Size<UPx>) -> DynamicImage {
        DynamicImage::ImageRgba8(
            RgbaImage::from_vec(size.width.get(), size.height.get(), data.to_vec())
                .expect("incorrect dimensions"),
        )
    }

    fn pixel_color(location: Point<UPx>, data: &[u8], size: Size<UPx>) -> Color {
        let pixel_offset = pixel_offset(data, location, size, 4);
        Color::new(
            pixel_offset[0],
            pixel_offset[1],
            pixel_offset[2],
            pixel_offset[3],
        )
    }
}

/// A builder of a [`VirtualRecorder`].
pub struct VirtualRecorderBuilder<Format> {
    contents: WidgetInstance,
    size: Size<UPx>,
    scale: f32,
    format: PhantomData<Format>,
    resize_to_fit: bool,
}

impl VirtualRecorderBuilder<Rgb8> {
    /// Returns a builder of a [`VirtualRecorder`] that renders `contents`.
    pub fn new(contents: impl MakeWidget) -> Self {
        Self {
            contents: contents.make_widget(),
            size: Size::upx(800, 600),
            scale: 1.0,
            format: PhantomData,
            resize_to_fit: false,
        }
    }

    /// Enables transparency support to render the contents without a background
    /// color.
    #[must_use]
    pub fn with_alpha(self) -> VirtualRecorderBuilder<Rgba8> {
        VirtualRecorderBuilder {
            contents: self.contents,
            size: self.size,
            scale: self.scale,
            resize_to_fit: self.resize_to_fit,
            format: PhantomData,
        }
    }
}

impl<Format> VirtualRecorderBuilder<Format>
where
    Format: CaptureFormat,
{
    /// Sets the size of the virtual window.
    #[must_use]
    pub fn size<Unit>(mut self, size: Size<Unit>) -> Self
    where
        Unit: Into<UPx>,
    {
        self.size = size.map(Into::into);
        self
    }

    /// Sets the DPI scaling to apply to this virtual window.
    ///
    /// When scale is 1.0, resolution-independent content will be rendered at
    /// 96-ppi.
    ///
    /// This setting does not affect the image's pixel dimensions.
    #[must_use]
    pub fn scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    /// Sets this virtual recorder to allow updating its size based on the
    /// contents being rendered.
    #[must_use]
    pub fn resize_to_fit(mut self) -> Self {
        self.resize_to_fit = true;
        self
    }

    /// Returns an initialized [`VirtualRecorder`].
    pub fn finish(self) -> Result<VirtualRecorder<Format>, VirtualRecorderError> {
        VirtualRecorder::new(self.size, self.scale, self.resize_to_fit, self.contents)
    }
}

struct Capture {
    bytes: u64,
    bytes_per_row: u32,
    buffer: wgpu::Buffer,
    texture: Texture,
    multisample: Texture,
}

impl Capture {
    fn map_into<Format>(
        &self,
        buffer: &mut Vec<u8>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), wgpu::BufferAsyncError>
    where
        Format: CaptureFormat,
    {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.texture.copy_to_buffer(
            wgpu::ImageCopyBuffer {
                buffer: &self.buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(self.bytes_per_row),
                    rows_per_image: None,
                },
            },
            &mut encoder,
        );
        queue.submit([encoder.finish()]);

        let map_result = Arc::new(Mutex::new(None));
        let slice = self.buffer.slice(0..self.bytes);

        slice.map_async(wgpu::MapMode::Read, {
            let map_result = map_result.clone();
            move |result| {
                *map_result.lock() = Some(result);
            }
        });

        buffer.clear();
        buffer.reserve(self.bytes.cast());

        loop {
            device.poll(wgpu::Maintain::Poll);
            let mut result = map_result.lock();
            if let Some(result) = result.take() {
                result?;
                break;
            }
        }

        buffer.extend_from_slice(&slice.get_mapped_range());
        self.buffer.unmap();

        Format::convert_rgba(buffer, self.texture.size().width.get(), self.bytes_per_row);

        Ok(())
    }
}

/// A recorder of a [`VirtualWindow`].
pub struct VirtualRecorder<Format = Rgb8> {
    /// The virtual window being recorded.
    pub window: VirtualWindow,
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    capture: Option<Box<Capture>>,
    data: Vec<u8>,
    data_size: Size<UPx>,
    cursor: Dynamic<Point<Px>>,
    cursor_visible: bool,
    cursor_graphic: Drawing,
    format: PhantomData<Format>,
}

impl<Format> VirtualRecorder<Format>
where
    Format: CaptureFormat,
{
    /// Returns a new virtual recorder that renders `contents` into a graphic of
    /// `size`.
    ///
    /// `scale` adjusts the default DPI scaling to perform. It does not affect
    /// the `size`.
    pub fn new(
        size: Size<UPx>,
        scale: f32,
        resize_to_fit: bool,
        contents: impl MakeWidget,
    ) -> Result<Self, VirtualRecorderError> {
        let wgpu = wgpu::Instance::default();
        let adapter =
            pollster::block_on(wgpu.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .ok_or(VirtualRecorderError::NoAdapter)?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: Kludgine::REQURED_FEATURES,
                required_limits: Kludgine::adjust_limits(wgpu::Limits::downlevel_webgl2_defaults()),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
            },
            None,
        ))?;

        let window = contents
            .build_standalone_window()
            .size(size)
            .scale(scale)
            .transparent()
            .resize_to_fit(resize_to_fit)
            .finish_virtual(&device, &queue);

        let mut recorder = Self {
            window,
            device: Arc::new(device),
            queue: Arc::new(queue),
            cursor: Dynamic::default(),
            cursor_graphic: Drawing::default(),
            cursor_visible: false,
            capture: None,
            data: Vec::new(),
            data_size: Size::ZERO,
            format: PhantomData,
        };
        recorder.refresh()?;

        if resize_to_fit && recorder.window.state.size != recorder.window.size() {
            recorder.refresh()?;
        }
        Ok(recorder)
    }

    /// Returns the tightly-packed captured bytes.
    ///
    /// The layout of this data is determined by the `Format` generic.
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    /// Returns the color of the pixel at `location`.
    ///
    /// # Panics
    ///
    /// This function will panic if location is outside of the bounds of the
    /// captured image. When the window's size has been changed, this function
    /// operates on the size of the window when the last call to
    /// [`Self::refresh()`] was made.
    pub fn pixel_color<Unit>(&self, location: Point<Unit>) -> Color
    where
        Unit: Into<UPx>,
    {
        Format::pixel_color(location.map(Into::into), self.bytes(), self.data_size)
    }

    /// Asserts that the color of the pixel at `location` is `expected`.
    ///
    /// This function allows for slight color variations. This is because of how
    /// colorspace corrections can lead to rounding errors.
    ///
    /// # Panics
    ///
    /// This function panics if the color is not the expected color.
    #[track_caller]
    pub fn assert_pixel_color<Unit>(&self, location: Point<Unit>, expected: Color, component: &str)
    where
        Unit: Into<UPx>,
    {
        let location = location.map(Into::into);
        let color = self.pixel_color(location);
        let max_delta = color
            .red()
            .abs_diff(expected.red())
            .max(color.green().abs_diff(expected.green()))
            .max(color.blue().abs_diff(expected.blue()))
            .max(color.alpha().abs_diff(expected.alpha()));
        assert!(
            max_delta <= 1,
            "assertion failed: {component} at {location:?} was {color:?}, not {expected:?}"
        );
    }

    /// Returns the current contents as an image.
    pub fn image(&self) -> DynamicImage {
        Format::load_image(self.bytes(), self.data_size)
    }

    fn recreate_buffers_if_needed(&mut self, size: Size<UPx>, bytes: u64, bytes_per_row: u32) {
        if self
            .capture
            .as_ref()
            .map_or(true, |capture| capture.texture.size() != size)
        {
            let texture = Texture::new(
                &self.window.graphics(&self.device, &self.queue),
                size,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                wgpu::FilterMode::Linear,
            );
            let multisample = Texture::multisampled(
                &self.window.graphics(&self.device, &self.queue),
                4,
                size,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                wgpu::FilterMode::Linear,
            );
            let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: bytes,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            self.capture = Some(Box::new(Capture {
                bytes,
                bytes_per_row,
                buffer,
                texture,
                multisample,
            }));
        }
    }

    fn redraw(&mut self) {
        let mut render_size = self.window.size().ceil();
        if self.window.state.size != render_size {
            let current_scale = self.window.dpi_scale();
            self.window
                .resize(self.window.state.size, current_scale, &self.queue);
            render_size = self.window.state.size;
        }
        let bytes_per_row = copy_buffer_aligned_bytes_per_row(render_size.width.get() * 4);
        let size = u64::from(bytes_per_row) * u64::from(render_size.height.get());
        self.recreate_buffers_if_needed(render_size, size, bytes_per_row);

        let capture = self.capture.as_ref().assert("always initialized above");

        if self.cursor_visible {
            let mut gfx = self.window.graphics(&self.device, &self.queue);
            let mut frame = self.cursor_graphic.new_frame(&mut gfx);
            frame.draw_shape(
                Shape::filled_circle(Px::new(4), Color::WHITE, Origin::Center)
                    .translate_by(self.cursor.get()),
            );
            drop(frame);
        }

        self.window.prepare(&self.device, &self.queue);

        self.window.render_with(
            &wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: capture.multisample.view(),
                    resolve_target: Some(capture.texture.view()),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color::CLEAR_BLACK.into()),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            },
            &self.device,
            &self.queue,
            self.cursor_visible.then_some(&self.cursor_graphic),
        );
    }

    /// Redraws the contents.
    pub fn refresh(&mut self) -> Result<(), wgpu::BufferAsyncError> {
        self.redraw();

        let capture = self.capture.as_ref().assert("always initialized above");

        capture.map_into::<Format>(&mut self.data, &self.device, &self.queue)?;
        self.data_size = capture.texture.size();

        Ok(())
    }

    /// Sets the cursor position immediately.
    pub fn set_cursor_position(&self, position: Point<Px>) {
        self.cursor.set(position);
    }

    /// Enables or disables drawing of the virtual cursor.
    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    /// Begins recording an animated png.
    pub fn record_animated_png(&mut self, target_fps: u8) -> AnimationRecorder<'_, Format> {
        AnimationRecorder {
            target_fps,
            assembler: Some(FrameAssembler::spawn::<Format>(
                self.device.clone(),
                self.queue.clone(),
            )),
            recorder: self,
        }
    }

    /// Returns a recorder that does not store any rendered frames.
    pub fn simulate_animation(&mut self) -> AnimationRecorder<'_, Format> {
        AnimationRecorder {
            target_fps: 0,
            assembler: None,
            recorder: self,
        }
    }
}

fn copy_buffer_aligned_bytes_per_row(width: u32) -> u32 {
    (width + COPY_BYTES_PER_ROW_ALIGNMENT - 1) / COPY_BYTES_PER_ROW_ALIGNMENT
        * COPY_BYTES_PER_ROW_ALIGNMENT
}

/// An animated PNG recorder.
pub struct AnimationRecorder<'a, Format> {
    recorder: &'a mut VirtualRecorder<Format>,
    target_fps: u8,
    assembler: Option<FrameAssembler>,
}

impl<Format> AnimationRecorder<'_, Format>
where
    Format: CaptureFormat,
{
    /// Animates the cursor to move from its current location to `location`.
    pub fn animate_cursor_to(
        &mut self,
        location: Point<Px>,
        over: Duration,
        easing: impl Easing,
    ) -> Result<(), VirtualRecorderError> {
        self.recorder
            .cursor
            .transition_to(location)
            .over(over)
            .with_easing(easing)
            .launch();
        self.wait_for(over)
    }

    /// Animates pressing and releasing a mouse button at the current cursor
    /// location.
    pub fn animate_mouse_button(
        &mut self,
        button: MouseButton,
        duration: Duration,
    ) -> Result<(), VirtualRecorderError> {
        let _ =
            self.recorder
                .window
                .mouse_input(DeviceId::Virtual(0), ElementState::Pressed, button);

        self.wait_for(duration)?;
        let _ =
            self.recorder
                .window
                .mouse_input(DeviceId::Virtual(0), ElementState::Released, button);
        Ok(())
    }

    /// Simulates a key down and key up event with the given information.
    pub fn animate_keypress(
        &mut self,
        physical_key: PhysicalKey,
        logical_key: Key,
        text: Option<&str>,
        duration: Duration,
    ) -> Result<(), VirtualRecorderError> {
        let text = text.map(SmolStr::new);
        let half_duration = duration / 2;
        let mut event = KeyEvent {
            physical_key,
            logical_key,
            text,
            state: ElementState::Pressed,
            repeat: false,
            location: KeyLocation::Standard,
            modifiers: Modifiers::default(),
        };
        self.recorder
            .window
            .keyboard_input(DeviceId::Virtual(0), event.clone(), true);
        self.wait_for(half_duration)?;
        event.state = ElementState::Released;
        self.recorder
            .window
            .keyboard_input(DeviceId::Virtual(0), event.clone(), true);

        self.wait_for(half_duration)
    }

    /// Animates entering the graphemes from `text` over `duration`.
    pub fn animate_text_input(
        &mut self,
        text: &str,
        duration: Duration,
    ) -> Result<(), VirtualRecorderError> {
        let graphemes = text.graphemes(true).count();
        let delay_per_event =
            Duration::from_nanos(duration.as_nanos().cast::<u64>() / graphemes.cast::<u64>() / 2);
        for grapheme in text.graphemes(true) {
            let grapheme = SmolStr::new(grapheme);
            let mut event = KeyEvent {
                physical_key: PhysicalKey::Unidentified(NativeKeyCode::Xkb(0)),
                logical_key: Key::Character(grapheme.clone()),
                text: Some(SmolStr::new(grapheme)),
                location: KeyLocation::Standard,
                state: ElementState::Pressed,
                repeat: false,
                modifiers: Modifiers::default(),
            };
            let _handled =
                self.recorder
                    .window
                    .keyboard_input(DeviceId::Virtual(0), event.clone(), true);
            self.wait_for(delay_per_event)?;

            event.state = ElementState::Released;
            let _handled = self
                .recorder
                .window
                .keyboard_input(DeviceId::Virtual(0), event, true);
            self.wait_for(delay_per_event)?;
        }
        Ok(())
    }

    /// Waits for `duration`, rendering frames as needed.
    pub fn wait_for(&mut self, duration: Duration) -> Result<(), VirtualRecorderError> {
        self.wait_until(Instant::now() + duration)
    }

    /// Waits until `time`, rendering frames as needed.
    pub fn wait_until(&mut self, time: Instant) -> Result<(), VirtualRecorderError> {
        let Some(assembler) = self.assembler.as_ref() else {
            return Ok(());
        };

        let frame_duration = Duration::from_micros(1_000_000 / u64::from(self.target_fps));
        let mut last_frame = Instant::now();

        loop {
            let now = Instant::now();
            let final_frame = now > time;

            self.recorder
                .window
                .cursor_moved(DeviceId::Virtual(0), self.recorder.cursor.get());

            let next_frame = match self.recorder.window.state.dynamic.redraw_target.get() {
                RedrawTarget::Never => now + frame_duration,
                RedrawTarget::Now => now,
                RedrawTarget::At(instant) => now.min(instant),
            };

            if final_frame || next_frame <= now {
                // Try to reuse an existing capture instead of forcing an
                // allocation.
                if let Ok(capture) = assembler.resuable_captures.try_recv() {
                    self.recorder.capture = Some(capture);
                }
                let elapsed = now.saturating_duration_since(last_frame);
                last_frame = now;
                self.recorder.redraw();
                let capture = self.recorder.capture.take().assert("always present");
                if assembler.sender.send((capture, elapsed)).is_err() {
                    break;
                }
            }

            if final_frame {
                break;
            }

            let render_duration = now.elapsed();
            std::thread::sleep(frame_duration.saturating_sub(render_duration));
        }

        Ok(())
    }

    /// Encodes the currently recorded frames into a new file at `path`.
    ///
    /// If this animation was created from
    /// [`VirtualRecorder::simulate_animation`], this function will do nothing.
    pub fn write_to(self, path: impl AsRef<Path>) -> Result<(), VirtualRecorderError> {
        let Some(frames) = self.assembler.map(FrameAssembler::finish).transpose()? else {
            return Ok(());
        };
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        let mut encoder = png::Encoder::new(
            &mut file,
            self.recorder.window.size().width.get(),
            self.recorder.window.size().height.get(),
        );
        encoder.set_color(if Format::HAS_ALPHA {
            png::ColorType::Rgba
        } else {
            png::ColorType::Rgb
        });
        encoder.set_adaptive_filter(png::AdaptiveFilterType::Adaptive);
        encoder.set_animated(u32::try_from(frames.len()).assert("too many frames"), 0)?;
        encoder.set_compression(png::Compression::Best);

        let mut current_frame_delay = Duration::ZERO;
        let mut writer = encoder.write_header()?;
        for frame in &frames {
            if current_frame_delay != frame.duration && frames.len() > 1 {
                current_frame_delay = frame.duration;
                // This has a limitation that a single frame can't be longer
                // than ~6.5 seconds, but it ensures frame timing is more
                // accurate.
                writer.set_frame_delay(
                    u16::try_from(current_frame_delay.as_nanos() / 100_000).unwrap_or(u16::MAX),
                    10_000,
                )?;
            }
            writer.write_image_data(&frame.data)?;
        }

        writer.finish()?;

        file.sync_all()?;

        Ok(())
    }
}

struct Frame {
    data: Vec<u8>,
    duration: Duration,
}

/// An error from a [`VirtualRecorder`].
#[derive(Debug)]
pub enum VirtualRecorderError {
    /// No compatible wgpu adapters could be found.
    NoAdapter,
    /// An error occurred requesting a device.
    RequestDevice(wgpu::RequestDeviceError),
    /// The capture texture dimensions are too large to fit in the current host
    /// platform's memory.
    TooLarge,
    /// An error occurred trying to read a buffer.
    MapBuffer(wgpu::BufferAsyncError),
    /// An error occurred encoding a png image.
    PngEncode(png::EncodingError),
}

impl From<png::EncodingError> for VirtualRecorderError {
    fn from(value: png::EncodingError) -> Self {
        Self::PngEncode(value)
    }
}

impl From<wgpu::RequestDeviceError> for VirtualRecorderError {
    fn from(value: wgpu::RequestDeviceError) -> Self {
        Self::RequestDevice(value)
    }
}

impl From<wgpu::BufferAsyncError> for VirtualRecorderError {
    fn from(value: wgpu::BufferAsyncError) -> Self {
        Self::MapBuffer(value)
    }
}

impl From<TryFromIntError> for VirtualRecorderError {
    fn from(_: TryFromIntError) -> Self {
        Self::TooLarge
    }
}

impl From<io::Error> for VirtualRecorderError {
    fn from(value: io::Error) -> Self {
        Self::PngEncode(value.into())
    }
}

impl std::fmt::Display for VirtualRecorderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VirtualRecorderError::NoAdapter => {
                f.write_str("no compatible graphics adapters were found")
            }
            VirtualRecorderError::RequestDevice(err) => {
                write!(f, "error requesting graphics device: {err}")
            }
            VirtualRecorderError::TooLarge => {
                f.write_str("the rendered surface is too large for this cpu architecture")
            }
            VirtualRecorderError::MapBuffer(err) => {
                write!(f, "error reading rendered graphics data: {err}")
            }
            VirtualRecorderError::PngEncode(err) => write!(f, "error encoding png: {err}"),
        }
    }
}

impl std::error::Error for VirtualRecorderError {}

/// A unique identifier of an input device.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DeviceId {
    /// A winit-supplied device id.
    Winit(winit::event::DeviceId),
    /// A simulated device.
    Virtual(u64),
}

impl From<winit::event::DeviceId> for DeviceId {
    fn from(value: winit::event::DeviceId) -> Self {
        Self::Winit(value)
    }
}

struct FrameAssembler {
    sender: mpsc::SyncSender<(Box<Capture>, Duration)>,
    result: mpsc::Receiver<Result<Vec<Frame>, VirtualRecorderError>>,
    resuable_captures: mpsc::Receiver<Box<Capture>>,
}

impl FrameAssembler {
    fn spawn<Format>(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self
    where
        Format: CaptureFormat,
    {
        let (frame_sender, frame_receiver) = mpsc::sync_channel(1000);
        let (finished_frame_sender, finished_frame_receiver) = mpsc::sync_channel(600);
        let (result_sender, result_receiver) = mpsc::sync_channel(1);

        std::thread::spawn(move || {
            Self::assembler_thread::<Format>(
                &frame_receiver,
                &result_sender,
                &finished_frame_sender,
                &device,
                &queue,
            );
        });

        Self {
            sender: frame_sender,
            result: result_receiver,
            resuable_captures: finished_frame_receiver,
        }
    }

    fn finish(self) -> Result<Vec<Frame>, VirtualRecorderError> {
        drop(self.sender);
        self.result.recv().assert("thread panicked")
    }

    fn assembler_thread<Format>(
        frames: &mpsc::Receiver<(Box<Capture>, Duration)>,
        result: &mpsc::SyncSender<Result<Vec<Frame>, VirtualRecorderError>>,
        reusable: &mpsc::SyncSender<Box<Capture>>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) where
        Format: CaptureFormat,
    {
        let mut assembled = Vec::<Frame>::new();
        let mut buffer = Vec::new();
        while let Ok((capture, elapsed)) = frames.recv() {
            if let Err(err) = capture.map_into::<Format>(&mut buffer, device, queue) {
                let _result = result.send(Err(err.into()));
                return;
            }
            match assembled.last_mut() {
                Some(frame) if frame.data == buffer => {
                    frame.duration += elapsed;
                }
                _ => {
                    assembled.push(Frame {
                        data: std::mem::take(&mut buffer),
                        duration: elapsed,
                    });
                }
            }
            let _result = reusable.try_send(capture);
        }

        let _result = result.send(Ok(assembled));
    }
}

/// Describes a keyboard input targeting a window.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct KeyEvent {
    /// The logical key that is interpretted from the `physical_key`.
    ///
    /// See [`KeyEvent::logical_key`](winit::event::KeyEvent::logical_key) for
    /// more information.
    pub logical_key: Key,
    /// The physical key that caused this event.
    ///
    /// See [`KeyEvent::logical_key`](winit::event::KeyEvent::physical_key) for
    /// more information.
    pub physical_key: PhysicalKey,

    /// The text being input by this event, if any.
    ///
    /// See [`KeyEvent::logical_key`](winit::event::KeyEvent::text) for
    /// more information.
    pub text: Option<SmolStr>,

    /// The physical location of the key being presed.
    ///
    /// See [`KeyEvent::logical_key`](winit::event::KeyEvent::location) for
    /// more information.
    pub location: KeyLocation,

    /// The state of this key for this event.
    ///
    /// See [`KeyEvent::logical_key`](winit::event::KeyEvent::state) for
    /// more information.
    pub state: ElementState,

    /// If true, this event was caused by a key being repeated.
    ///
    /// See [`KeyEvent::logical_key`](winit::event::KeyEvent::logical_key) for
    /// more information.
    pub repeat: bool,

    /// The modifiers state active for this event.
    pub modifiers: Modifiers,
}

impl KeyEvent {
    /// Returns a new key event from a winit key event and modifiers.
    #[must_use]
    pub fn from_winit(event: winit::event::KeyEvent, modifiers: Modifiers) -> Self {
        Self {
            physical_key: event.physical_key,
            logical_key: event.logical_key,
            text: event.text,
            location: event.location,
            state: event.state,
            repeat: event.repeat,
            modifiers,
        }
    }
}
