# gooey

![Gooey is considered experimental and unsupported](https://img.shields.io/badge/status-experimental-blueviolet)
[![crate version](https://img.shields.io/crates/v/gooey.svg)](https://crates.io/crates/gooey)
[![Live Build Status](https://img.shields.io/github/workflow/status/khonsulabs/gooey/Tests/main)](https://github.com/khonsulabs/gooey/actions?query=workflow:Tests)
[![HTML Coverage Report for `main` branch](https://khonsulabs.github.io/gooey/coverage/badge.svg)](https://gooey.rs/coverage/)
[![Documentation for `main` branch](https://img.shields.io/badge/docs-main-informational)](https://gooey.rs/main/gooey/)

**Warning:** This crate is incredibly early in development.

This is an attempt to write a cross-platform framework for creating user interfaces in Rust.

It is born from the ashes of the former UI system in [Kludgine](https://github.com/khonsulabs/kludgine). It aims to provide a framework that makes it easy to create descriptive and reative interfaces.

![gooey architecture](./Gooey.png)

At the core of the architecture is the principle that `gooey` can't ever hope to cover every possible widget/control that every platform has access to, but it expects that you could add support for that widget without needing to submit PRs to this repository.

That being said, the main crate aims to provide two frontends:

* WASM-based virtual dom: The ability to convert gooey interfaces into an interactive browser application. The vdom implementation hasn't been decided upon yet.
* Kludgine theme-able rendering: The ability to run efficiently and natively using wgpu on a variety of platforms.

## `gooey-core`

The core crate will consist of platform-agnostic code including:

* `Widget` trait: Each user interface element must implement this trait.
* Layout logic: Basic layout strategies including grid and absolute layout.
* `Transmogrifier` trait: To render a `Widget`, a `Transmogrifier` must exist for the `Frontend` being used.

## `gooey-widgets`

The core widgets that are built-in. At this level, the cross-platform definitions and functionality are provided.

## Frontends

These crates define the API that users creating applications will use to initialize and run their user interfaecs. These crates will also define any traits needed to implement `Transmogrifier`s for the front end in question.

These crates will also implement all widgets defined by gooey-core. These crates are:

* `gooey-rasterizer`: [`Rasterizer`](https://gooey.rs/main/gooey/frontends/rasterizer/struct.Rasterizer.html) frontend. Requires a [`Renderer`](https://gooey.rs/main/gooey/core/renderer/trait.Renderer.html).
* `gooey-browser`: [`WebSys`](https://gooey.rs/main/gooey/frontends/browser/struct.WebSys.html) frontend. See [`gooey/examples/basic.rs`](./gooey/examples/basic.rs), or run `cargo xtask build-browser-example`.

## Renderers

These crates implement [`Renderer`](https://gooey.rs/main/gooey/core/renderer/trait.Renderer.html) for an environment where raw drawing APIs are the only tools available to display a user interface. For example, inside of a game. The only renderer currently being developed is:

* `gooey-kludgine`: Provides the [`Kludgine`](https://gooey.rs/main/gooey/frontends/renderers/kludgine/struct.Kludgine.html) renderer. See [`gooey/examples/basic.rs`](./gooey/examples/basic.rs) or run `cargo run --example basic`.

## `gooey`

The omnibus crate wraps all of the crates into a single consumable crate.
