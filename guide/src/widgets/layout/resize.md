# Resize

The [`Resize`][resize] widget constrains and/or overrides its child's size.

The resize widget uses [`DimensionRange`s][dimension-range] to specify the
allowed range of dimensions that its child can use. `DimensionRange` implements
`From` for all of the built-in range types in Rust with [`Lp`][lp], [`Px`][px], or [`Dimension`][dimension] bounds.

[resize]: <{{ docs }}/widgets/struct.Resize.html>
[dimension-range]: <{{ docs }}/styles/struct.DimensionRange.html>
[dimension]: <{{ docs }}/styles/enum.Dimension.html>
[lp]: <https://docs.rs/figures/latest/figures/units/struct.Lp.html>
[px]: <https://docs.rs/figures/latest/figures/units/struct.Px.html>
