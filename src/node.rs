use std::cell::Ref;
use std::ops::{ControlFlow, Deref};
use std::rc::Rc;

/// The kind of a DOM [`Node`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeType {
    /// The root document node.
    Document,
    /// A `<!DOCTYPE ...>` declaration node.
    Doctype,
    /// A text node.
    Text,
    /// An HTML comment node.
    Comment,
    /// An HTML element node.
    Element,
    /// A processing instruction node.
    ProcessingInstruction,
}

/// A borrowed string view backed by a DOM [`Node`].
pub struct NodeStr<'a>(Ref<'a, str>);

impl Deref for NodeStr<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for NodeStr<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::borrow::Borrow<str> for NodeStr<'_> {
    #[inline]
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl std::hash::Hash for NodeStr<'_> {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (*self.0).hash(state);
    }
}

impl PartialOrd for NodeStr<'_> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NodeStr<'_> {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self.0).cmp(&*other.0)
    }
}

impl PartialEq for NodeStr<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}

impl Eq for NodeStr<'_> {}

impl PartialEq<str> for NodeStr<'_> {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<&str> for NodeStr<'_> {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        &*self.0 == *other
    }
}

impl PartialEq<String> for NodeStr<'_> {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        &*self.0 == other
    }
}

impl PartialEq<NodeStr<'_>> for str {
    #[inline]
    fn eq(&self, other: &NodeStr<'_>) -> bool {
        self == &*other.0
    }
}

impl PartialEq<NodeStr<'_>> for &str {
    #[inline]
    fn eq(&self, other: &NodeStr<'_>) -> bool {
        *self == &*other.0
    }
}

impl PartialEq<NodeStr<'_>> for String {
    #[inline]
    fn eq(&self, other: &NodeStr<'_>) -> bool {
        self == &*other.0
    }
}

impl std::fmt::Display for NodeStr<'_> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::fmt::Debug for NodeStr<'_> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&*self.0, f)
    }
}

/// A borrowed view of an element [`Node`]'s attributes.
pub struct NodeAttrs<'a>(Option<Ref<'a, [html5ever::Attribute]>>);

impl NodeAttrs<'_> {
    /// The number of attributes on the element, or `0` for non-elements.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.as_deref().map_or(0, <[html5ever::Attribute]>::len)
    }

    /// Whether the element has no attributes (or the node is not an element).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The value of the first attribute matching `name`, or `None` if absent.
    #[inline]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .as_deref()?
            .iter()
            .find(|attr| &*attr.name.local == name)
            .map(|attr| &*attr.value)
    }

    /// Iterate lazily over the element's attributes as `(name, value)`.
    #[inline]
    pub fn iter(&self) -> NodeAttrsIter<'_> {
        NodeAttrsIter(self.0.as_deref().unwrap_or(&[]).iter())
    }
}

impl<'a, 'b> IntoIterator for &'b NodeAttrs<'a> {
    type Item = (&'b str, &'b str);
    type IntoIter = NodeAttrsIter<'b>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl std::fmt::Debug for NodeAttrs<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// An iterator over `(name, value)` pairs borrowed from [`NodeAttrs`].
#[derive(Clone)]
pub struct NodeAttrsIter<'a>(std::slice::Iter<'a, html5ever::Attribute>);

#[inline]
fn attr_pair(attr: &html5ever::Attribute) -> (&str, &str) {
    (&*attr.name.local, &*attr.value)
}

impl<'a> Iterator for NodeAttrsIter<'a> {
    type Item = (&'a str, &'a str);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(attr_pair)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth(n).map(attr_pair)
    }

    #[inline]
    fn count(self) -> usize {
        self.0.len()
    }

    #[inline]
    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl DoubleEndedIterator for NodeAttrsIter<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(attr_pair)
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.0.nth_back(n).map(attr_pair)
    }
}

impl ExactSizeIterator for NodeAttrsIter<'_> {}
impl std::iter::FusedIterator for NodeAttrsIter<'_> {}

