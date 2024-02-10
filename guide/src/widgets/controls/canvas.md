# Canvas

The [`Canvas`][canvas] widget invokes a function each time it needs to paint.
This function has access to a graphics context exposing most of
[Kludine][kludgine]'s 2D graphics API.

A [`Tick`][tick] can be attached to the `Canvas` to have a callback invoked at a
steady rate. This tick function can be used to update the state of the `Canvas`,
and it can signal when the `Canvas` should be redrawn.

[canvas]: <{{ docs }}/widgets/struct.Canvas.html>
[tick]: <{{ docs }}/struct.Tick.html>
[kludgine]: <https://github.com/khonsulabs/kludgine>
