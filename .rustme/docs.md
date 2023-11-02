![Gooey is considered experimental and unsupported](https://img.shields.io/badge/status-prototype-blueviolet)
[![crate version](https://img.shields.io/crates/v/gooey.svg)](https://crates.io/crates/gooey)
[![Documentation for `main` branch](https://img.shields.io/badge/docs-main-informational)]($docs$)

Gooey is an experimental Graphical User Interface (GUI) crate for the Rust
programming language. It is built using [`Kludgine`][kludgine], which is powered
by [`winit`][winit] and [`wgpu`][wgpu]. It is incredibly early in development,
and is being developed for a game that will hopefully be developed shortly.

The [`Widget`][widget] trait is the building block of Gooey: Every user
interface element implements `Widget`. A full list of built-in widgets can be
found in the [`gooey::widgets`][widgets] module.

Gooey uses a reactive data model. To see [an example][button-example] of how
reactive data models work, consider this example that displays a button that
increments its own label:

```rust
$../examples/button.rs:readme$
```

[widget]: $widget$
[kludgine]: https://github.com/khonsulabs/kludgine
[wgpu]: https://github.com/gfx-rs/wgpu
[winit]: https://github.com/rust-windowing/winit
[widgets]: $widgets$
[button-example]: https://github.com/khonsulabs/gooey/tree/$ref-name$/examples/button.rs
