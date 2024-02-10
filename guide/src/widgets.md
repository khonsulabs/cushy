# Widgets

Types that implement [`Widget`][widget] are the [building
blocks](./about/widgets.md) of Cushy user interfaces. The built-in widgets each
aim to serve a single purpose or solve a single problem. [Through
composition](./about/composition.md), complex user interfaces can be built by
combining these single-purpose widgets.

This section is organized into four categories of widgets:

- [Multi-widget Layout Widgets](./multi-layout.md): Widgets that are designed to
  layout multiple widgets as a single widget.
- [Single-widget Layout Widgets](./single-layout.md): Widgets that are designed
  to influence a single widget's layout.
- [Controls](./controls.md): Widgets that are visible to the user to present
  and/or interact with data.
- [Utility Widgets](./utility.md): Widgets that have no direct layout or visual
  presentation. This type of widget usually associates extra data that may
  impact how child widgets are presented.

[widget]: <{{ docs }}/widget/trait.Widget.html>
