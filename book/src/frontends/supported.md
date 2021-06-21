# Supported Frontends

The [`Frontends`](../about/concepts.md#frontend-trait) supported by `Gooey` are:

- [`gooey-browser`](./browser/web-sys.md): Allows deploying applications to modern browsers through WebAssembly.
- [`gooey-rasterizer`](./rasterizer/native.md): Allows deploying native applications using a `Renderer`.
  - `gooey-kludgine`: A `Renderer` implementation targeting [`wgpu`](https://github.com/gfx-rs/wgpu) using [`Kludgine`](https://github.com/khonsulabs/kludgine).
