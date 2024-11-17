# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- markdownlint-disable no-duplicate-heading -->

## Unreleased

### Breaking Changes

- `PendingWindow`s opened into a `PendingApp` now have working `WindowHandle`s.
  As a result of this fix, `Open::open()` now returns a `WindowHandle` instead
  of an `Option<WindowHandle>`.
- `CushyWindow::set_occluded` and `CushyWindow::resize now require a
  `PlatformWindowImplementation` parameter.
- `PlatformWindowImplementation::position` has been renamed to
  `PlatformWindowImplementation::outer_position`.
- `Window::position` has been renamed to `Window::outer_position` and takes an
  additional parameter controlling whether to position the window with the
  initial value of the dynamic or whether to let the operating system perform
  the initial positioning.
- Keyboard modifiers have been added to two event structures: `ButtonClick` and
  `KeyboardEvent`. Because of this change, `KeyboardEvent` no longer implements
  `From<winit::event::KeyEvent>`. Instead, a new api `KeyEvent::from_winit`
  allows constructing from both the modifiers and key event.
- The type alias `WindowAttributes` has been removed. This type is no longer
  used in Cushy's public API.
- `Window::attributes` has been made private. All functionality available from
  this field should be available directly via functions on `Window`, and when
  possible, the attributes are be able to be dynamically updated as well.
- `Window::focused` now accepts an `IntoValue` implementor instead of
  `IntoDynamic`. Additionally, the initial value is now used to control whether
  the OS should initially activate the window when it is opened.
- `WrapperWidget::root_behavior`'s provided implementation now returns
  `Some(RootBehavior::PassThrough)` to reflect the general intention of what
  `WrapperWidget` implementors are generally doing. Previously, `None` was
  returned.
- `PlatformWindowImplementation::request_inner_size` and
  `PlatformWindow::request_inner_size` now both return `Option<Size<UPx>>`. When
  the function returns a value, Cushy is expected to apply the new size to the
  state immediately rather than waiting for the platform to notify it of a
  resize event.
- `PlatformWindow::outer_size` is a new function that returns the window's
  current outer size.
- `Graphics::draw_texture` and `Graphics::draw_textured_shape` now both accept
  an opactiy parameter controlling how opaque the texture should be rendered at.
- `MakeWigetList` has had some of its functions moved to a new trait:
  `IntoWidgetList`.  A blanket implementation of `MakeWidgetList` is provided
  for types that implement `IntoWidgetList`. In general, compilation errors
  related to this change can be fixed by importing `IntoWidgetList`.

  - `MakeWidgetList::into_rows` -> `IntoWidgetList::into_rows`
  - `MakeWidgetList::into_columns` -> `IntoWidgetList::into_columns`
  - `MakeWidgetList::into_layers` -> `IntoWidgetList::into_layers`
  - `MakeWidgetList::into_wrap` -> `IntoWidgetList::into_wrap`
  - `MakeWidgetList::into_list` -> `IntoWidgetList::into_list`
- `ConstraintLimit::fit_measured` and `FitMeasuredSize::fit_measured` now accept
  either a `Px` or `UPx` measurement, and does not perform scaling adjustments.
  To convert `Lp` use `into_upx()` first.
- `IntoReader::to_label` and `IntoReader::into_label` have been moved to their
  own trait: `Displayable`. This allows more flexible acceptance of types.
- `MakeWidget::widget_ref` has been renamed to `MakeWidget::into_ref`.
- `Radio` and `Checkbox` are now powered by a new widget `Indicator<T>`. This
  new implementation treats the indicators as independently focusable widgets
  rather than how the `Button`-powered implementation shows focus around the
  entire "label" of the button.

  The `Button`-powered implementation can still be used by using the `kind`
  function to pick the `ButtonKind` to use. Prior to this change,
  `ButtonKind::Transparent` was the default.

  Lastly, several APIs no longer accept a `label` parameter. Instead, the
  widgets have new functions `labelled_by(label)` that can be used to attach a
  clickable label to an indicator. The affected APIs are:

  - `Radio::new`
  - `Checkbox::new`
  - `Checkable::into_checkbox`
  - `Checkable::to_checkbox`
  - `Dynamic::new_radio`
  - `Dynamic::new_checkbox`
- `Space` no longer implements hit_test. If you need an area to intercept mouse
  events, wrap the `Space` in a `Custom` widget:

  ```rust
    Custom::new(Space::colored(Color::RED)).on_hit_test(|_, _| true)
  ```
- `Component` is now `#[non_exhaustive]`.
- `Wrap::vertical_align` has been removed in favor of the `VerticalAlignment`
  component.
- `widgets::wrap::VerticalAlign` has been moved to `styles::VerticalAlign`.
  Additionally `VerticalAlign::Middle` has been renamed `VerticalAlign::Center`
  for consistency with horizontal alignment.
- `Label` now honor `HorizontalAlignment` and `VerticalAlignment`. Previously
  `Label`s centered their content when sized larger than the text they
  contained. The defaults for the alignment components are left and top,
  respectively.

### Changed

