# Creating a Widget

There are generally two types of [`Widgets`](./concepts.md#widget-trait) you might find yourself wanting to create. The most common approach will be to use a `Component` to create a new widget using other widgets to power the component. For example, a "New User" form would be a `Component` with multiple text fields, labels, and buttons. This approach allows you to write [`Frontend`](./concepts.md#frontend-trait)-independent widgets. To learn more, read the chapter [about `Components`](../widgets/component.md).

If the widget you want to create can't be implemented using existing widgets, you will need to implement a new widget.

## Implementing Widget

TODO Coming soon.

For now, the simplest widget implementation is `Label` and can be viewed [in `gooey-widgets` here](https://github.com/khonsulabs/gooey/blob/main/widgets/src/label.rs).
