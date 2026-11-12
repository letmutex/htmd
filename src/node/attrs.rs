#[cfg(feature = "rcdom")]
use std::cell::Ref;

pub(super) enum NodeAttrsInner<'a> {
    Slice(&'a [html5ever::Attribute]),
    #[cfg(feature = "rcdom")]
    Ref(Ref<'a, [html5ever::Attribute]>),
}

/// A borrowed view of an element [`Node`](super::Node)'s attributes.
pub struct NodeAttrs<'a>(pub(super) NodeAttrsInner<'a>);

impl NodeAttrs<'_> {
    #[inline]
    fn as_slice(&self) -> &[html5ever::Attribute] {
        match &self.0 {
            NodeAttrsInner::Slice(attrs) => attrs,
            #[cfg(feature = "rcdom")]
            NodeAttrsInner::Ref(attrs) => attrs,
        }
    }

    /// The number of attributes on the element, or `0` for non-elements.
    #[inline]
    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    /// Whether the element has no attributes (or the node is not an element).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    /// The value of the first attribute matching `name`, or `None` if absent.
    #[inline]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.as_slice()
            .iter()
            .find(|attr| &*attr.name.local == name)
            .map(|attr| &*attr.value)
    }

    /// Iterate lazily over the element's attributes as `(name, value)`.
    #[inline]
    pub fn iter(&self) -> NodeAttrsIter<'_> {
        NodeAttrsIter(self.as_slice().iter())
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
