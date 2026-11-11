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
    pub fn text(&self) -> Option<String> {
        match &self.0.data {
            markup5ever_rcdom::NodeData::Text { contents } => Some(contents.borrow().to_string()),
            _ => None,
        }
    }

    /// The value of the first attribute matching `name` if this node is an element,
    /// or `None` otherwise.
    #[inline]
    pub fn attr(&self, name: &str) -> Option<String> {
        match &self.0.data {
            markup5ever_rcdom::NodeData::Element { attrs, .. } => attrs
                .borrow()
                .iter()
                .find(|attr| &*attr.name.local == name)
                .map(|attr| attr.value.to_string()),
            _ => None,
        }
    }

    /// Iterate lazily over the element's attributes as `(name, value)`.
    #[inline]
    pub fn attrs(
        &self,
    ) -> impl ExactSizeIterator<Item = (String, String)> + DoubleEndedIterator + '_ {
        let attrs = match &self.0.data {
            markup5ever_rcdom::NodeData::Element { attrs, .. } => Some(attrs.borrow()),
            _ => None,
        };
        RefSliceMap::new(attrs, |attr| {
            (attr.name.local.to_string(), attr.value.to_string())
        })
    }

    /// The parent node, if any.
    #[inline]
    pub fn parent(&self) -> Option<Node> {
        crate::util::node::get_parent_node(&self.0).map(Node)
    }

    /// The first direct child node, if any.
    #[inline]
    pub fn first_child(&self) -> Option<Node> {
        self.0.children.borrow().first().cloned().map(Node)
    }

    /// Iterate lazily over the direct child nodes without allocating a `Vec`.
    #[inline]
    pub fn children(&self) -> impl ExactSizeIterator<Item = Node> + DoubleEndedIterator + '_ {
        RefSliceMap::new(Some(self.0.children.borrow()), |child| {
            Node(Rc::clone(child))
        })
    }

    /// Serialize this node and its descendants to HTML.
    pub fn html(&self) -> String {
        crate::element_handler::element_util::serialize_node(&self.0).unwrap_or_else(|e| {
            panic!("Failed to serialize node: {}", e);
        })
    }
}

struct RefSliceMap<'a, T, F> {
    items: Option<std::cell::Ref<'a, Vec<T>>>,
    front: usize,
    back: usize,
    map: F,
}

impl<'a, T, R, F> RefSliceMap<'a, T, F>
where
    F: FnMut(&T) -> R,
{
    #[inline]
    fn new(items: Option<std::cell::Ref<'a, Vec<T>>>, map: F) -> Self {
        let back = items.as_ref().map_or(0, |items| items.len());
        Self {
            items,
            front: 0,
            back,
            map,
        }
    }
}

impl<T, R, F> Iterator for RefSliceMap<'_, T, F>
where
    F: FnMut(&T) -> R,
{
    type Item = R;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let items = self.items.as_ref()?;
        if self.front < self.back {
            let item = (self.map)(&items[self.front]);
            self.front += 1;
            Some(item)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.back - self.front;
        (len, Some(len))
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        let items = self.items.as_ref()?;
        if n < self.back - self.front {
            self.front += n;
            let item = (self.map)(&items[self.front]);
            self.front += 1;
            Some(item)
        } else {
            self.front = self.back;
            None
        }
    }

    #[inline]
    fn count(self) -> usize {
        self.len()
    }

    #[inline]
    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }
}

impl<T, R, F> DoubleEndedIterator for RefSliceMap<'_, T, F>
where
    F: FnMut(&T) -> R,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        let items = self.items.as_ref()?;
        if self.front < self.back {
            self.back -= 1;
            Some((self.map)(&items[self.back]))
        } else {
            None
        }
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        let items = self.items.as_ref()?;
        if n < self.back - self.front {
            self.back -= n + 1;
            Some((self.map)(&items[self.back]))
        } else {
            self.back = self.front;
            None
        }
    }
}

impl<T, R, F> ExactSizeIterator for RefSliceMap<'_, T, F> where F: FnMut(&T) -> R {}
impl<T, R, F> std::iter::FusedIterator for RefSliceMap<'_, T, F> where F: FnMut(&T) -> R {}

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
