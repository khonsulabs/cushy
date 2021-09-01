//! A graphical user interface library. This crate exposes all of the built-in
//! functionality of Gooey, as well as types for building apps.
//!
//! ## Feature Flags
//!
//! This crate has several feature flags to control what features are enabled.
//! The default feature flags are `["frontend-kludgine", "frontend-browser", "fluent"]`.
//!
//! * `frontend-browser`: Enables the `frontends::browser` module, which is
//!   `gooey-browser` re-exported.
//! * `frontend-kludgine`: Enables the `frontends::rasterizer` module and the
//!   `frontends::renderers::kludgine` module. These are re-exports of
//!   `gooey-rasterizer` and `gooey-kludgine` respectively.
//! * `async`: Enables the `App::spawn()` function.
//! * `fluent`: Enables using [`fluent`](https://crates.io/crates/fluent) for
//!   localization.
//!
//! ## Top-level exports
//!
//! The `ActiveFrontend`, `main()`, and `main_with()` function will change types
//! based on the feature flags enabled as well as the target platform. Here is a
//! reference to understand what symbols to expect:
//!
//! | `target_arch = "wasm32"` | `frontend-kludgine` | `frontend-browser` | `ActiveFrontend`/`main()` |
//! | - | - | - | - |
//! | `false` | `false` | `false` | |
//! | `false` | `false` | `true`  | |
//! | `false` | `true`  | `false` | `Rasterizer<Kludgine>`/`kludgine_main` |
//! | `false` | `true`  | `true`  | `Rasterizer<Kludgine>`/`kludgine_main` |
//! | `true`  | `false` | `true`  | `WebSys`/`browser_main` |
//! | `true`  | `true`  | `true`  | `WebSys`/`browser_main` |

#![forbid(unsafe_code)]
#![warn(
    clippy::cargo,
    missing_docs,
    clippy::pedantic,
    future_incompatible,
    rust_2018_idioms
)]
#![allow(
    clippy::if_not_else,
    clippy::module_name_repetitions,
    clippy::multiple_crate_versions, // this is a mess due to winit dependencies and wgpu dependencies not lining up
)]
#![cfg_attr(doc, warn(rustdoc::all))]

/// Available [`Frontends`](gooey_core::Frontend).
pub mod frontends {
    #[cfg(feature = "frontend-browser")]
    #[doc(inline)]
    pub use gooey_browser as browser;
    #[cfg(feature = "gooey-rasterizer")]
    #[doc(inline)]
    pub use gooey_rasterizer as rasterizer;
    /// Available [`Renderers`](gooey_renderer::Renderer).
    pub mod renderers {
        #[cfg(all(feature = "frontend-kludgine", not(target_arch = "wasm32")))]
        #[doc(inline)]
        pub use gooey_kludgine as kludgine;
    }
}
use cfg_if::cfg_if;
#[doc(inline)]
pub use gooey_core as core;
#[cfg(feature = "fluent")]
#[doc(inline)]
pub use gooey_fluent as fluent;
#[doc(inline)]
pub use gooey_renderer as renderer;
#[doc(inline)]
pub use gooey_text as text;
#[doc(inline)]
pub use gooey_widgets as widgets;

#[cfg(all(feature = "frontend-kludgine", not(target_arch = "wasm32")))]
mod headless;
#[cfg(all(feature = "frontend-kludgine", not(target_arch = "wasm32")))]
pub use headless::{Headless, HeadlessError, Recorder};

#[cfg(all(feature = "frontend-kludgine", not(target_arch = "wasm32")))]
mod kludgine;
#[cfg(all(feature = "frontend-kludgine", not(target_arch = "wasm32")))]
pub use kludgine::{kludgine_app, kludgine_main, kludgine_main_with, kludgine_run};

#[cfg(feature = "frontend-browser")]
mod browser;
#[cfg(feature = "frontend-browser")]
pub use browser::{browser_app, browser_main, browser_main_with, browser_run};

cfg_if! {
    if #[cfg(all(target_arch = "wasm32", feature = "frontend-browser"))] {
        pub use browser_main as main;
        pub use browser_main_with as main_with;
        pub use browser_app as app;
        pub use browser_run as run;
        /// The active frontend.
        pub type ActiveFrontend = gooey_browser::WebSys;
    } else if #[cfg(feature = "frontend-kludgine")] {
        pub use kludgine_main as main;
        pub use kludgine_main_with as main_with;
        pub use kludgine_app as app;
        pub use kludgine_run as run;
        /// The active frontend.
        pub type ActiveFrontend = gooey_rasterizer::Rasterizer<gooey_kludgine::Kludgine>;
    }
}

mod app;

pub use app::App;
/// Styles for applications.
pub mod style;
