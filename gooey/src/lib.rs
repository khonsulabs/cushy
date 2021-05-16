pub mod frontends {

    #[cfg(feature = "frontend-browser")]
    #[doc(inline)]
    pub use gooey_browser as browser;
    #[cfg(feature = "gooey-rasterized")]
    #[doc(inline)]
    pub use gooey_rasterized as rasterized;
    pub mod renderers {
        #[cfg(feature = "frontend-kludgine")]
        #[doc(inline)]
        pub use gooey_kludgine as kludgine;
    }
}
#[doc(inline)]
pub use gooey_core as core;
#[doc(inline)]
pub use gooey_widgets as widgets;