/// A borrowed slice of a DOM [`Node`]'s direct children.
pub struct NodeChildren<'a>(Ref<'a, [Node]>);

impl Deref for NodeChildren<'_> {
    type Target = [Node];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[Node]> for NodeChildren<'_> {
    #[inline]
    fn as_ref(&self) -> &[Node] {
        &self.0
    }
}

impl<'a, 'b> IntoIterator for &'b NodeChildren<'a> {
    type Item = &'b Node;
    type IntoIter = std::slice::Iter<'b, Node>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl std::fmt::Debug for NodeChildren<'_> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// A reference-counted handle to a DOM node.
///
/// Equality (`PartialEq`/`Eq`) and hashing (`Hash`) compare node handle identity
/// (`Rc::ptr_eq`).
#[repr(transparent)]
#[derive(Clone)]
pub struct Node(pub(crate) Rc<markup5ever_rcdom::Node>);

impl Node {
    #[inline]
    pub(crate) fn from_rc_ref(node: &Rc<markup5ever_rcdom::Node>) -> &Self {
        // SAFETY: `Node` is `#[repr(transparent)]` over `Rc<markup5ever_rcdom::Node>`.
        unsafe { &*std::ptr::from_ref(node).cast::<Node>() }
    }

    #[inline]
    pub(crate) fn as_rc(&self) -> &Rc<markup5ever_rcdom::Node> {
        &self.0
    }

    /// The kind of this DOM node.
    #[inline]
    pub fn node_type(&self) -> NodeType {
        match &self.0.data {
            markup5ever_rcdom::NodeData::Document => NodeType::Document,
            markup5ever_rcdom::NodeData::Doctype { .. } => NodeType::Doctype,
            markup5ever_rcdom::NodeData::Text { .. } => NodeType::Text,
            markup5ever_rcdom::NodeData::Comment { .. } => NodeType::Comment,
            markup5ever_rcdom::NodeData::Element { .. } => NodeType::Element,
            markup5ever_rcdom::NodeData::ProcessingInstruction { .. } => {
                NodeType::ProcessingInstruction
            }
        }
    }

    /// The element tag name if this node is an HTML element, or `None` otherwise.
    #[inline]
    pub fn tag_name(&self) -> Option<&str> {
        match &self.0.data {
            markup5ever_rcdom::NodeData::Element { name, .. } => Some(&name.local),
            _ => None,
        }
    }

    /// The text contents if this node is a text node, or `None` otherwise.
    #[inline]
    pub fn text(&self) -> Option<NodeStr<'_>> {
        match &self.0.data {
            markup5ever_rcdom::NodeData::Text { contents } => {
                Some(NodeStr(Ref::map(contents.borrow(), |text| &**text)))
            }
            _ => None,
        }
    }

