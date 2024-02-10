# Welcome to the Cushy User's Guide

This is a user's guide for [Cushy][cushy], a [Rust][rust] GUI crate. The
[documentation][docs] is a great resource for finding information about specific
functionality quickly. This guide is aimed to providing an example-rich
walkthrough of how to use and extend Cushy.

## A "Hello, World" Example

Here's the simplest "Hello, World" example:

```rust,no_run,no_playground
{{#include ../guide-examples/examples/hello-world.rs:example}}
```

When run, the app just displays the text as one would hope:

![Hello World Example](./examples/hello_world.png)

That was a little too easy. Let's take it a step further by letting a user type
in their name and have a label display "Hello, {name}!":

```rust,no_run,no_playground
{{#include ../guide-examples/examples/intro.rs:example}}
```

This app looks like this when executed:

![Hello Ferris Example](./examples/intro.png)

In this example, both `name` and `greeting` are [`Dynamic<String>`s][dynamic]. A
`Dynamic<T>` is an `Arc<Mutex<T>>`-like type that is able to invoke a set of
callbacks when its contents is changed. This simple feature is the core of
Cushy's reactive data model.

Each time `name` is changed, the `map_each` closure will be executed and
`greeting` will be updated with the result. Now that we have the individual
pieces of data our user interface is going to work with, we can start assembling
the interface.

First, we create `name_input` by converting the `Dynamic<String>` into a text
input widget ([`Input<String>`][input]). Since `Dynamic<String>` can be used as
a [`Label`][label], all that's left is laying out our two widgets.

To layout `name_input` and `greeting`, we use a [`Stack`][stack] to lay out the
widgets as rows.

Don't worry if this example seems a bit magical or confusing as to how it works.
Cushy can feel magical to use. But, it should never be a mystery. The goal of
this guide is to try and explain how and why Cushy works the way it does.

[cushy]: <https://github.com/khonsulabs/cushy>
[rust]: <https://rust-lang.org/>
[docs]: <{{docs}}>
[dynamic]: <{{docs}}/value/struct.Dynamic.html>
[input]: <{{docs}}/widgets/input/struct.Input.html>
[label]: <{{docs}}/widgets/label/struct.Label.html>
[stack]: <{{docs}}/widgets/stack/struct.Stack.html>