- `Open` is now implemented for most types via a blanket implementation for a
  new trait, `MakeWindow`. `MakeWindow` splits the process of creating a
  `Window<Behavior>` from the process of opening a window.

  The new `MakeWindow` trait adds some new functionality:
  `open_centered`/`run_centered`/`run_centered_in`. These functions present a
  window centered on the screen where the window initially is shown.
- `IntoAnimate::on_complete` now takes an `FnOnce` instead of an `FnMut`. Since
  this widens the types of functions this accepts, this is not a breaking
  change. This matches the documented behavior.
- `tracing_subscriber` initialization has been moved to `PendingApp::default()`
  from `Window::run`. This ensures logging is always initialized for all Cushy
  apps.
- The default logging output has been trimmed to only show errors for wgpu,
  winit, and naga. Thanks to @bluenote10 for the feedback!
- `WrapperWidget::activate`'s default implementation now activates the wrapped
  widget.
- `Space` now intercepts mouse events if its color has a non-zero alpha channel.
- If the root widget of a window is a `Resize` with an exact width and height,
  the window will have its resizable attribute disabled. This will not update
  the resizable `Dynamic<bool>` on `Window`.
- Transparent buttons' focus rings are now drawn using the same corner radius as
  the button and have padding between the label and the focus ring.

### Fixed

- `Collapse`, `OverlayLayer`, and `Progress` all honor the theme components
  `EasingIn` and `EasingOut` rather than hard-coded easing functions.
- `Collapse` widgets now apply child size changes immediately rather than
  animating the transition. The previous behavior caused nested collapsed
  widgets to grow and shrink in an accordian-like fashion rather than animating
  together.
