# Grid

The [`Grid`][grid] widget lays out a set of widgets in a two dimensional grid.

It is constructed with a primary orientation, which can be given a set of
[`GridDimension`][grid-dimension] to affect how the opposite orientation's
elements are measured.

For example, to create a grid that resembles a traditional table, use
[`Grid::from_rows`][from-rows] to create the grid, and
[`Grid::dimensions`][dimensions] would be used to control each column's
measurement strategy.

Alternatively when creating a grid with [`Grid::from_columns`][from-columns],
[`Grid::dimensions`][dimensions] is instead used to control each row's
measurement strategy.

[grid]: <{{ docs }}/widgets/grid/struct.Grid.html>
[from-rows]: <{{ docs }}/widgets/grid/struct.Grid.html#method.from_rows>
[from-columns]: <{{ docs }}/widgets/grid/struct.Grid.html#method.from_columns>
[dimensions]: <{{ docs }}/widgets/grid/struct.Grid.html#method.dimensions>
[grid-dimension]: <{{ docs }}/widgets/grid/enum.GridDimension.html>
