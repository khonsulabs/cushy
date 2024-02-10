# Container

The [`Container`][container] widget encloses a widget in a visual container.

The `Container`'s background color can be either specified explicitly, set using
a [`ContainerLevel`][container-level], or automatically selected based on the
current container level. Each container level has an associated theme color.

When using automatic container levels and the highest-level container level is
reached, the level will wrap to the lowest level.

[container]: <{{ docs }}/widgets/container/struct.Container.html>
[container-level]: <{{ docs }}/styles/enum.ContainerLevel.html>