- `Switcher` now unmounts child widgets in all windows it is mounted in. Fixes
  [#139][139].
- `inner_size` and `outer_size` are now initialized after the first layout is
  performed. This ensures that when `resize_to_fit` is used, the first observed
  values will be the resized values.
- `Resize` now performs a second layout pass, if necessary, to ensure that
  children widgets have an opportunity to fill the resized area. Additionally,
  the first SizeToFit measurement will be performed with the minimum dimension.
- When a keyboard-activated widget activates another widget during its callback,
  the key-up event now sends the deactivate event to the finally-activated
  widget.
- A rare deadlock occurring when multiple threads were racing to execute
  `Dynamic<T>` change callbacks has been fixed.
- `Stack` no longer unwraps a `Resize` child if the resize widget is resizing in
  the direction opposite of the Stack's orientation.
- If the layout of widgets changes during a redraw, the currently hovered widget
  is now properly updated immediately. Previously, the hover would only update
  on the next cursor event.
- `Scroll` no longer attempts to preserve scroll amounts using a percentage when
  its child changes size.
- `Stack` and `Grid` now properly recompute exact-sized `Lp` children when the
  display scale is changed.
- `Button`'s colors are now fully reactive. The caching code to prevent color
  duplicate change animations has been simplified to fix this.
- Clipping math of widgets with negative origins has been fixed to no longer
  allow overdrawing the widget's bounds. This was noticable in the
  nested-scroll.rs example when reducing the height of the window below 6
  inches.
- `Scroll` now uses the new `ScrollBar` widget for its bars rather than manually
  drawing them. By making this change, the bars now have input priority over the
  contents. This means that the scroll bars are now clickable even in areas
  where interactive widgets are beneath them.
- `GraphicsContext::fill` now properly fills the entire region of the widget.
- `Slider` now correctly calculates its width when in a fully `SizeToFit`
  layout.
- `ThemedMode` is now properly applied consistently. Previously sometimes the
  window's theme mode would be used instead of the overridden mode.
- `Expand` now requests children size to fit in the non-expanding direction when
  using either vertical or horizontal expanding modes.
- `Input` now draws its selection when not focused.
- `Input` no longer immediately selects all of its contents. Instead, when focus
  is shifted to the input, if there is no selection the entire field will be
  selected. This is not always the desired behavior, and further customization
  of this behavior should be allowed, but this is a step in the right direction
  compared to the previous behavior.

### Added

- `ComponentProbe<T>` is a new widget that allows reading a
  `ComponentDefinition` value from the theme at runtime through a
  `Dynamic<T::ComponentType>`. For example, a `ComponentProbe<TextSize>` will
  provide access to a `Dynamic<Dimension>`. Previously this required creating a
  custom widget to access the runtime theme information.

  `ContextFreeComponent::probe()` and `ContextFreeComponent::probe_wrapping()`
  provide an easy interface for creating probes from components.
- These `to_*` variations of existing `into_*` functions have been added to
  avoid some cases where cloning might be needed.

  - `MakeWidget::to_button()`
  - `MakeWidget::to_checkbox()`
  - `WidgetInstance::to_window()`
- `Watcher` is a new type that executes callbacks when one or more sources
  changes. This can simplify some data flows such as regenerating a value when
  many one of many UI elements are changed.
- The `resize_to_fit` setting has been added to all window types and is no
  longer limited to `VirtualRecorder`s. This setting can be changed using:

  - `Window::resize_to_fit`
  - `StandaloneWindowBuilder::resize_to_fit`
- `Tracked<Source>` is a new type that can store a `Dynamic<T>` or `Value<T>`
  and provide easy ways to track whether the value has been updated.
- Many new functions have been added to `Window` to expose more functionality
  supported by winit:

  - `Window::app_name`
  - `Window::content_protected`
  - `Window::cursor_hittest`
  - `Window::cursor_position`
  - `Window::cursor_visible`
  - `Window::decorated`
  - `Window::enabled_buttons`
  - `Window::fullscreen`
  - `Window::icon`
  - `Window::inner_position`
  - `Window::maximized`
  - `Window::minimized`
  - `Window::modifiers`
  - `Window::outer_position`
  - `Window::outer_size`
  - `Window::resize_increments`
  - `Window::resized`
  - `Window::transparent`
  - `Window::visible`
  - `Window::window_level`
- `run(&mut App)` is a new function that can provide a more concise way of
  executing applications that would normally require using `PendingApp`.
- `PendingApp::on_startup` allows executing a function once the application's
  event loop has begun.
- `App::monitors()` returns a snapshot of the currently configured monitors
  attached to the device. A new example demonstrating this API is available at
  `examples/monitors.rs`.
- `PlatformWindowImplementation` has several new functions with provided
  implementations for winit users:

  - `inner_position`
  - `outer_size`
- `#[cushy::main]` is a new attribute proc-macro that simplifies initializing
  and running multi-window applications.
- `Window::on_open` executes a callback when the window is initially opened.
- `Shortcuts` is a new widget that simplifies attaching logic to keyboard
  shortcuts. Any widget can be wrapped with keyboard shortcut handling by using
  `MakeWidget::with_shortcut`/`MakeWidget::with_repeating_shortcut`.
- `Window` now can have its own `ShortcutMap`, which can be populated using
  `Window::with_shortcut`/`Window::with_repeating_shortcut`, or provided using
  `Window::with_shortcuts`.
- `ModifiersStateExt` is a new trait that adds functionality to winit's
  `ModifiersState` type. Specifically, this trait adds an associated `PRIMARY`
  constant that resolves to the primary shortcut modifier on the target
  platform.
- New feature flag `serde` enables serialization support for some types.
- `WindowHandle::execute` executes a function on the window's thread providing
  access to an `EventContext`. This can be used to gain access to the window
  directly, including getting a reference to the underlying winit Window.
- `Modal` is a new layer widget that presents a single widget as a modal
  session.
- `App::prevent_shutdown()` returns a guard that prevents the application from
  closing automatically when the final window is closed.
- `WindowBehavior::initialized` is invoked once the window has been fully
  initialized.
- `Window::on_init` is a new callback that is invoked before the winit window is
  initialized.
- `Window::on_file_drop` is a new callback that is invoked when file drop events
  occur for the window.
- `Image::opacity` allows rendering the image with a given opacity.
- `PendingApp::with_tracing` and `PendingApp::initialize_tracing` install
  Cushy's tracing Subscriber. The default `PendingApp` has tracing initialized,
  but `PendingApp::new` does not.
- `Animate` and `IntoAnimate` are now implemented for:

  - `impl FnMut(Duration) -> ControlFlow<Duration> + Send + Sync + 'static`
  - `SharedCallback<Duration, ControlFlow<Duration>>`
  - `SharedCallback`
- `Cushy::multi_click_threshold`/`Cushy::set_multi_click_threshold` provide
  access to the setting used by Cushy widgets to detect whether two clicks are
  related.
- `ClickCounter` is a new helper that simplifies handling actions based on how
  many sequential clicks were observed.
- `Dimension::is_unbounded` is a new helper that returns true if neither the
  start or end is bounded.
- `&String` and `Cow<'_, str>` now implement `MakeWidget`.
- Cargo feature `native-dialogs` has been added to enable native dialogs to be
  shown by Cushy applications.
- `PendingApp::with_on_unrecoverable_error` allows overriding the default
  behavior when an unrecoverable error occurs. Previously, all unrecoverable
  errors resulted in a panic. The new default behavior uses the `native-dialogs`
  feature when enabled to display the error using a system-native message
  dialog.
- `MessageBox` displays a prompt to the user in a `Modal` layer, above a
  `WindowHandle`, or in an `App`. When shown above a window or app, the `rfd`
  crate is used to use the native system dialogs.
- `FilePicker` displays a file picker to the user in a `Modal` layer, above a
  `WindowHandle`, or in an `App`. When shown above a window or app, the `rfd`
  crate is used to use the native system dialogs.

  The `FilePicker` type supports these modes of operation:

  - Saving a file
  - Choosing a single file
  - Choosing one or more files
  - Choosing a single folder/directory
  - Choosing one or more folders/directories
- `DynamicGuard::unlocked` executes a closure while the guard is temporarily
  unlocked.
- `Scroll::preserve_max_scroll` controls whether the scroll view automatically
  scrolls when currently scrolled to the maximum and its child grows. The
  default is `true`, which was the behavior before this flag was added.
- `Image` now supports `ImageCornerRadius`. Thanks to @danbulant for helping
  with this change!
- `Scroll` now exposes its scroll amount, maximum scroll, and more information
  that allows completely customizing a scroll view's behavior. Thanks to
  @danbulant for helping with this change!
- `ScrollBar` is a new widget that renders a scroll bar meant to scroll through
  a large container. Additionally its appearance has been updated to be based on
  the theme. Several new style components have been added to control how scroll
  bars are rendered: `ScrollBarThumbColor`, `ScrollBarThumbOutlineColor`,
  `ScrollBarThumbOutlineThickness`, `ScrollBarThumbCornerRadius`.
- `Label::overflow` allows customizing the behavior for a label when it cannot
  be drawn on a single line.
- `ConstraintLimit::fill_or_fit` is a new function that will fill the available
  space when being requested to fill, otherwise it return the minimum of the
  measured size and the constraint limit.
- `OutlineWidth` is a new component that is used to control the width of most
  outlines drawn in the user interface.
- `FocusColor` is a new component that controls the color of the keyboard focus
  indicator.
- `Graphics::draw` and `Graphics::draw_with` are a new function that allows
  performing arbitrary `wgpu` drawing operations when rendering. See the
  `shaders.rs` example for an example on how to use this to render into a Canvas
  with a custom shader.
- `Pile` is a new widget that shows one of many widgets. `PiledWidget` handles
  are returned for each widget pushed into a pile. These handles can be used to
  show or close a specific widget in a pile.
- `HorizontalAlignment` and `VerticalAlignment` are new components that are used
  by some widgets when positioning their contents. `WrapperWidget::position_child` has been updated to use `WrappedLayout::aligned` to support these components on most widgets automatically. `Label` and `Resize` have also been updated to support these components.
- Local style support has been fully exposed. Local styles are applied to the widget they are attached to, but are not inherited to child widgets. The new APIs are:

  - `Styles::insert_local`
  - `Styles::insert_local_dynamic`
  - `Styles::insert_local_named`
  - `Styles::with_local`
  - `Styles::with_local_dynamic`
  - `Styles::with_local_named`
  - `Style::with_local`
  - `Style::with_local_dynamic`
  - `Style::with_local_named`
  - `MakeWidget::with_local`
  - `MakeWidget::with_local_dynamic`


[139]: https://github.com/khonsulabs/cushy/issues/139

## v0.4.0 (2024-08-20)

### Breaking Changes

- Dependency `kludgine` has been updated to `v0.10.0`, which updates Cushy to
  `wgpu v22.0.0` and `cosmic-text v0.12.0`.
- At some point, a dependency of the `image` crate has been updated with a
  minimum supported Rust version (MSRV) of `1.80.0`. This is Cushy's new MSRV to
  reflect this requirement.
- `Source::for_each_*` now invoke the callback with the current contents of of
  the source before attaching the callback. New functions beginning with
  `for_each_subsequent_` have been added with the original behavior.
- `CushyWindowBuilder` has been renamed to `StandaloneWindowBuilder` and
  `MakeWidget::build_virtual_window` has been renamed to
  `build_standalone_window`.
- All animation easing related functionality has been reactored into a separate
  crate: `easing-function`. Most code will remain unaffected due to re-exports,
  but the `Easing` trait no longer accepts a `ZeroToOne` parameter, instead
  accepting an `f32` value.

### Fixed

- Fixed a panic that could occur when removing certain nested hierarchies from a
  window.
- `CallbackHandle` now has a `must_use` hint that might help users discover the
  persist function.
- Fixed a deadlock that could occur when multiple threads were attempting to
  execute change callbacks for the same dynamic at the same time.
- The initial `inner_size` of a `Window` is now used if it is non-zero and
  `WindowAttributes::inner_size` is `None`.
- Container's layout and drawing functions now properly round up/down the
  measurements to ensure accurate rendering. Fixes [#158][158].
- `Input` selection handling when dragging below or above the text field is now
  handled correctly.
- Nested hierarchies of widgets stored in a reused `WidgetInstance` are now
  properly unmounted and remounted. For widgets that store `MountedWidget`s, in
  their `mounted` events the widgets should remount their children if needed.

  This fix not only fixes underlying issues with how unmounting was occuring,
  but also fixes `Stack`, `Grid`, and `WidgetRef` to automatically remount as
  needed.


[158]: https://github.com/khonsulabs/cushy/issues/158

### Added

- `AnimationRecorder::animate_keypress` is a new helper that animates a single
  key press.
- `AnimationRecorder::animate_mouse_button` is a new helper that animates a
  single mouse button press and release.
- `Window::on_close_requested` is a new function that allows providing a
  callback that is invoked before the window is closed when the user or
  operating system requests that a window is closed. If the callback returns
  true, the window is allowed to be closed. If false is returned, the window
  will remain open. This feature is most commonly used to prevent losing unsaved
  changes.
- `Fraction` now has `LinearInterpolation` and `PercentBetween` implementations.
- `Window::zoom` allows setting a `Dynamic<Fraction>` that scales all DPI-scaled
  operations by an additional scaling factor.
- `Edges` and `ContainerShadow` now implement `figures::Round`.

## v0.3.0 (2024-05-12)

### Breaking Changes

- This crate's MSRV is now `1.74.1`, required by updating `wgpu`.
- `wgpu` has been updated to `0.20`.
- `winit` has been updated to `0.30`.
- All context types no longer accept a `'window` lifetime. For most end-user
  code, it means removing one elided lifetime from these types:
  - `WidgetContext`
  - `EventContext`
  - `LayoutContext`
  - `GraphicsContext`
- `WidgetContext`'s `Deref` target is now `&mut dyn PlatformWindow`. This change
  ensures all widgets utilize a shared interface between any host architecture.
- All `DeviceId` parameters have been changed to a `DeviceId` type provided by
  Cushy. This allows for creating arbitrary input device IDs when creating an
  integration with other frameworks or driving simulated input in a
  `VirtualWindow`.
- `WidgetRef` is now a `struct` instead of an enum. This refactor changes the
  mounted state to be stored in a `WindowLocal`, ensuring `WidgetRef`s work
  properly when used in a `WidgetInstance` shared between multiple windows.
- `WidgetRef::unmount_in` should be called when the widget is being unmounted to
  clean up individual window state.
- `Dynamic<T>` and `DynamicReader<T>` have had most of their functions moved
  into the traits `Source<T>` and `Destination<T>`. This unifies the APIs
  between the two types, and offers a path for other specialized reactive data
  types to all share a unified API.
- `map_mut` now takes a `Mutable<'_, T>` parameter instead of an `&mut T`
  parameter. This type tracks whether the reference is accessed using
  `DerefMut`, allowing `map_mut` to skip invoking change callbacks if only
  `Deref` is used.
- `redraw_when_changed()`/`invalidate_when_changed()` from some types have been
  moved to the `Trackable` trait. This was to ensure all trackable types provide
  the same API.
- `Label` has been refactored to accept any `Display` type. As a result of this,
  `Label::text` is now named `display` and `Label::new()` now accepts an
  `IntoReadOnly<T>` instead of `IntoValue<String>`.
- `Dynamic<WidgetList>::wrap` and `WidgetList::wrap` have been renamed to
  `into_wrap` for consistency.
- Cushy now has its own `KeyEvent` type, as winit's has private fields. This
  prevented simulating input in a `VirtualWindow`.
- `FlexibleDimension::ZERO` has been removed, and now `FlexibleDimension`
  implements `Zero` which defines an associated constant of the same name and
  purpose.
- `Children` has been renamed to `WidgetList`.
- `ColorExt::into_source_and_lightness` has been renamed to
  `ColorExt::into_hsl`, and its return type is now `Hsl` instead of the
  individual components.
- `Window::font_data_to_load` has been renamed to `fonts`, and now has the
  `FontCollection` type.
- Several font-related functions have been moved from `GraphicsContext` to
  `WidgetContext`:

  - `GraphicsContext::set_font_family()`
  - `GraphicsContext::find_available_font_family()`
  - `GraphicsContext::set_available_font_family()`
- `Open::open` now require exclusive references to the application.
- `PlatformWindowImplementation::set_cursor_icon` and
  `PlatformWindow::set_cursor_icon` have been renamed to `set_cursor` and accept
  `winit` 0.30's new `Cursor` type.
- `Button::on_click` now takes a `Option<ButtonClick>` structure. When this
  value is provided, information about the mouse click that caused the event is
  provided.
- `OverlayBuilder` has hade many of its functions moved into a new trait,
  `Overlayable`. This is to ensure common API surfaces across all overlayable
  widgets including the new `Menu` widget.

### Fixed

- The root widget is now included in the search for widgets to accept focus.
- Widgets that have been laid out with a 0px width or height no longer have
  their `redraw` functions called nor can they receive focus.
- `Grid` now synchronizes removal of widgets from `GridWidgets` correctly.
- `WidgetInstance`s can now be shared between windows. Any unpredictable
  behaviors when doing this should be reported, as some widgets may still have
  state that should be moved into a `WindowLocal` type.
- `Grid` no longer passes `ConstraintLimit::Fill` along to children when it
  contains more than one element. Previously, if rows contained widgets that
  filled the given space, this would cause the grid to calculate layouts
  incorrectly.
- A potential edge case where a `DynamicReader` would not return after being
  disconnected has been removed.
- [#120][120]: Dividing a `ZeroToOne` now properly checks for `NaN` and `0.`.
- Removed a possible deadlock when using `DynamicReader::block_until_updated`.
- Removed an edge case ensuring `Waker`s are signaled for `DynamicReader`s that
  are waiting for value when the last `Dynamic` is dropped.
- `Progress` now utilizes `IntoSource<Progress>` instead of
  `IntoDynamic<Progress>`. In general, this should not cause any code breakages
  unless the traits were being used in generics.
- `Space` now honors `ConstraintLimit::Fill` in its layout.
- When handling the Ctrl/Cmd+W shortcut to close windows, repeat keys are now
  ignored.
- `Color::constrast_between` was incorrectly allowing hue shifts to weigh in on
  the contrast when the color was desaturated, and the attempt to account for
  that was incorrectly being applied to the lightness contrast calculation. In
  short, this function should be much more accurate in perceived contrast
  evaluation.
- `Graphics::set_font_family` now clears the cached font family list, ensuring
  that the next call to apply_current_font_settings works correctly.
- `Image` now returns the correct size from `layout()` when in aspect scaling
  modes. Previously, it reported back the minimum size, since it's scale was
  considered flexible. This new behavior ensures that it always requests a size
  that is scaled with the aspect ratio.

  The rendering behavior remains unchanged, and the image will scale correctly
  within whatever bounds it is given.
- `Widget::unmounted` is now invoked for all widgets in the hierarchy.
  Previously, only the parent widget was having its unmounted event invoked.
- Resizing windows should no longer be out of sync with the resize operation.
  Previously, the window background would sometimes paint in newly revealed
  areas before the UI was redrawn.

### Changed

- `WidgetCacheKey` now includes the `KludgineId` of the context it was created
  from. This ensures if a `WidgetInstance` moves or is shared between windows,
  the cache is invalidated.
- All `Dynamic` mapping functions now utilize weak references, and the
  `CallbackHandle` now contains a strong reference to the originating dynamic.
  This should have no visible impact on end-user code.
- `ForEach`/`MapEach`'s implementations for tuples are now defined using
  `Source<T>` and `DynamicRead<T>`. This allows combinations of `Dynamic<T>`s
  and `DynamicReader<T>`s to be used in for_each/map_each expressions.

### Added

- Cushy now supports being embedded in any wgpu application. Here are the API
  highlights:

  - `CushyWindow` is a type that contains the state of a standalone window. It
    defines an API designed to enable full control with winit integration into
    any wgpu application. This type's design is inspired by wpgu's
    "Encapsulating Graphics Work" article. Each of its functions require being
    passed a type that implements `PlatformWindowImplementation`, which exposes
    all APIs Cushy needs to be fully functional.
  - `VirtualWindow` is a type that makes it easy to render a Cushy interface in
    any wgpu application where no winit integration is desired. It utilizes
    `VirtualState` as its `PlatformWindowImplementation`. This type also exposes
    a design inspired by wpgu's "Encapsulating Graphics Work" article.
  - `WindowDynamicState` is a set of dynamics that can be updated through
    external threads and tasks.
  - is a new trait that allows
  customizing the behavior that Cushy widgets need to be rendered.
- Cushy now supports easily rendering a virtual window: `VirtualRecorder`. This
  type utilizes a `VirtualWindow` and provides easy access to captured images.
  This type has the ability to capture animated PNGs as well as still images.
- `figures` is now directly re-exported at this crate's root. Kludgine still
  also provides this export, so existing references through kludgine will
  continue to work. This was added as an attempt to fix links on docs.rs (see
  rust-lang/docs.rs#1588).
- `Disclose` is a new widget that shows a disclosure triangle and uses a
  `Collapse` widget to show/hide the content when the disclosure button is
  clicked. This widget also supports an optional label that is shown above the
  content and is also clickable.
- [#99][99]: When an unhandled spacebar event is received by the window, the
  focused widget will be activated and deactived by the events. This previously
  was a `Button`-specific behavior that has been refactored into an automatic
  behavior for all widgets.
- `GridWidgets` now implements `FromIterator` for types that implement
  `Into<GridSection<N>>`.
- `Window::titled` allows setting a window's title, and can be provided a
  string-type or a `Dynamic<String>` to allow updating the title while the
  window is open.
- `DynamicReader::on_disconnect` allows attaching a callback that is invoked
  once the final source `Dynamic` is dropped.
- `Dynamic::instances()` returns the number of clones the dynamic has in
  existence.
- `Dynamic::readers()` returns the number of `DynamicReader`s for the dynamic in
  existence.
- `RunningWindow::kludgine_id()` returns a unique id for that window.
- `WindowLocal<T>` is a `HashMap`-based type that stores data on a per-window
  basis using `RunningWindow::kludgine_id()` as the key.
- `Source<T>` and `Destination<T>` are new traits that contain the reactive data
  model's API interface. `Dynamic<T>` implements both traits, and
  `DynamicReader<T>` implements only `Source<T>`.
- `DynamicRead<T>` is a new trait that provides read-only access to a dynamic's
  contents.
- `IntoReadOnly<T>` is a new trait that types can implement to convert into a
  `ReadOnly<T>`.
- `IntoReader<T>` is a new trait that types can implement to convert into a
  `DynamicReader<T>`.
- `ReadOnly<T>` is a type similar to `Value<T>` but instead of possibly being a
  `Dynamic<T>`, `ReadOnly::Reader` contains a `DynamicReader<T>`. This type can
  be used where widgets that receive a value but never mutate it.
- `Owned<T>` is a new type that can be used where no shared ownership is
  necessary. This type uses a `RefCell` internally instead of an `Arc` +
  `Mutex`. `Owned<T>` implements `Source<T>` and `Destination<T>`.
- `GenerationalValue<T>` now implements `Default` when `T` does.
- `Value<T>` now implements `From<Dynamic<T>>`.
- Most `into_` functions that create widgets now have `to_` variations that
  clone `self` before calling the `into_` function. This has only been done in
  situations where it is known or likely that the clone being performed is
  cheap.
- `CallbackHandle` now has `weak()` and `forget_owners()`. These functions allow
  a `CallbackHandle` to release its strong references to the `Dynamic` that the
  callback is installed on. This enables forming weak callback graphs that clean
  up independent of one another.
- `Source<T>::weak_clone` returns a `Dynamic<T>` with a clone of each value
  stored in the original source. The returned dynamic holds no strong references
  to the original source.
- `Point`, `Size`, and `Rect` now implement `LinearInterpolate`.
- `MakeWidget::build_virtual_window()` returns a builder for a `VirtualWindow`.
- `MakeWidget::build_recorder()` returns a builder for a `VirtualRecorder`.
- `Space::dynamic()` returns a space that dynamically colors itself using
  component provided. This allows the spacer to use values from the theme at
  runtime.
- `Space::primary()` returns a space that contains the primary color.
- `Hsl` is a new color type that is composed of hue, saturation, and lightness.
- `Hsla` is a new color type that combines `Hsl` with an alpha component.
- Additional color pickers are now available:

  - `HslPicker` picks `Hsl`
  - `HslaPicker` picks `Hsla`
  - `RgbPicker` picks `Color` with 255/1.0 alpha channel
  - `RgbaPicker` picks `Color`
- `ComponentPicker` is a picker of various `ColorComponent` implementors. It has
  constructors for each
- `InvalidationBatch` is a type that can batch invalidation requests being made
  by a background task. This can be useful if the background task is updating a
  variety of `Dynamic<T>`s, but wish to limit redrawing the interface until the
  task has completed its updates.

  This type does not prevent redraws from being performed due to the operating
  system or other threads requeseting them.
- A new feature `plotters` enables integration with the excellent
  [plotters][plotters] crate. `Graphics::as_plot_area()` is a new function that
  returns a `plotters::DrawingArea` that can be used to draw any plot that the
  `plotters` crate supports.
- `Delimiter` is a new widget that is similar to html's `hr` tag.
- `List` is a new widget that creates lists similar to HTML's `ol` and `ul`
  tags.
- `Dynamic::try_lock()` is a panic-free version of `Dynamic::lock()`.
- `FontCollection` is a new type that can be used to load fonts at app/window
  startup or at runtime.
- `Cushy::fonts()`returns a `FontCollection` that is loaded into all windows.
- `WidgetContext::loaded_font_faces()` returns a list of fonts that were loaded
  for a given `LoadedFont`.
- `Graphics::font_system()` returns a reference to the underlying Cosmic Text
  `FontSystem`.
- `Window::vsync` is a new setting that allows disabling VSync for that window.
- `ModifiersExt` is an extension trait for winit's `Modifiers` and
  `ModifiersState` types. This trait adds helpers for dealing with
  platform-specific meanings for keyboard modifiers.
- `OverlayLayer::dismiss_all()` dismisses all overlays immediately.
- `Menu` is a new widget type that can be shown in an `OverlayLayer` to create
  contextual menus or other popup menus.
- `PendingApp::new` is a new function that accepts an `AppRuntime` implementor.
  This abstraction is how Cushy provides the optional integration for `tokio`.
- Features `tokio` and `tokio-multi-thread` enable the tokio integration for
  this crate and expose a new type `TokioRuntime`. The `DefaultRuntime`
  automatically will use the `TokioRuntime` if either feature is enabled.

  When the `tokio` integration is enabled, `tokio::spawn` is able to be invoked
  from all Cushy code safely.

[plotters]: https://github.com/plotters-rs/plotters

[99]: https://github.com/khonsulabs/cushy/issues/99
[120]: https://github.com/khonsulabs/cushy/issues/120

## v0.2.0

### Breaking Changes

- This crate has been renamed from `Gooey` to `Cushy`. Other than the name of
  the library changing, the only type to change name is `Gooey` -> `Cushy`. This
  changelog has had all references and links updated.
- Many bounds required `UnwindSafe` due to a misunderstanding on how to handle
  this trait in `appit`. All requirements for `UnwindSafe` have been removed.
- `Cushy` no longer implements default. To gain access to a `Cushy` instance,
  create a `PendingApp` or get a reference to the running `App`.
- `Window::new` no longer accepts a `Cushy` parameter. The window now adopts the
  `Cushy` from the application it is opened within.
- `MakeWidget::into_window()` no longer takes any parameters.
- `Label<T>` is now generic over a new trait: `DynamicDisplay`. This new trait
  allows a way to query a `WidgetContext` to resolve the value to display. The
  trait is automatically implemented for all types that implement `Display`, so
  this change in practice shouldn't break much code.

### Changed

- [#92][92]: When a Window is resizable and the root widget's `layout()`
  function returns a size larger than the window's inner size, the window will
  no longer be resized to fit. The content will be forced to render in the given
  space, which may result in clipping.

  Using a `Resize` widget in the root hierarchy allows setting minimum width and
  heights for the content.

### Fixed

- A memory leak has been fixed that prevented the underlying widget tree of each
  window from being dropped. This was caused by a reference counting cycle, and
  has been fixed by switching `MountedWidget` to use a weak reference internally
  and having the window hold the strong reference to the tree.
- [#112][112]: Click-selection is handled correctly across graphemes now.
  Previously, code that was handling selecting between "ff" where cosmic_text
  had merged the two ASCII characters into a single glpyh was not honoring
  graphemes, allowing dragging selections inbetween multi-character glyphs.
- [#113][113]: `Input` now constraints its internal selection to the value's
  length automatically. This fixes an issue where the backspace key no longer
  would work after clearing the text field by setting the `Dynamic`.
- Validation callbacks are now associated with the `Dynamic<Validation>` being
  created rather than being persisted indefinitely on the source dynamic.

### Added

- `Validations::validate_result` attaches a `Dynamic<Result<T,E>>` to the
  validations. This was already available on `when` conditioned validations.
- `Dynamic::[try_]compare_swap` allows swapping the contents of a dynamic after
  verifying the current contents.
- [#91][91]: Multi-window support has been implemented. `PendingApp` allows
  opening one or more windows before starting the program. `App` is a handle to
  the running application that can be used to open additional windows at
  runtime.

  `Open` is a new trait that allows various types to open as a window given a
  reference to an application. This trait is implemented for all types that
  implemented `Run`, which means any type that was previously able to be run as
  a standalone executable can now be opened as a window within a multi-window
  application.

  The `multi-window` example demonstates using this feature to open multiple
  windows before starting Cushy as well as dynamically opening windows at
  runtime.
- `Window::on_close` sets a callback to be invoked when the window has closed.
- `WindowHandle` is a handle to a Cushy window. It enables requesting that the
  window closes, refreshing the window, or invalidating a widget contained in
  the window.
- `RunningWindow::handle()` returns a `WindowHandle` for the current window.
- `RunningWindow::request_close()` requests that the window should close. This
  ensures `WindowBehavior::close_requested` is invoked before the window is
  closed.
- `PendingWindow` is a new type that can return a `WindowHandle` for a window
  that hasn't opened yet. This can be used to allow a widget on a window to
  close the window.
- Style components for customizing default widget colors have been added:
  - `DefaultForegroundColor`
  - `DefaultBackgroundColor`
  - `DefaultHoveredForegroundColor`
  - `DefaultHoveredBackgroundColor`
  - `DefaultActiveForegroundColor`
  - `DefaultActiveBackgroundColor`
  - `DefaultDisabledForegroundColor`
  - `DefaultDisabledBackgroundColor`
- `CallbackHandle` can now be added with other `CallbackHandle`s to merge
  multiple handles into a single handle.
- `Dynamic::set_source` allows attaching a `CallbackHandle` to a `Dynamic`,
  ensuring the callback stays alive as long as the dynamic has an instance
  alive.

[91]: https://github.com/khonsulabs/cushy/issues/91
[92]: https://github.com/khonsulabs/cushy/issues/92
[112]: https://github.com/khonsulabs/cushy/issues/112
[113]: https://github.com/khonsulabs/cushy/issues/113

## v0.1.3 (2023-12-19)

### Added

- [#94][94]`Window::inner_size` allows setting a dynamic that will be
  synchronized with the window's inner size. When the dynamic is set to a new
  value, a resize request will be sent to the operating system. When the
  window's size is changed by the operating system, this dynamic will be updated
  with the new value.

  This dynamic is also accessible through `RunningWindow::inner_size`, which is
  accessible through contexts passed into various `Widget` functions.
- `Progress` now implements `Default` by returning `Progress::Indeterminant`.
- `WeakDynamic<T>` now implements `Debug` when `T` is `Debug`.

### Fixed

- [#97][97]: `Dynamic` callback invocations could be missed for a value when
  multiple threads were updating values at the same time. Now it is
  guaranteed that each callback will observe the latest value at least once.

  Cycles on the same thread are still detected and logged to prevent infinite
  loops from callback chain cycles.
- An integer underflow has been fixed in the Grid/Stack widgets.
- Padding is now rounded to the nearest whole pixel when applied across widgets.

[94]: https://github.com/khonsulabs/cushy/pull/94
[97]: https://github.com/khonsulabs/cushy/issues/97

## v0.1.2 (2023-12-18)

### Fixed

- Cushy now compiles for Windows. An indirect dependency, `appit`, also needs to
  be updated to v0.1.1. Running `cargo update` should be enough to update
  `appit`.

## v0.1.1 (2023-12-18)

This release only contains fixed links in the README. No code was changed.

## v0.1.0 (2023-12-18)

This is the initial alpha release of Cushy.
