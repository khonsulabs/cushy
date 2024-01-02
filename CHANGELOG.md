# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Breaking Changes

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

### Changed

- `WidgetCacheKey` now includes the `KludgineId` of the context it was created
  from. This ensures if a `WidgetInstance` moves or is shared between windows,
  the cache is invalidated.
- All `Dynamic` mapping functions now utilize weak references, and clean up as
  necessary if a value is not able to be upgraded.

### Added

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
