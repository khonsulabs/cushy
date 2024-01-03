//! Types for displaying a [`Widget`] inside of a desktop window.

use std::cell::RefCell;
use std::collections::hash_map;
use std::ffi::OsStr;
use std::hash::Hash;
use std::marker::PhantomData;
use std::num::TryFromIntError;
use std::ops::{Deref, DerefMut, Not};
use std::path::Path;
use std::string::ToString;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use ahash::AHashMap;
use alot::LotId;
use arboard::Clipboard;
use figures::units::{Px, UPx};
use figures::{IntoSigned, IntoUnsigned, Point, Ranged, Rect, Round, ScreenScale, Size, Zero};
use intentional::{Assert, Cast};
use kludgine::app::winit::dpi::{PhysicalPosition, PhysicalSize};
use kludgine::app::winit::event::{
    DeviceId, ElementState, Ime, KeyEvent, Modifiers, MouseButton, MouseScrollDelta, TouchPhase,
};
use kludgine::app::winit::keyboard::{Key, NamedKey};
use kludgine::app::winit::window::{self, CursorIcon};
use kludgine::app::{winit, WindowBehavior as _};
use kludgine::cosmic_text::{fontdb, Family, FamilyOwned};
use kludgine::render::Drawing;
use kludgine::wgpu::{self, CompositeAlphaMode, COPY_BYTES_PER_ROW_ALIGNMENT};
use kludgine::{Color, Kludgine, KludgineId, Texture};
use tracing::Level;

use crate::animation::{LinearInterpolate, PercentBetween, ZeroToOne};
use crate::app::{Application, Cushy, Open, PendingApp, Run};
use crate::context::sealed::InvalidationStatus;
use crate::context::{
    AsEventContext, EventContext, Exclusive, GraphicsContext, LayoutContext, Trackable,
    WidgetContext,
};
use crate::graphics::{FontState, Graphics};
use crate::styles::{Edges, FontFamilyList, ThemePair};
use crate::tree::Tree;
use crate::utils::ModifiersExt;
use crate::value::{
    Destination, Dynamic, DynamicReader, Generation, IntoDynamic, IntoValue, Source, Value,
};
use crate::widget::{
    EventHandling, MakeWidget, MountedWidget, OnceCallback, RootBehavior, Widget, WidgetId,
    WidgetInstance, HANDLED, IGNORED,
};
use crate::window::sealed::WindowCommand;
use crate::{initialize_tracing, ConstraintLimit};

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
    fn set_cursor_icon(&mut self, cursor: CursorIcon);
    /// Returns a handle for the window.
    fn handle(&self, redraw_status: InvalidationStatus) -> WindowHandle;
    /// Returns the current inner size of the window.
    fn inner_size(&self) -> Size<UPx>;

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
}

impl PlatformWindowImplementation for kludgine::app::Window<'_, WindowCommand> {
    fn set_cursor_icon(&mut self, cursor: CursorIcon) {
        self.winit().set_cursor_icon(cursor);
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
    /// Returns the underlying `winit` window, if one exists.
    fn winit(&self) -> Option<&winit::window::Window>;
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
    fn set_cursor_icon(&mut self, cursor: CursorIcon);

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
}

