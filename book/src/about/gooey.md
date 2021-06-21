# About Gooey

`Gooey` is a graphical user interface framework for Rust. It aims to provide a cross-platform way for developing applications to achieve the "write once, deploy anywhere" holy grail.

Where it differs from many is in its core design philosophy: You should never feel boxed in by limitations of your UI framework. In many other frameworks, if you use a third party widget, and you want to target a new platform that the widget doesn't support, you usually need to ask the library author to add support for that platform. Not so with `Gooey`!

In `Gooey`, the roles of [`Widgets`](./concepts.md#widget-trait), [`Transmogrifiers`](./concepts.md#transmogrifier-trait), and [`Frontends`](./concepts.md#frontends-trait) are clearly defined, and you would be able to implement your own `Transmogrifier` for the unsupported widget. Or, if a platform implementation leaves something to be desired, you can use your own `Transmogrifier` for that as well.

While this may not be the first framework to take this approach, it is not a common approach, and it's one thing that separates `Gooey` from many user interface frameworks.
