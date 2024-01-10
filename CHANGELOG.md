# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- markdownlint-disable no-duplicate-heading -->

## Unreleased

### Breaking Changes

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
- Compatibility with Rust v1.70.0 has been restored, and continuous integration
  testing the MSRV has been added.
- `Progress` now utilizes `IntoSource<Progress>` instead of
  `IntoDynamic<Progress>`. In general, this should not cause any code breakages
  unless the traits were being used in generics.
- `Space` now honors `ConstraintLimit::Fill` in its layout.
- When handling the Ctrl/Cmd+W shortcut to close windows, repeat keys are now
  ignored.

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
- `HslPicker` is a color picker for `Hsl` colors.
- `LightnessPicker` is a picker of lightness values.

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
