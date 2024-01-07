# Cushy's Philosophies

There are a lot of GUI libraries with wildly varying approaches to how UIs are
displayed. Here's the philosophies that drive Cushy's design:

- Cushy retains information between redraws so that many events can be handled
  without redrawing the user interface.
- [Everything is a widget](./widgets.md). The "root" of a user interface/window
  is a widget, and widgets can contain other widgets.
- Composition is powerful and easy to reason about. The built-in widget library
  is aimed at providing a suite of single-purpose widgets that can be composed
  to create more complex user interfaces.
- If a developer dislikes a built-in widget's behavior, they should be empowered
  to create their own that behaves the way they desire. To ensure developers
  have this flexibility, all provided widgets must only utilize functionality
  that is publicly available.
- Widgets should be flexible in the types they support, prefering trait
  implementations instead of hard-coded types. For example, the Label widget
  supports any type that implements `Display`.
- Cushy needs both physical pixel and resolution independent measurement types.
  UI designers want to use real-world measurements that scale based on the DPI
  resolution of the device it is being rendered on. Widget authors and game
  developers want to work with pixel-perfect measurements to ensure perfect
  alignment.

From an implementation standpoint, Cushy has these goals:

- For graphics, provide a wgpu-centric library that exposes a rendering API
  inspired by wgpu's Encapsulating Graphics Work article.
- For windowing, embrace winit and route input events to the correct widgets.
  This allows widgets to support any features that winit can support.
- Cushy should be able to idle at close to 0% CPU. Cushy should not redraw
  unless needed.
