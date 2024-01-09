# Cushy

![Cushy is considered alpha and unsupported](https://img.shields.io/badge/status-alpha-orange)
[![crate version](https://img.shields.io/crates/v/cushy.svg)](https://crates.io/crates/cushy)
[![Documentation for `main`](https://img.shields.io/badge/docs-main-informational)](https://cushy.rs/main/docs/cushy/)
[![Cushy User's Guide](https://img.shields.io/badge/user%27s%20guide-main-informational)][guide]

Cushy is an experimental Graphical User Interface (GUI) crate for the Rust
programming language. It features a reactive data model and aims to enable
easily creating responsive, efficient user interfaces. To enable easy
cross-platform development, Cushy uses its own collection of consistently-styled
[`Widget`s][widget].

Cushy is powered by:

- [`Kludgine`][kludgine], a 2d graphics library powered by:
  - [`winit`][winit] for windowing/input
  - [`wgpu`][wgpu] for graphics
  - [`cosmic_text`][cosmic_text] for text layout + rasterization
- [`palette`][palette] for OKLab-based HSL color calculations
- [`arboard`][arboard] for clipboard support
- [`figures`][figures] for integer-based 2d math

## Getting Started with Cushy

The [`Widget`][widget] trait is the building block of Cushy: Every user
interface element implements `Widget`. The `Widget` trait
[documentation][widget] has an overview of how Cushy works. A list of built-in
widgets can be found in the [`cushy::widgets`][widgets] module.

Cushy uses a reactive data model. To see [an example][button-example] of how
reactive data models work, consider this example that displays a button that
increments its own label:

```rust,ignore
fn main() -> cushy::Result {
    // Create a dynamic usize.
    let count = Dynamic::new(0_isize);

    // Create a new label displaying `count`
    count
        .to_label()
        // Use the label as the contents of a button
        .into_button()
        // Set the `on_click` callback to a closure that increments the counter.
        .on_click(move |_| count.set(count.get() + 1))
        // Run the application
        .run()
}
```

Here are some ways to learn more about Cushy:

- Explore the [examples directory][examples]. Nearly every feature in Cushy was
initially tested by creating an example.
- Browse the [user's guide][guide].
- Ask questions [in Discussions][discussions] or [on Discord][discord].

## Project Status

This project is early in development, but is quickly becoming a decent
framework. It is considered alpha and unsupported at this time, and the primary
focus for [@ecton][ecton] is to use this for his own projects. Feature requests
and bug fixes will be prioritized based on @ecton's own needs.

If you would like to contribute, bug fixes are always appreciated. Before
working on a new feature, please [open an issue][issues] proposing the feature
and problem it aims to solve. Doing so will help prevent friction in merging
pull requests, as it ensures changes fit the vision the maintainers have for
Cushy.

[widget]: https://cushy.rs/main/docs/cushy/widget/trait.Widget.html
[widgets]: https://cushy.rs/main/docs/cushy/widgets/index.html
[button-example]: https://github.com/khonsulabs/cushy/tree/main/examples/basic-button.rs
[examples]: https://github.com/khonsulabs/cushy/tree/main/examples/
[kludgine]: https://github.com/khonsulabs/kludgine
[figures]: https://github.com/khonsulabs/figures
[wgpu]: https://github.com/gfx-rs/wgpu
[winit]: https://github.com/rust-windowing/winit
[cosmic_text]: https://github.com/pop-os/cosmic-text
[palette]: https://github.com/Ogeon/palette
[arboard]: https://github.com/1Password/arboard
[ecton]: https://github.com/khonsulabs/ecton
[issues]: https://github.com/khonsulabs/cushy/issues
[guide]: https://cushy.rs/main/guide/
[discussions]: https://github.com/khonsulabs/cushy/discussions
[discord]: https://discord.khonsulabs.com/

## Open-source Licenses

This project, like all projects from [Khonsu Labs](https://khonsulabs.com/), is open-source.
This repository is available under the [MIT License](./LICENSE-MIT) or the
[Apache License 2.0](./LICENSE-APACHE).

To learn more about contributing, please see [CONTRIBUTING.md](./CONTRIBUTING.md).
