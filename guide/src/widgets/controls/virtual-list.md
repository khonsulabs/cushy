# Virtual List

The [VirtualList] widget allows efficient rendering of long lists of items.
It currently only supports the simplest form - a known width, equal heights of each item and a known item count, though all of the values are reactive, as is the custom in cushy.

You can create a widget by implementing the [VirtualListContent] trait:

```rust,no_run,no_playground
{{#include ../../../guide-examples/examples/virtual-list.rs:implementation}}
```

And then using it as any other widget in your functions

```rust,no_run,no_playground
{{#include ../../../guide-examples/examples/virtual-list.rs:list}}
```

[VirtualList]: <{{ docs }}/widgets/virtual_list/struct.VirtualList.html>
[VirtualListContent]: <{{ docs }}/widgets/virtual_list/struct.VirtualListContent.html>
