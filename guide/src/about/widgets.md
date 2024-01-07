# Everything is a `Widget`

A widget is a rectangular area of a screen that implements the
[`Widget`][widget] trait. Widgets are the fundamental building block of Cushy.

The `Widget` trait can look daunting, as it defines every possible function a
`Widget` might need in a graphical user interface. Thankfully, the details of
how this trait works can be ignored until you're ready to create custom widgets.

Developing a user interface in Cushy is a two-step process: gather the
information for the interface and present the information in one or more
widgets.

Cushy makes the process of creating widgets easy through the
[`MakeWidget`][makewidget] trait. Every `Widget` implementor automatically
implements `MakeWidget`, but it can also be implemented by any type to make it
easy to utilize within Cushy. For example, `String` implements `MakeWidget` by
returning a [`Label`][label]. This approach can also be used to convert complex
structures into multi-widget components without needing to create any new
`Widget` implementations.

`MakeWidget` is also responsible for why `"Hello, World".run()` works. The
[`Run`][run] trait is automatically implemented for all `MakeWidget`
implementations. The implementation simply creates a [`Window`][window] from the
widget and runs it:

```rust,no_run,no_playground
{{#include ../../../src/widget.rs:run}}
```

So now that we know our goal is to create one or more widgets to represent our
data, how do we transform our data and application state into widgets?

[widget]: <{{ docs }}/widget/trait.Widget.html>
[makewidget]: <{{ docs }}/widget/trait.MakeWidget.html>
[label]: <{{docs}}/widgets/label/struct.Label.html>
[run]: <{{ docs }}/trait.Run.html>
[window]: <{{ docs }}/window/struct.Window.html>
