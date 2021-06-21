# Understanding Gooey's Core Concepts

## `Widget` trait

The `Widget` trait ([documentation](https://gooey.rs/main/gooey/core/trait.Widget.html)) is used to define all user interface elements. This is similar to a View, Control, or Component in other user interface frameworks.

In `Gooey` a `Widget` is what you will create your cross-platform interfaces with. The built-in widgets are:

- [Button](../widgets/button.md)
- [Component](../widgets/component.md)
- [Container](../widgets/container.md)
- [Label](../widgets/label.md)

A `Widget` has no knowledge about how it's being presented. When you present your user interface, you will pick a [`Frontend`](#frontend-trait). The [`Transmogrifier`](#transmogrifier-trait) is responsible for presenting the `Widget` in the active `Frontend`.

### Implementing a new `Widget`

Implementing a new widget requires adding the `Widget` implementor and a `Transmogrifier` implementor for each `Frontend` you wish to support. For example, [`Button`](../widgets/button.md) provides the cross-platform API that allows you to use a push button in your interface. It also exposes [`ButtonTransmogrifier`](https://gooey.rs/main/gooey/widgets/button/struct.ButtonTransmogrifier.html), which is the type that implements `Transmogrifier` for the supported `Frontends`.

## `Frontend` trait

The `Frontend` trait ([documentation](https://gooey.rs/main/gooey/core/trait.Frontend.html)) is what presents a user interface.

`Gooey` supports two frontends:

- [Browser](../frontends/browser/web-sys.md)
- [Native (Rasterized)](../frontends/rasterizer/native.md)

Each `Frontend` will define the necessary APIs that [`Transmogrifiers`](#transmogrifier-trait) need to implement [`Widgets`](#widget-trait). In general, you only need to know the details about `Transmogrifiers` and `Frontends` if you're implementing new `Widgets` or `Transmogrifiers`.

## `Transmogrifier` trait

The `Transmogrifier` trait ([documentation](https://gooey.rs/main/gooey/core/trait.Transmogrifier.html)) presents a [`Widget`](#widget-trait) with a [`Frontend`](#frontend-trait). If a `Widget` doesn't have a `Transmogrifier` for a given `Frontend`, it will not be able to be used on that `Frontend`. Never fear, all `Widgets` built into `Gooey` support the two built-in `Frontends`: [`WebSys`](../frontends/browser/web-sys.md) and [`Rasterizer`](../frontends/rasterizer/native.md).

One of the unique designs of `Gooey` is that if a `Transmogrifier` doesn't exist for a `Frontend` you wish to target, or if a `Transmogrifier` doesn't support a feature you need, you can implement a new `Transmogrifier` and replace it in your application. This flexility ensures that you will always have the ability to implement new `Frontends` or add support to existing `Widgets` for an unsupported `Frontend`.