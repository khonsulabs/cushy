# Stack

The [`Stack`][stack] widget lays a set of [`Children`][children] as either a set
of columns or rows. It is a convenient way to construct a 1D
[`Grid`](./grid.md). It can be constructed using either:

- [`Stack::rows`][rows]/[`Children::into_rows()`][into_rows]
- [`Stack::columns`][columns]/[`Children::into_columns()`][into_columns]

The stack widget places spacing between each element called a [gutter][gutter].

[stack]: <{{ docs }}/widgets/stack/struct.Stack.html>
[children]: <{{ docs }}/widget/struct.Children.html>
[rows]: <{{ docs }}/widgets/stack/struct.Stack.html#method.rows>
[columns]: <{{ docs }}/widgets/stack/struct.Stack.html#method.columns>
[into_columns]: <{{ docs }}/widget/struct.Children.html#method.into_columns>
[into_rows]: <{{ docs }}/widget/struct.Children.html#method.into_rows>
[gutter]: <{{ docs }}/widgets/stack/struct.Stack.html#method.gutter>
