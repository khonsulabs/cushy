# Expand

The [`Expand`][expand] widget expands its child to fill as much space as
available.

The Expand widget can be constructed to expand horizontally and/or vertically:

- Expand the child's width and height: [`Expand::new`][new]/[`MakeWidget::expand`][mw-expand]
- Expand the child's width only:
  [`Expand::horizontal`][horizontal]/[`MakeWidget::expand_horizontally`][mw-expand-h]
- Expand the child's height only:
  [`Expand::vertical`][vertical]/[`MakeWidget::expand_vertically`][mw-expand-v]

[expand]: <{{ docs }}/widgets/struct.Expand.html>
[new]: <{{ docs }}/widgets/struct.Expand.html#method.new>
[vertical]: <{{ docs }}/widgets/struct.Expand.html#method.vertical>
[horizontal]: <{{ docs }}/widgets/struct.Expand.html#method.horizontal>
[mw-expand]: <{{ docs }}/widget/trait.MakeWidget.html#method.expand>
[mw-expand-v]: <{{ docs }}/widget/trait.MakeWidget.html#method.expand_vertically>
[mw-expand-h]: <{{ docs }}/widget/trait.MakeWidget.html#method.expand_horizontally>