impl<W> RunningWindow<W>
where
    W: PlatformWindowImplementation,
{
    pub(crate) fn new(
        window: W,
        kludgine_id: KludgineId,
        invalidation_status: &InvalidationStatus,
        cushy: &Cushy,
        focused: &Dynamic<bool>,
        occluded: &Dynamic<bool>,
        inner_size: &Dynamic<Size<UPx>>,
    ) -> Self {
        Self {
            window,
            kludgine_id,
            invalidation_status: invalidation_status.clone(),
            cushy: cushy.clone(),
            focused: focused.clone(),
            occluded: occluded.clone(),
            inner_size: inner_size.clone(),
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

    fn winit(&self) -> Option<&winit::window::Window> {
        self.window.winit()
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

    fn set_cursor_icon(&mut self, cursor: CursorIcon) {
        self.window.set_cursor_icon(cursor);
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
}

/// The attributes of a Cushy window.
pub type WindowAttributes = kludgine::app::WindowAttributes;

/// A Cushy window that is not yet running.
#[must_use]
pub struct Window<Behavior = WidgetInstance>
where
    Behavior: WindowBehavior,
{
    context: Behavior::Context,
    pending: PendingWindow,
    /// The attributes of this window.
    pub attributes: WindowAttributes,
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
    /// A list of data buffers that contain font data to ensure are loaded
    /// during drawing operations.
    pub font_data_to_load: Vec<Vec<u8>>,

    on_closed: Option<OnceCallback>,
    inner_size: Option<Dynamic<Size<UPx>>>,
    occluded: Option<Dynamic<bool>>,
    focused: Option<Dynamic<bool>>,
    theme_mode: Option<Value<ThemeMode>>,
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

impl Window<WidgetInstance> {
    /// Returns a new instance using `widget` as its contents.
    pub fn for_widget<W>(widget: W) -> Self
    where
        W: Widget,
    {
        Self::new(WidgetInstance::new(widget))
    }

    /// Sets `focused` to be the dynamic updated when this window's focus status
    /// is changed.
    ///
    /// When the window is focused for user input, the dynamic will contain
    /// `true`.
    ///
    /// `focused` will be initialized with an initial state
    /// of `false`.
    pub fn focused(mut self, focused: impl IntoDynamic<bool>) -> Self {
        let focused = focused.into_dynamic();
        focused.set(false);
        self.focused = Some(focused);
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

    /// Sets `inner_size` to be the dynamic syncrhonized with this window's
    /// inner size.
    ///
    /// When the window is resized, the dynamic will contain its new size. When
    /// the dynamic is updated with a new value, a resize request will be made
    /// with the new inner size.
    pub fn inner_size(mut self, inner_size: impl IntoDynamic<Size<UPx>>) -> Self {
        let inner_size = inner_size.into_dynamic();
        self.inner_size = Some(inner_size);
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
    pub fn loading_font(mut self, font_data: Vec<u8>) -> Self {
        self.font_data_to_load.push(font_data);
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

    /// Sets the window's title.
    pub fn titled(mut self, title: impl IntoValue<String>) -> Self {
        self.title = title.into_value();
        self
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
            font_data_to_load: vec![
                #[cfg(feature = "roboto-flex")]
                include_bytes!("../assets/RobotoFlex.ttf").to_vec(),
            ],
        }
    }
}

impl<Behavior> Run for Window<Behavior>
where
    Behavior: WindowBehavior,
{
    fn run(self) -> crate::Result {
        initialize_tracing();
        let app = PendingApp::default();
        self.open(&app)?;
        app.run()
    }
}

impl<Behavior> Open for Window<Behavior>
where
    Behavior: WindowBehavior,
{
    fn open<App>(self, app: &App) -> crate::Result<Option<WindowHandle>>
    where
        App: Application + ?Sized,
    {
        let cushy = app.cushy().clone();
        // let Some(app) = app.as_app().as_kludgine() else {
        //     return Ok(None);
        // };
        let handle = CushyWindow::<Behavior>::open_with(
            app,
            sealed::Context {
                user: self.context,
                settings: RefCell::new(sealed::WindowSettings {
                    cushy,
                    title: self.title,
                    redraw_status: self.pending.0.redraw_status.clone(),
                    on_closed: self.on_closed,
                    transparent: self.attributes.transparent,
                    attributes: Some(self.attributes),
                    occluded: self.occluded.unwrap_or_default(),
                    focused: self.focused.unwrap_or_default(),
                    inner_size: self.inner_size.unwrap_or_default(),
                    theme: Some(self.theme),
                    theme_mode: self.theme_mode,
                    font_data_to_load: self.font_data_to_load,
                    serif_font_family: self.serif_font_family,
                    sans_serif_font_family: self.sans_serif_font_family,
                    fantasy_font_family: self.fantasy_font_family,
                    monospace_font_family: self.monospace_font_family,
                    cursive_font_family: self.cursive_font_family,
                }),
            },
        )?;

        Ok(handle.map(|handle| self.pending.opened(handle)))
    }

    fn run_in(self, app: PendingApp) -> crate::Result {
        self.open(&app)?;
        app.run()
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
    fn close_requested<W>(&self, window: &mut RunningWindow<W>) -> bool
    where
        W: PlatformWindowImplementation,
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

struct CushyWindow<T> {
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
    inner_size: Dynamic<Size<UPx>>,
    inner_size_generation: Generation,
    keyboard_activated: Option<WidgetId>,
    min_inner_size: Option<Size<UPx>>,
    max_inner_size: Option<Size<UPx>>,
    theme: Option<DynamicReader<ThemePair>>,
    current_theme: ThemePair,
    theme_mode: Value<ThemeMode>,
    transparent: bool,
    fonts: FontState,
    cushy: Cushy,
    on_closed: Option<OnceCallback>,
}

impl<T> CushyWindow<T>
where
    T: WindowBehavior,
{
    fn request_close(
        should_close: &mut bool,
        behavior: &mut T,
        window: &mut RunningWindow<kludgine::app::Window<'_, WindowCommand>>,
    ) -> bool {
        *should_close |= behavior.close_requested(window);

        *should_close
    }

    fn keyboard_activate_widget(
        &mut self,
        is_pressed: bool,
        widget: Option<LotId>,
        window: &mut RunningWindow<kludgine::app::Window<'_, WindowCommand>>,
        kludgine: &mut Kludgine,
    ) {
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
        fontdb: &mut fontdb::Database,
    ) -> FontState {
        for font_to_load in settings.font_data_to_load.drain(..) {
            fontdb.load_font_data(font_to_load);
        }

        let fonts = FontState::new(fontdb);
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

    fn handle_window_keyboard_input(
        &mut self,
        window: &mut RunningWindow<kludgine::app::Window<'_, WindowCommand>>,
        kludgine: &mut Kludgine,
        input: KeyEvent,
    ) {
        match input.logical_key {
            Key::Character(ch) if ch == "w" && window.modifiers().primary() => {
                if input.state.is_pressed()
                    && Self::request_close(&mut self.should_close, &mut self.behavior, window)
                {
                    window.set_needs_redraw();
                }
            }
            Key::Named(NamedKey::Space) if !window.modifiers().possible_shortcut() => {
                let target = self.tree.focused_widget().unwrap_or(self.root.node_id);
                let target = self.tree.widget_from_node(target).expect("missing widget");
                let mut target = EventContext::new(
                    WidgetContext::new(
                        target,
                        &self.current_theme,
                        window,
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
            }
            Key::Named(NamedKey::Enter) => {
                self.keyboard_activate_widget(
                    input.state.is_pressed(),
                    self.tree.default_widget(),
                    window,
                    kludgine,
                );
            }
            Key::Named(NamedKey::Escape) => {
                self.keyboard_activate_widget(
                    input.state.is_pressed(),
                    self.tree.escape_widget(),
                    window,
                    kludgine,
                );
            }
            _ => {
                tracing::event!(
                    Level::DEBUG,
                    logical = ?input.logical_key,
                    physical = ?input.physical_key,
                    state = ?input.state,
                    "Ignored Keyboard Input",
                );
            }
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn new<W>(
        mut behavior: T,
        window: W,
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
        let occluded = settings.occluded.clone();
        let focused = settings.focused.clone();
        let theme = settings.theme.take().unwrap_or_default();
        let inner_size = settings.inner_size.clone();
        let on_closed = settings.on_closed.take();

        inner_size.set(window.inner_size());

        let fonts = Self::load_fonts(&mut settings, graphics.font_system().db_mut());

        let theme_mode = match settings.theme_mode.take() {
            Some(Value::Dynamic(dynamic)) => {
                dynamic.set(window.theme().into());
                Value::Dynamic(dynamic)
            }
            Some(Value::Constant(mode)) => Value::Constant(mode),
            None => Value::dynamic(window.theme().into()),
        };
        let transparent = settings.transparent;

        let tree = Tree::default();
        let root = tree.push_boxed(behavior.make_root(), None);

        let (current_theme, theme) = match theme {
            Value::Constant(theme) => (theme, None),
            Value::Dynamic(dynamic) => (dynamic.get(), Some(dynamic.into_reader())),
        };

        Self {
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
            occluded,
            focused,
            inner_size_generation: inner_size.generation(),
            inner_size,
            keyboard_activated: None,
            min_inner_size: None,
            max_inner_size: None,
            current_theme,
            theme,
            theme_mode,
            transparent,
            fonts,
            cushy,
            on_closed,
        }
    }

    fn prepare<W>(&mut self, window: W, graphics: &mut kludgine::Graphics<'_>)
    where
        W: PlatformWindowImplementation,
    {
        if let Some(theme) = &mut self.theme {
            if theme.has_updated() {
                self.current_theme = theme.get();
                self.root.invalidate();
            }
        }

        self.redraw_status.refresh_received();
        graphics.reset_text_attributes();
        self.tree
            .new_frame(self.redraw_status.invalidations().drain());

        let resizable = window.is_resizable();
        let mut window = RunningWindow::new(
            window,
            graphics.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            &self.inner_size,
        );
        let root_mode = self.constrain_window_resizing(resizable, &mut window, graphics);

        self.fonts.next_frame();
        let graphics = self.contents.new_frame(graphics);
        let mut context = GraphicsContext {
            widget: WidgetContext::new(
                self.root.clone(),
                &self.current_theme,
                &mut window,
                self.theme_mode.get(),
                &mut self.cursor,
            ),
            gfx: Exclusive::Owned(Graphics::new(graphics, &mut self.fonts)),
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
        layout_context.redraw_when_changed(&self.inner_size);
        let inner_size_generation = self.inner_size.generation();
        if self.inner_size_generation != inner_size_generation {
            layout_context.request_inner_size(self.inner_size.get());
            self.inner_size_generation = inner_size_generation;
        } else if actual_size != window_size && !resizable {
            let mut new_size = actual_size;
            if let Some(min_size) = self.min_inner_size {
                new_size = new_size.max(min_size);
            }
            if let Some(max_size) = self.max_inner_size {
                new_size = new_size.min(max_size);
            }
            layout_context.request_inner_size(new_size);
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
        if self.behavior.close_requested(&mut RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            &self.inner_size,
        )) {
            self.should_close = true;
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
enum RootMode {
    Fit,
    Expand,
    Align,
}

impl<T> kludgine::app::WindowBehavior<WindowCommand> for CushyWindow<T>
where
    T: WindowBehavior,
{
    type Context = sealed::Context<T::Context>;

    fn initialize(
        window: kludgine::app::Window<'_, WindowCommand>,
        graphics: &mut kludgine::Graphics<'_>,
        context: Self::Context,
    ) -> Self {
        let settings = context.settings.borrow_mut();
        let mut window = RunningWindow::new(
            window,
            graphics.id(),
            &settings.redraw_status,
            &settings.cushy,
            &settings.focused,
            &settings.occluded,
            &settings.inner_size,
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

    fn focus_changed(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        _kludgine: &mut Kludgine,
    ) {
        self.focused.set(window.focused());
    }

    fn occlusion_changed(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        _kludgine: &mut Kludgine,
    ) {
        self.occluded.set(window.ocluded());
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
        attrs
    }

    fn close_requested(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
    ) -> bool {
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
                &self.inner_size,
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

    // fn scale_factor_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    fn resized(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        _kludgine: &mut Kludgine,
    ) {
        self.inner_size.set(window.inner_size());
        // We want to prevent a resize request for this resized event.
        self.inner_size_generation = self.inner_size.generation();
        self.root.invalidate();
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
        device_id: DeviceId,
        input: KeyEvent,
        is_synthetic: bool,
    ) {
        let target = self.tree.focused_widget().unwrap_or(self.root.node_id);
        let Some(target) = self.tree.widget_from_node(target) else {
            return;
        };
        let mut window = RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            &self.inner_size,
        );
        let mut target = EventContext::new(
            WidgetContext::new(
                target,
                &self.current_theme,
                &mut window,
                self.theme_mode.get(),
                &mut self.cursor,
            ),
            kludgine,
        );

        let handled = recursively_handle_event(&mut target, |widget| {
            widget.keyboard_input(device_id, input.clone(), is_synthetic)
        })
        .is_some();
        drop(target);

        if !handled {
            self.handle_window_keyboard_input(&mut window, kludgine, input);
        }
    }

    fn mouse_wheel(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        delta: MouseScrollDelta,
        phase: TouchPhase,
    ) {
        let widget = self
            .tree
            .hovered_widget()
            .and_then(|hovered| self.tree.widget_from_node(hovered))
            .unwrap_or_else(|| self.tree.widget(self.root.id()).expect("missing widget"));

        let mut window = RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            &self.inner_size,
        );
        let mut widget = EventContext::new(
            WidgetContext::new(
                widget,
                &self.current_theme,
                &mut window,
                self.theme_mode.get(),
                &mut self.cursor,
            ),
            kludgine,
        );
        recursively_handle_event(&mut widget, |widget| {
            widget.mouse_wheel(device_id, delta, phase)
        });
    }

    // fn modifiers_changed(&mut self, window: kludgine::app::Window<'_, ()>) {}

    fn ime(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        ime: Ime,
    ) {
        let widget = self
            .tree
            .focused_widget()
            .and_then(|hovered| self.tree.widget_from_node(hovered))
            .unwrap_or_else(|| self.tree.widget(self.root.id()).expect("missing widget"));
        let mut window = RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            &self.inner_size,
        );
        let mut target = EventContext::new(
            WidgetContext::new(
                widget,
                &self.current_theme,
                &mut window,
                self.theme_mode.get(),
                &mut self.cursor,
            ),
            kludgine,
        );

        let _handled =
            recursively_handle_event(&mut target, |widget| widget.ime(ime.clone())).is_some();
    }

    fn cursor_moved(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        position: PhysicalPosition<f64>,
    ) {
        let location = Point::<Px>::from(position);
        self.cursor.location = Some(location);

        let mut window = RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            &self.inner_size,
        );

        EventContext::new(
            WidgetContext::new(
                self.root.clone(),
                &self.current_theme,
                &mut window,
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

    fn cursor_left(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        _device_id: DeviceId,
    ) {
        if self.cursor.widget.take().is_some() {
            let mut window = RunningWindow::new(
                window,
                kludgine.id(),
                &self.redraw_status,
                &self.cushy,
                &self.focused,
                &self.occluded,
                &self.inner_size,
            );
            let mut context = EventContext::new(
                WidgetContext::new(
                    self.root.clone(),
                    &self.current_theme,
                    &mut window,
                    self.theme_mode.get(),
                    &mut self.cursor,
                ),
                kludgine,
            );
            context.clear_hover();
        }
    }

    fn mouse_input(
        &mut self,
        window: kludgine::app::Window<'_, WindowCommand>,
        kludgine: &mut Kludgine,
        device_id: DeviceId,
        state: ElementState,
        button: MouseButton,
    ) {
        let mut window = RunningWindow::new(
            window,
            kludgine.id(),
            &self.redraw_status,
            &self.cushy,
            &self.focused,
            &self.occluded,
            &self.inner_size,
        );
        match state {
            ElementState::Pressed => {
                EventContext::new(
                    WidgetContext::new(
                        self.root.clone(),
                        &self.current_theme,
                        &mut window,
                        self.theme_mode.get(),
                        &mut self.cursor,
                    ),
                    kludgine,
                )
                .clear_focus();

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
                    }
                }
            }
            ElementState::Released => {
                let Some(device_buttons) = self.mouse_buttons.get_mut(&device_id) else {
                    return;
                };
                let Some(handler) = device_buttons.remove(&button) else {
                    return;
                };
                if device_buttons.is_empty() {
                    self.mouse_buttons.remove(&device_id);
                }
                let Some(handler) = self.tree.widget(handler) else {
                    return;
                };
                let cursor_location = self.cursor.location;
                let mut context = EventContext::new(
                    WidgetContext::new(
                        handler,
                        &self.current_theme,
                        &mut window,
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
            }
        }
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
            WindowCommand::RequestClose => {
                let mut window = RunningWindow::new(
                    window,
                    kludgine.id(),
                    &self.redraw_status,
                    &self.cushy,
                    &self.focused,
                    &self.occluded,
                    &self.inner_size,
                );
                if self.behavior.close_requested(&mut window) {
                    window.close();
                }
            }
            WindowCommand::SetTitle(new_title) => {
                window.set_title(&new_title);
            }
        }
    }
}

impl<Behavior> Drop for CushyWindow<Behavior> {
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

    use figures::units::UPx;
    use figures::Size;

    use crate::app::Cushy;
    use crate::context::sealed::InvalidationStatus;
    use crate::styles::{FontFamilyList, ThemePair};
    use crate::value::{Dynamic, Value};
    use crate::widget::OnceCallback;
    use crate::window::{ThemeMode, WindowAttributes};

    pub struct Context<C> {
        pub user: C,
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
        pub theme: Option<Value<ThemePair>>,
        pub theme_mode: Option<Value<ThemeMode>>,
        pub transparent: bool,
        pub serif_font_family: FontFamilyList,
        pub sans_serif_font_family: FontFamilyList,
        pub fantasy_font_family: FontFamilyList,
        pub monospace_font_family: FontFamilyList,
        pub cursive_font_family: FontFamilyList,
        pub font_data_to_load: Vec<Vec<u8>>,
        pub on_closed: Option<OnceCallback>,
    }

    #[derive(Clone)]
    pub enum WindowCommand {
        Redraw,
        RequestClose,
        SetTitle(String),
    }

    pub trait CaptureFormat {
        const HAS_ALPHA: bool;

        fn convert_rgba(data: &mut Vec<u8>, width: u32, bytes_per_row: u32);
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
#[derive(Clone)]
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

#[derive(Clone)]
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
                    pending
                        .commands
                        .lock()
                        .expect("lock poisoned")
                        .push(message);
                }
            }
            InnerWindowHandle::Known(handle) => {
                let _result = handle.send(message);
            }
            InnerWindowHandle::Virtual(state) => match message {
                WindowCommand::Redraw => state.redraw_target.set(RedrawTarget::Now),
                WindowCommand::RequestClose => state.close_requested.set(true),
                WindowCommand::SetTitle(title) => state.title.set(title),
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
    pub fn with_root(self, root: impl MakeWidget) -> Window<WidgetInstance> {
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

        for command in pending.commands.lock().expect("poisoned").drain(..) {
            let _result = handle.send(command);
        }

        WindowHandle::new(handle, self.0.redraw_status.clone())
    }
}

#[derive(Default)]
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

    /// Removes any stored value for this window.
    pub fn clear_for(&mut self, context: &WidgetContext<'_>) -> Option<T> {
        self.by_window.remove(&context.kludgine_id())
    }
}

impl<T> Default for WindowLocal<T> {
    fn default() -> Self {
        Self {
            by_window: AHashMap::default(),
        }
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
    /// The currently set cursor icon.
    pub cursor: CursorIcon,
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
            cursor: CursorIcon::default(),
            size: Size::new(UPx::new(800), UPx::new(600)),
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

    fn set_cursor_icon(&mut self, cursor: CursorIcon) {
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

/// A builder for a [`VirtualWindow`].
pub struct VirtualWindowBuilder {
    widget: WidgetInstance,
    multisample_count: u32,
    initial_size: Size<UPx>,
    scale: f32,
    transparent: bool,
}

impl VirtualWindowBuilder {
    /// Returns a new builder for a virtual window that contains `contents`.
    #[must_use]
    pub fn new(contents: impl MakeWidget) -> Self {
        Self {
            widget: contents.make_widget(),
            multisample_count: 4,
            initial_size: Size::new(UPx::new(800), UPx::new(600)),
            scale: 1.,
            transparent: false,
        }
    }

    /// Sets this virtual window's multi-sample count.
    ///
    /// By default, 4 samples are taken. When 1 sample is used, multisampling is
    /// fully disabled.
    #[must_use]
    pub fn multisample_count(mut self, count: u32) -> Self {
        self.multisample_count = count;
        self
    }

    /// Sets the size of the virtual window.
    #[must_use]
    pub fn size<Unit>(mut self, size: Size<Unit>) -> Self
    where
        Unit: Into<UPx>,
    {
        self.initial_size = size.map(Into::into);
        self
    }

    /// Sets the DPI scaling factor of the virtual window.
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

    /// Returns the initialized virtual window.
    #[must_use]
    pub fn finish(self, device: &wgpu::Device, queue: &wgpu::Queue) -> VirtualWindow {
        VirtualWindow::new(
            self.widget,
            self.multisample_count,
            self.initial_size,
            self.scale,
            self.transparent,
            device,
            queue,
        )
    }
}

/// A virtual Cushy window.
///
/// This type allows rendering Cushy applications directly into any wgpu
/// application.
pub struct VirtualWindow {
    window: CushyWindow<WidgetInstance>,
    kludgine: Kludgine,
    last_rendered_at: Option<Instant>,
    state: VirtualState,
}

impl VirtualWindow {
    /// Returns a new virtual window with the provided specifications.
    fn new(
        widget: WidgetInstance,
        multisample_count: u32,
        initial_size: Size<UPx>,
        scale: f32,
        transparent: bool,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let mut kludgine = Kludgine::new(
            device,
            queue,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::MultisampleState {
                count: multisample_count,
                ..Default::default()
            },
            initial_size,
            scale,
        );
        let mut state = VirtualState::new();
        let window = CushyWindow::<WidgetInstance>::new(
            widget.make_widget(),
            &mut state,
            &mut kludgine::Graphics::new(&mut kludgine, device, queue),
            sealed::WindowSettings {
                cushy: Cushy::new(),
                redraw_status: InvalidationStatus::default(),
                title: Value::default(),
                attributes: None,
                occluded: Dynamic::default(),
                focused: Dynamic::default(),
                inner_size: Dynamic::default(),
                theme: None,
                theme_mode: None,
                transparent,
                serif_font_family: FontFamilyList::default(),
                sans_serif_font_family: FontFamilyList::default(),
                fantasy_font_family: FontFamilyList::default(),
                monospace_font_family: FontFamilyList::default(),
                cursive_font_family: FontFamilyList::default(),
                font_data_to_load: Vec::default(),
                on_closed: None,
            },
        );

        Self {
            window,
            kludgine,
            last_rendered_at: None,
            state,
        }
    }

    /// Prepares all necessary resources and operations necessary to render the
    /// next frame.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let now = Instant::now();
        self.state.elapsed = self
            .last_rendered_at
            .map(|i| now.duration_since(i))
            .unwrap_or_default();
        self.last_rendered_at = Some(now);
        self.window.prepare(
            &mut self.state,
            &mut kludgine::Graphics::new(&mut self.kludgine, device, queue),
        );
    }

    /// Renders this window in a wgpu render pass created from `pass`.
    ///
    /// Returns the submission index of the last command submission, if any
    /// commands were submitted.
    pub fn render(
        &mut self,
        pass: &wgpu::RenderPassDescriptor<'_, '_>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Option<wgpu::SubmissionIndex> {
        let mut frame = self.kludgine.next_frame();
        let mut gfx = frame.render(pass, device, queue);
        self.window.contents.render(1., &mut gfx);
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

    /// Requests that the window close.
    ///
    /// Returns true if the request should be honored.
    pub fn request_close(&mut self) -> bool {
        if self
            .window
            .close_requested(&mut self.state, &mut self.kludgine)
        {
            self.state.closed = true;
            true
        } else {
            self.state.dynamic.close_requested.set(false);
            false
        }
    }

    /// Returns true if this window should no longer be open.
    #[must_use]
    pub fn closed(&self) -> bool {
        self.state.closed
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
}

/// A builder of a [`VirtualRecorder`].
pub struct VirtualRecorderBuilder<Format> {
    contents: WidgetInstance,
    size: Size<UPx>,
    scale: f32,
    format: PhantomData<Format>,
}

impl VirtualRecorderBuilder<Rgb8> {
    /// Returns a builder of a [`VirtualRecorder`] that renders `contents`.
    pub fn new(contents: impl MakeWidget) -> Self {
        Self {
            contents: contents.make_widget(),
            size: Size::new(UPx::new(800), UPx::new(600)),
            scale: 1.0,
            format: PhantomData,
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

    /// Returns an initialized [`VirtualRecorder`].
    pub fn finish(self) -> Result<VirtualRecorder<Format>, VirtualRecorderError> {
        VirtualRecorder::new(self.size, self.scale, self.contents)
    }
}

struct Capture {
    buffer: wgpu::Buffer,
    texture: Texture,
    multisample: Texture,
}

/// A recorder of a [`VirtualWindow`].
pub struct VirtualRecorder<Format = Rgb8> {
    window: VirtualWindow,
    device: wgpu::Device,
    queue: wgpu::Queue,
    capture: Option<Capture>,
    data: Vec<u8>,
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
        contents: impl MakeWidget,
    ) -> Result<Self, VirtualRecorderError> {
        let wgpu = wgpu::Instance::default();
        let adapter =
            pollster::block_on(wgpu.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .ok_or(VirtualRecorderError::NoAdapter)?;
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: Kludgine::REQURED_FEATURES,
                limits: Kludgine::adjust_limits(wgpu::Limits::downlevel_webgl2_defaults()),
            },
            None,
        ))?;

        let mut recorder = Self {
            window: VirtualWindow::new(
                contents.make_widget(),
                4,
                size,
                scale,
                Format::HAS_ALPHA,
                &device,
                &queue,
            ),
            device,
            queue,
            capture: None,
            data: Vec::new(),
            format: PhantomData,
        };

        recorder.refresh()?;

        Ok(recorder)
    }

    /// Returns the tightly-packed captured bytes.
    ///
    /// The layout of this data is determined by the `Format` generic.
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    /// Returns the current size of the recorder.
    pub const fn size(&self) -> Size<UPx> {
        self.window.kludgine.size()
    }

    fn recreate_buffers_if_needed(&mut self, size: Size<UPx>, bytes: u64) {
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
            self.capture = Some(Capture {
                buffer,
                texture,
                multisample,
            });
        }
    }

    /// Redraws the contents.
    pub fn refresh(&mut self) -> Result<(), wgpu::BufferAsyncError> {
        let render_size = self.window.kludgine.size().ceil();
        let bytes_per_row = copy_buffer_aligned_bytes_per_row(render_size.width.get() * 4);
        let size = u64::from(bytes_per_row) * u64::from(render_size.height.get());
        self.recreate_buffers_if_needed(render_size, size);

        let capture = self.capture.as_ref().assert("always initialized above");

        self.window.prepare(&self.device, &self.queue);

        self.window.render(
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
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        capture.texture.copy_to_buffer(
            wgpu::ImageCopyBuffer {
                buffer: &capture.buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: None,
                },
            },
            &mut encoder,
        );
        self.queue.submit([encoder.finish()]);

        let map_result = Arc::new(Mutex::new(None));
        let condvar = Arc::new(Condvar::new());
        let slice = capture.buffer.slice(0..size);

        std::thread::scope(|scope| {
            scope.spawn({
                let map_result = map_result.clone();
                let condvar = condvar.clone();
                move || {
                    condvar.notify_one();
                    slice.map_async(wgpu::MapMode::Read, {
                        move |result| {
                            *map_result.lock().assert("thread panicked") = Some(result);
                            condvar.notify_one();
                        }
                    });
                }
            });

            // Now that we've queued up the data mapping thread, let's make sure
            // our vec is allocated. Since an allocation can take a moment, this
            // is the perfect to do it.
            self.data.clear();
            self.data.reserve(size.cast());

            // Wait for the buffer to have been mapped.
            loop {
                self.device.poll(wgpu::Maintain::Poll);

                let mut result = map_result.lock().assert("thread panicked");
                if let Some(result) = result.take() {
                    result?;
                    break;
                }

                let _guard = condvar
                    .wait_timeout(result, Duration::from_millis(1))
                    .assert("thread panicked");
            }

            Ok(())
        })?;

        self.data.extend_from_slice(&slice.get_mapped_range());

        Format::convert_rgba(&mut self.data, render_size.width.get(), bytes_per_row);

        Ok(())
    }
}

fn copy_buffer_aligned_bytes_per_row(width: u32) -> u32 {
    (width + COPY_BYTES_PER_ROW_ALIGNMENT - 1) / COPY_BYTES_PER_ROW_ALIGNMENT
        * COPY_BYTES_PER_ROW_ALIGNMENT
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
