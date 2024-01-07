# Wrap

The [`Wrap`][wrap] widget lays its [`Children`][children] widgets out in a
fashion that mimics text layout.

It works by measuring each child with [`SizeToFit`][size-to-fit] and laying out
the widgets into a series of rows. A fixed amount of [spacing][spacing] between
each widget can be applied.

Once the widgets have been grouped into rows, the [alignment][align] and
[vertical alignment][v-align] are applied to position the widgets on each row.
[`WrapAlign`][wrapalign] can be any of these strategies:

- `Start`: Position the widgets at the start of the line, honoring
  [`LayoutOrder`][layoutorder].
- `End`: Position the widgets at the end of the line, honoring
  [`LayoutOrder`][layoutorder].
- `Center`: Position the widgets centered on each line.
- `SpaceBetween`: Position the elements evenly along the line with no space
  before the first widget or after the last widget.
- `SpaceEvenly`: Position the elements evenly along the line with an additional
  half of the spacing between elements before the first widget and after the
  last widget.
- `SpaceAround`: Position the elements evenly along the line with an additional
  equal amount of spacing before the first widget and after the last widget.

[wrap]: <{{ docs }}/widgets/wrap/struct.Wrap.html>
[wrapalign]: <{{ docs }}/widgets/wrap/enum.WrapAlign.html>
[children]: <{{ docs }}/widget/struct.Children.html>
[size-to-fit]: <{{ docs }}/enum.ConstraintLimit.html#variant.SizeToFit>
[spacing]: <{{ docs }}/widgets/wrap/struct.Wrap.html#method.spacing>
[align]: <{{ docs }}/widgets/wrap/struct.Wrap.html#method.align>
[v-align]: <{{ docs }}/widgets/wrap/struct.Wrap.html#method.vertical_align>
[layoutorder]: <{{ docs }}/styles/components/struct.LayoutOrder.html>