    /// The value of the first attribute matching `name` if this node is an element,
    /// or `None` otherwise.
    #[inline]
    pub fn attr(&self, name: &str) -> Option<NodeStr<'_>> {
        match &self.0.data {
            markup5ever_rcdom::NodeData::Element { attrs, .. } => {
                Ref::filter_map(attrs.borrow(), |attrs| {
                    attrs
                        .iter()
                        .find(|attr| &*attr.name.local == name)
                        .map(|attr| &*attr.value)
                })
                .ok()
                .map(NodeStr)
            }
            _ => None,
        }
    }

    /// Borrow the element's attributes, or an empty view for non-elements.
    #[inline]
    pub fn attrs(&self) -> NodeAttrs<'_> {
        let attrs = match &self.0.data {
            markup5ever_rcdom::NodeData::Element { attrs, .. } => {
                Some(Ref::map(attrs.borrow(), Vec::as_slice))
            }
            _ => None,
        };
        NodeAttrs(attrs)
    }

    /// The parent node, if any.
    #[inline]
    pub fn parent(&self) -> Option<Node> {
        crate::util::node::get_parent_node(&self.0).map(Node)
    }

    /// The previous sibling node under the same parent, if any.
    #[inline]
    pub fn prev_sibling(&self) -> Option<Node> {
        self.sibling_by_offset(-1)
    }

    /// The next sibling node under the same parent, if any.
    #[inline]
    pub fn next_sibling(&self) -> Option<Node> {
        self.sibling_by_offset(1)
    }

    /// The first direct child node, if any.
    #[inline]
    pub fn first_child(&self) -> Option<Node> {
        self.0.children.borrow().first().cloned().map(Node)
    }

    /// Borrow the direct child nodes as a slice.
    #[inline]
    pub fn children(&self) -> NodeChildren<'_> {
        NodeChildren(Ref::map(self.0.children.borrow(), |children| {
            rc_slice_as_nodes(children)
        }))
    }

    /// Visit this node and its descendants in pre-order.
    pub fn walk(&self, mut visitor: impl FnMut(&Node)) {
        let _ = walk_rc(&self.0, &mut |node| {
            visitor(node);
            ControlFlow::<()>::Continue(())
        });
    }

    /// Return the first node in this subtree (including `self`) matching `predicate`.
    pub fn find(&self, mut predicate: impl FnMut(&Node) -> bool) -> Option<Node> {
        walk_rc(&self.0, &mut |node| {
            if predicate(node) {
                ControlFlow::Break(node.clone())
            } else {
                ControlFlow::Continue(())
            }
        })
        .break_value()
    }

    /// Serialize this node and its descendants to HTML.
    pub fn html(&self) -> String {
        crate::element_handler::element_util::serialize_node(&self.0).unwrap_or_else(|e| {
            panic!("Failed to serialize node: {}", e);
        })
    }

    fn sibling_by_offset(&self, offset: isize) -> Option<Node> {
        let parent = crate::util::node::get_parent_node(&self.0)?;
        let children = parent.children.borrow();
        let idx = children
            .iter()
            .position(|child| Rc::ptr_eq(child, &self.0))?;
        let target = idx.checked_add_signed(offset)?;
        children.get(target).cloned().map(Node)
    }
}

#[inline]
fn rc_slice_as_nodes(slice: &[Rc<markup5ever_rcdom::Node>]) -> &[Node] {
    // SAFETY: `Node` is `#[repr(transparent)]` over `Rc<markup5ever_rcdom::Node>`.
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<Node>(), slice.len()) }
}

fn walk_rc<T>(
    node: &Rc<markup5ever_rcdom::Node>,
    visitor: &mut impl FnMut(&Node) -> ControlFlow<T>,
) -> ControlFlow<T> {
    visitor(Node::from_rc_ref(node))?;
    for child in node.children.borrow().iter() {
        walk_rc(child, visitor)?;
    }
    ControlFlow::Continue(())
}

impl PartialEq for Node {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Node {}

impl std::hash::Hash for Node {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.0).hash(state);
    }
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0.data {
            markup5ever_rcdom::NodeData::Document => f.debug_tuple("Node::Document").finish(),
            markup5ever_rcdom::NodeData::Doctype { name, .. } => {
                let name: &str = name;
                f.debug_tuple("Node::Doctype").field(&name).finish()
            }
            markup5ever_rcdom::NodeData::Text { contents } => {
                let text = contents.borrow();
                let text: &str = &text;
                f.debug_tuple("Node::Text").field(&text).finish()
            }
            markup5ever_rcdom::NodeData::Comment { contents } => {
                let comment: &str = contents;
                f.debug_tuple("Node::Comment").field(&comment).finish()
            }
            markup5ever_rcdom::NodeData::Element { name, .. } => {
                let tag: &str = &name.local;
                f.debug_tuple("Node::Element").field(&tag).finish()
            }
            markup5ever_rcdom::NodeData::ProcessingInstruction { target, .. } => {
                let target: &str = target;
                f.debug_tuple("Node::ProcessingInstruction")
                    .field(&target)
                    .finish()
            }
        }
    }
}
