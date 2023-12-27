# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Breaking Changes

- Many bounds required `UnwindSafe` due to a misunderstanding on how to handle
  this trait in `appit`. All requirements for `UnwindSafe` have been removed.
- `Gooey` no longer implements default. To gain access to a `Gooey` instance,
  create a `PendingApp` or get a reference to the running `App`.
- `Window::new` no longer accepts a `Gooey` parameter. The window now adopts the
  `Gooey` from the application it is opened within.
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
  windows before starting Gooey as well as dynamically opening windows at
  runtime.
- `Window::on_close` sets a callback to be invoked when the window has closed.
- `WindowHandle` is a handle to a Gooey window. It enables requesting that the
  window closes, refreshing the window, or invalidating a widget contained in
  the window.
- `RunningWindow::handle()` returns a `WindowHandle` for the current window.
- `RunningWindow::request_close()` requests that the window should close. This
  ensures `WindowBehavior::close_requested` is invoked before the window is
  closed.
- `PendingWindow` is a new type that can return a `WindowHandle` for a window
  that hasn't opened yet. This can be used to allow a widget on a window to
  close the window.

[91]: https://github.com/khonsulabs/gooey/issues/91
[92]: https://github.com/khonsulabs/gooey/issues/92
[112]: https://github.com/khonsulabs/gooey/issues/112
[113]: https://github.com/khonsulabs/gooey/issues/113

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

[94]: https://github.com/khonsulabs/gooey/pull/94
[97]: https://github.com/khonsulabs/gooey/issues/97

## v0.1.2 (2023-12-18)

### Fixed

- Gooey now compiles for Windows. An indirect dependency, `appit`, also needs to
  be updated to v0.1.1. Running `cargo update` should be enough to update
  `appit`.

## v0.1.1 (2023-12-18)

This release only contains fixed links in the README. No code was changed.

## v0.1.0 (2023-12-18)

This is the initial alpha release of Gooey.
