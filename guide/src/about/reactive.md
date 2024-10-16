# Reactive Data Model

Cushy is designed around a reactive data model. In many UI frameworks, setting
the text of a [`Label`][label] is done by telling the label what it's new text
is. The Cushy way is to create a `Dynamic<T>` containing the value you want to
display, and give a copy to the label and keep a copy for your application to
update as needed.

When a widget uses a `Dynamic<T>`, it informs Cushy that it needs to be either
redrawn or invalidated when its contents change. In turn, when Cushy detects a
change, it invalidates what is necessary.

This means that updating a progress bar from a different thread is as simple as
passing a clone of a `Dynamic<T>` that has been provided to a progress bar:

```rust,no_run,no_playground
{{#include ../../guide-examples/examples/thread-progress.rs:example}}
```

The above snippet produces this user interface:

![Threaded Progress Bar Update](../examples/thread_progress.png)

This example just shows one simple way that Cushy's reactive model simplifies
development. To learn more, let's dive into the data types and traits that power
everything.

## How widgets interact with data

In Cushy, it's conventional for widgets to accept data using one of these
traits:

| Trait | Target Type | Purpose |
|-------|-------------|---------|
| [`IntoValue<T>`][intovalue] | [`Value<T>`][value] | For possibly constant or dynamic values. Can be either a `T` or a `Dynamic<T>`. |
| [`IntoDynamic<T>`][intodynamic] | [`Dynamic<T>`][dynamic] | For values that are read from and written to. |
| [`IntoReadOnly<T>`][intoreadonly] | [`ReadOnly<T>`][readonly] | For values that are read-only. Can be either a `T` or a `DynamicReader<T>`. |
| [`IntoDynamicReader<T>`][intodynamicreader] | [`DynamicReader<T>`][dynamicreader] | For values that are read-only, but are unexpected to be constant. In general, `IntoValue<T>` should be preferred if a single value makes sense to accept. |

Let's look at an example of how these traits are utilizes.
[`Label::new()`][label-new] accepts the value it is to display as a
`ReadOnly<T>` where `T: Display + ...`.

This showcases Cushy's philosophy of embracing the Rust type system. Rather than
forcing `Label` to receive a `String`, it accepts any type that implements
`Display`, This allows it to accept a wide variety of types.

Beyond basic values, it can also be given a special type that the `Label` can
react to when updated: a `Dynamic<T>` or a `DynamicReader<T>`.

## What is a `Dynamic<T>`?

A [`Dynamic<T>`][dynamic] is a reference-counted, threadsafe, async-friendly
location in memory that can invoke a series of callbacks when its contents
change. Let's revisit the example from the [intro](../intro.md):

```rust,no_run,no_playground
{{#include ../../guide-examples/examples/intro.rs:example}}
```

![Hello Ferris Example](../examples/intro.png)

Both the [`Input`][input] and the [`Label`][label] widgets have been given
instances of `Dynamic<String>`s, but they are two different dynamics. The text
input field was given the dynamic we want to be edited. We react to the changes
through the `name.map_each(...)` callback. You can react to multiple `Dynamic`s
at once using `(&name, &surname).map_each(...)` callback.

## What is a `DynamicReader<T>`?

A [`DynamicReader<T>`][dynamicreader] provides read-only access to a
`Dynamic<T>`, and also can:

- [block][block] the current thread until the underlying `Dynamic<T>` is changed.
- [wait][wait] for a change in an async task.
- Detect when the underlying `Dynamic<T>` has had all of its instances dropped.

`DynamicReader<T>`s can be created using [`Dynamic::into_reader`][into-reader]/[`Dynamic::create_reader`][create-reader].

[value]: <{{ docs }}/value/enum.Value.html>
[readonly]: <{{ docs }}/value/enum.ReadOnly.html>
[dynamic]: <{{ docs }}/value/struct.Dynamic.html>
[into-reader]: <{{ docs }}/value/struct.Dynamic.html#method.into_reader>
[create-reader]: <{{ docs }}/value/struct.Dynamic.html#method.create_reader>
[dynamicreader]: <{{ docs }}/value/struct.DynamicReader.html>
[block]: <{{ docs }}/value/struct.DynamicReader.html#method.block_until_updated>
[wait]: <{{ docs }}/value/struct.DynamicReader.html#method.wait_until_updated>
[intovalue]: <{{ docs }}/value/trait.IntoValue.html>
[intodynamic]: <{{ docs }}/value/trait.IntoDynamic.html>
[intoreadonly]: <{{ docs }}/value/trait.IntoReadOnly.html>
[intodynamicreader]: <{{ docs }}/value/trait.IntoDynamicReader.html>
[label-new]: <{{ docs }}/widgets/label/struct.Label.html#method.new>
[label]: <{{ docs }}/widgets/label/struct.Label.html>
[input]: <{{ docs }}/widgets/input/struct.Input.html>
