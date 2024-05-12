# Input

The [`Input`][Input] widget is a basic text entry widget. It supports
generic-driven storage for some intuitive behaviors:

- When using a `String`, a default text entry widget is used.
- When using a [`MaskedString`][MaskedString], the text entry will be masked and
  the contained value will be zeroed before the storage is freed.

When an `Input` is masked, the system input manager is notified that it is a
password entry field.

[Input]: <{{ docs }}/widgets/input/struct.Input.html>
[MaskedString]: <{{ docs }}/widgets/input/struct.MaskedString.html>
