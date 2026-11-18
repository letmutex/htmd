# Migration Guide

## v0.6.0

`v0.6.0` hides `markup5ever_rcdom` and `html5ever` from the default public API behind `htmd::node`, switches the default DOM backend to `ego-tree`, and passes `Element` by reference in handlers.

### Imports

| v0.5.x | v0.6.0 |
| --- | --- |
| `htmd::Node` (`markup5ever_rcdom::Node`) | `htmd::node::{Node, Tree}` (or `markup5ever_rcdom::Node` directly with the `rcdom` feature) |
| `htmd::Element` | `htmd::element_handler::Element` (`htmd::Element` is still re-exported) |
| — | `htmd::element_handler::Context` (when constructing a context for `Handlers::handle` / `walk_children`) |

If you pass an external `Rc<markup5ever_rcdom::Node>` into `htmd`, enable the `rcdom` feature (and depend on `markup5ever_rcdom = "0.40"`):

```toml
htmd = { version = "0.6", default-features = false, features = ["rcdom"] }
```

### Types

* **`htmd::Node` → `htmd::node::Tree` and `htmd::node::Node<'a>`**:
  DOM trees and nodes are now opaque wrappers instead of `Rc<markup5ever_rcdom::Node>`.
* **`Element<'a>`**:
  * `element.node` changed from `&'a Rc<markup5ever_rcdom::Node>` to `&'a htmd::node::Node<'a>`.
  * `element.attrs: &'a [Attribute]` was removed; read attributes via `element.node.attr("href")` or `element.node.attrs()`.
  * `element.markdown_translated: bool` was removed.
  * `element.context: Context` was added for forwarding into `Handlers::handle` and `Handlers::walk_children`.

### Functions

#### Conversion & Tree APIs

* **`HtmlToMarkdown::html_to_tree`** now returns `std::io::Result<Tree>` instead of `std::io::Result<Rc<Node>>`.
* **`HtmlToMarkdown::tree_to_markdown`** was removed. Pass `&Tree`, `&Node`, or (with `rcdom`) `&Rc<markup5ever_rcdom::Node>` to `convert` instead (returns `std::io::Result<String>`):

```rust
// v0.5.x
let tree = converter.html_to_tree(html)?;
let md = converter.tree_to_markdown(&tree);

// v0.6.0
let tree = converter.html_to_tree(html)?;
let md = converter.convert(&tree)?;
```

* **`htmd::convert` and `HtmlToMarkdown::convert`** are now generic over `ConvertInput` instead of taking `&str`. Existing `&str` / `&String` calls continue to work, but calls relying on target-typed inference (such as `convert(x.as_ref())` or `fn(&str)` coercion) need an explicit type or `&str` conversion.

#### Element Handlers & `Handlers` Trait

* **`ElementHandler::handle`** (and closures passed to `HtmlToMarkdownBuilder::add_handler`) now take `element: &Element` instead of `element: Element`.
* **`Handlers::fallback`** now takes `element: &Element` instead of `element: Element`.
* **`Handlers::handle`** and **`Handlers::walk_children`** now take `(node: &Node<'_>, context: Context)` instead of `(node: &Rc<Node>)`.

```rust
// v0.5.x
.add_handler(vec!["div"], |handlers: &dyn Handlers, element: Element| {
    let has_class = element.attrs.iter().any(|a| &*a.name.local == "class");
    if has_class {
        return handlers.fallback(element);
    }
    Some(handlers.walk_children(element.node))
})

// v0.6.0
.add_handler(vec!["div"], |handlers: &dyn Handlers, element: &Element| {
    if element.node.attr("class").is_some() {
        return handlers.fallback(element);
    }
    Some(handlers.walk_children(element.node, element.context))
})
```
