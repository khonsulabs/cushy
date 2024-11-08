# Virtual List

The [`VirtualList`][VirtualList] widget allows efficient rendering of long lists of items. It
is restricted to uniform row width and heights to be very efficient.

For a virtual list to be rendered, it needs to be given an item count and a
function that creates a widget for a given item index.

```rust,no_run,no_playground
{{#include ../../../guide-examples/examples/virtual-list.rs:list}}
```

With this information, the `VirtualList` will only keep exactly the widgets
needed to display the currently visible rows. The item count can be a
`Dynamic<usize>` to allow changing the item count while the list is being
displayed. Additionally, [`content_watcher()`][content_watcher] allows fully
refreshing the contents when a `Source` changes or through manual notification.

[VirtualList]: <{{ docs }}/widgets/virtual_list/struct.VirtualList.html>
[content_watcher]: <{{ docs }}/widgets/struct.VirtualList.html#method.content_watcher>