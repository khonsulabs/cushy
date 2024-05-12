# ProgressBar

The [`ProgressBar`][ProgressBar] widget draws an indicator to represent
progress. A progress bar can be indeterminant, empty, partially complete, or
fully complete.

The [`Progressable`][Progressable] trait allows many types to be used within progress bars by
implementing one of several helper traits:

- [`ProgressValue`][ProgressValue]: This trait can be implemented for full control over how
      progress is reported.
- Types that ipmlement [`Ranged`][Ranged] and [`PercentBetween`][PercentBetween]
  have [`ProgressValue`][ProgressValue] implemented automatically. This includes
  all numeric types in Rust..

[ProgressBar]: <{{ docs }}/widgets/progress/struct.ProgressBar.html>
[Progressable]: <{{ docs }}/widgets/progress/trait.Progressable.html>
[ProgressValue]: <{{ docs }}/widgets/progress/trait.ProgressValue.html>
[PercentBetween]: <{{ docs }}/animation/trait.PercentBetween.html>
[Ranged]: <https://docs.rs/figures/latest/figures/trait.Ranged.html>
