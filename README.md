# Gooey

![Gooey is considered experimental and unsupported](https://img.shields.io/badge/status-prototype-blueviolet)
[![crate version](https://img.shields.io/crates/v/gooey.svg)](https://crates.io/crates/gooey)

Gooey is a cross-platform Graphical User Interface (GUI) crate for Rust. **This crate is
incredibly early in development and is not ready to be used to develop
applications.**

Gooey embraces a reactive model for connecting widgets with each other and
non-Gooey code. For example, this snippet shows how to build a counter with plus
and minus buttons to increment and decrement the value.

```rust
fn main() {
    gooey::run(gooey_widgets::widgets(), |cx| {
        let counter = cx.new_dynamic(0i32);
        let label = counter.map_each(|count| count.to_string()).unwrap();

        Flex::rows(
            Children::new(cx)
                .with_widget(Label::new(label, cx))
                .with_widget(Button::new("+").on_click(move |_| {
                    counter.set(counter.get().unwrap() + 1);
                }))
                .with_widget(Button::new("-").on_click(move |_| {
                    counter.set(counter.get().unwrap().saturating_sub(1));
                })),
        )
    })
}
```

In this example, `counter` and `label` are `Dynamic` values. `Dynamic` values
are able to be updated and have their changes observed. `label` is automatically updated each time `counter` is changed.

The `on_click` callbacks for each button update `counter`, which automatically
updates `label`, which the `Label` widget reacts to by displaying the new
caption.

## What makes Gooey different?

Gooey is designed with a few core principles in mind:

- **Write once, deploy anywhere.** Gooey applications should be able to be
  written once and deployed to different technology stacks. This repository aims
  to have two *frontends*:

  - `gooey-web`: Run Gooey applications in a browser using `wasm-bindgen` and
    `web-sys`. `gooey-web` utilizes native DOM elements, unlike most other
    frameworks that render the entire application using a `<Canvas>` element.
  - `gooey-raster`: The ability to power and render a user interface using any
    `Renderer` implementor. `gooey-kludgine` provides a `wgpu` and `winit`
    powered implementation.

  `gooey-core` contains no knowledge of `gooey-web` or `gooey-raster`. This
  means that, in theory, any way of presenting a graphical user interface could
  be possible by implemented another `Frontend`.

  - **Cross-platform appearance and behavior**: Widgets are expected to look and
    feel similarly on every platform that Gooey targets. This is a tradeoff
    aimed at making apps easier to support and more consistent.
  - ****:

- **No special widgets.** The architecture of Gooey separates `gooey-widgets`
  from `gooey-core` and each `Frontend`. This ensures that all built-in widgets
  are implemented using only public APIs available to all developers.

- **Powered by reactivity.** Once the application is running, Gooey doesn't have
  any active runtime powering it. As changes happen, each widget affected by
  those changes updates only what is necessary.

## Crates

This repository contains many crates that power Gooey. At the root of the
repository is the `gooey` crate, which is the crate applications will generally
wish to import. It is powered by the other crates, available in the `crates/`
directory in the repository.

- `gooey-reactor`: A platform-independent, forbid-unsafe reactive system.
- `gooey-core`: Defines all of the cross-platform structures and traits.
- `gooey-web`: Defines the `WebApp` type which implements
  `gooey_core::Frontend`.
- `gooey-raster`: Defines the `RasterizedApp<Surface>` type which implements
  `gooey_core::Frontend` powered by a `Surface` implementor.
- `gooey-widgets`: Defines the provided widgets, and implements
  *transmogrifiers* for `WebApp` and `RasterizedApp`.
- `gooey-kludgine`: Defines `Kludgine` which implements `gooey_raster::Surface`.
  Additionally, provides `KludgineRenderer` which implements
  `gooey_core::graphics::Renderer`. This allows rasterized applications to be
  built using [Kludgine][kludgine], which is powered by `wgpu` and `winit`.

## Open-source Licenses

This project, like all projects from [Khonsu Labs](https://khonsulabs.com/), are
open-source. This repository is available under the [MIT License](./LICENSE-MIT)
or the [Apache License 2.0](./LICENSE-APACHE).

[kludgine]: https://github.com/khonsulabs/kludgine
