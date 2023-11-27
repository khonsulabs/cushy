![Gooey is considered experimental and unsupported](https://img.shields.io/badge/status-prototype-blueviolet)
[![crate version](https://img.shields.io/crates/v/gooey.svg)](https://crates.io/crates/gooey)
[![Documentation for `main` branch](https://img.shields.io/badge/docs-main-informational)]($docs$)

Gooey is an experimental Graphical User Interface (GUI) crate for the Rust
programming language. It is powered by:

- [`Kludgine`][kludgine], a 2d graphics library powered by:
  - [`winit`][winit] for windowing/input
  - [`wgpu`][wgpu] for graphics
  - [`cosmic_text`][cosmic_text]
- [`palette`][palette]
- [`arboard`][arboard]

## Getting Started with Gooey

The [`Widget`][widget] trait is the building block of Gooey: Every user
interface element implements `Widget`. A full list of built-in widgets can be
found in the [`gooey::widgets`][widgets] module.

Gooey uses a reactive data model. To see [an example][button-example] of how
reactive data models work, consider this example that displays a button that
increments its own label:

```rust,ignore
$../examples/basic-button.rs:readme$
```

A great way to learn more about Gooey is to explore the [examples
directory][examples]. Nearly every feature in Gooey was initially tested by
creating an example.

## Project Status

This project is early in development, but is quickly becoming a decent
framework. It is considered experimental and unspported at this time, and the
primary focus for [@ecton][ecton] is to use this for his own projects. Feature
requests and bug fixes will be prioritized based on @ecton's own needs.

[widget]: $widget$
[kludgine]: https://github.com/khonsulabs/kludgine
[wgpu]: https://github.com/gfx-rs/wgpu
[winit]: https://github.com/rust-windowing/winit
[widgets]: $widgets$
[button-example]: https://github.com/khonsulabs/gooey/tree/$ref-name$/examples/basic-button.rs
[examples]: https://github.com/khonsulabs/gooey/tree/$ref-name$/examples/
[cosmic_text]: https://github.com/pop-os/cosmic-text
[palette]: https://github.com/Ogeon/palette
[arboard]: https://github.com/1Password/arboard
[ecton]: https://github.com/khonsulabs/ecton
