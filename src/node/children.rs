#[cfg(feature = "rcdom")]
use std::cell::Ref;
#[cfg(feature = "rcdom")]
use std::rc::Rc;

use crate::dom::SyntheticTextElement;
#[cfg(feature = "ego-tree")]
use crate::dom::ego_tree::EgoNodeData;

use super::{Node, NodeInner};

pub(super) enum NodeChildrenInner<'a> {
    Empty,
    #[cfg(feature = "ego-tree")]
    Ego(ego_tree::NodeRef<'a, EgoNodeData>),
    #[cfg(feature = "rcdom")]
    Rc(Ref<'a, [Rc<markup5ever_rcdom::Node>]>),
    Synthetic(&'a SyntheticTextElement<'a>),
}

/// A borrowed view of a DOM [`Node`]'s direct children.
pub struct NodeChildren<'a>(pub(super) NodeChildrenInner<'a>);

impl NodeChildren<'_> {
    /// The number of direct children.
    ///
    /// Traverses the child list in linear time under the default `ego-tree`
    /// backend; prefer [`iter`](Self::iter) or [`is_empty`](Self::is_empty)
    /// when iterating or checking emptiness.
    #[inline]
    pub fn len(&self) -> usize {
        match &self.0 {
            NodeChildrenInner::Empty => 0,
            #[cfg(feature = "ego-tree")]
            NodeChildrenInner::Ego(parent) => parent.children().count(),
            #[cfg(feature = "rcdom")]
            NodeChildrenInner::Rc(slice) => slice.len(),
            NodeChildrenInner::Synthetic(_) => 1,
        }
    }

    /// Whether the node has no direct children.
    #[inline]
    pub fn is_empty(&self) -> bool {
        match &self.0 {
            NodeChildrenInner::Empty => true,
            #[cfg(feature = "ego-tree")]
            NodeChildrenInner::Ego(parent) => !parent.has_children(),
            #[cfg(feature = "rcdom")]
            NodeChildrenInner::Rc(slice) => slice.is_empty(),
            NodeChildrenInner::Synthetic(_) => false,
        }
    }

    /// The first direct child, if any.
    #[inline]
    pub fn first(&self) -> Option<Node<'_>> {
        match &self.0 {
            NodeChildrenInner::Empty => None,
            #[cfg(feature = "ego-tree")]
            NodeChildrenInner::Ego(parent) => parent.first_child().map(Node::from_ego_ref),
            #[cfg(feature = "rcdom")]
            NodeChildrenInner::Rc(slice) => slice.first().map(Node::from_rc_borrowed),
            NodeChildrenInner::Synthetic(synth) => Some(Node(NodeInner::SyntheticText(synth))),
        }
    }

    /// The last direct child, if any.
    #[inline]
    pub fn last(&self) -> Option<Node<'_>> {
        match &self.0 {
            NodeChildrenInner::Empty => None,
            #[cfg(feature = "ego-tree")]
            NodeChildrenInner::Ego(parent) => parent.last_child().map(Node::from_ego_ref),
            #[cfg(feature = "rcdom")]
            NodeChildrenInner::Rc(slice) => slice.last().map(Node::from_rc_borrowed),
            NodeChildrenInner::Synthetic(synth) => Some(Node(NodeInner::SyntheticText(synth))),
        }
    }

    /// The direct child at `index`, or `None` if out of bounds.
    ///
    /// Traverses the child list linearly under the default `ego-tree` backend;
    /// prefer [`iter`](Self::iter) for sequential access.
    #[inline]
    pub fn get(&self, index: usize) -> Option<Node<'_>> {
        match &self.0 {
            NodeChildrenInner::Empty => None,
            #[cfg(feature = "ego-tree")]
            NodeChildrenInner::Ego(parent) => parent.children().nth(index).map(Node::from_ego_ref),
            #[cfg(feature = "rcdom")]
            NodeChildrenInner::Rc(slice) => slice.get(index).map(Node::from_rc_borrowed),
            NodeChildrenInner::Synthetic(synth) => {
                (index == 0).then_some(Node(NodeInner::SyntheticText(synth)))
            }
        }
    }

    /// Iterate over the direct children in document order.
    #[inline]
    pub fn iter(&self) -> NodeChildrenIter<'_> {
        let inner = match &self.0 {
            NodeChildrenInner::Empty => NodeChildrenIterInner::Empty,
            #[cfg(feature = "ego-tree")]
            NodeChildrenInner::Ego(parent) => NodeChildrenIterInner::Ego {
                front: parent.first_child(),
                back: parent.last_child(),
            },
            #[cfg(feature = "rcdom")]
            NodeChildrenInner::Rc(slice) => NodeChildrenIterInner::Rc(slice.iter()),
            NodeChildrenInner::Synthetic(synth) => NodeChildrenIterInner::Synthetic(Some(synth)),
        };
        NodeChildrenIter(inner)
    }
}

impl<'a, 'b> IntoIterator for &'b NodeChildren<'a> {
    type Item = Node<'b>;
    type IntoIter = NodeChildrenIter<'b>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl std::fmt::Debug for NodeChildren<'_> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[derive(Clone)]
enum NodeChildrenIterInner<'a> {
    Empty,
    #[cfg(feature = "ego-tree")]
    Ego {
        front: Option<ego_tree::NodeRef<'a, EgoNodeData>>,
        back: Option<ego_tree::NodeRef<'a, EgoNodeData>>,
    },
    #[cfg(feature = "rcdom")]
    Rc(std::slice::Iter<'a, Rc<markup5ever_rcdom::Node>>),
    Synthetic(Option<&'a SyntheticTextElement<'a>>),
}

/// An iterator over the direct children of a [`Node`].
#[derive(Clone)]
pub struct NodeChildrenIter<'a>(NodeChildrenIterInner<'a>);

impl<'a> Iterator for NodeChildrenIter<'a> {
    type Item = Node<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            NodeChildrenIterInner::Empty => None,
            #[cfg(feature = "ego-tree")]
            NodeChildrenIterInner::Ego { front, back } => {
                let current = (*front)?;
                if back.is_some_and(|b| b == current) {
                    *front = None;
                    *back = None;
                } else {
                    *front = current.next_sibling();
                }
                Some(Node::from_ego_ref(current))
            }
            #[cfg(feature = "rcdom")]
            NodeChildrenIterInner::Rc(iter) => iter.next().map(Node::from_rc_borrowed),
            NodeChildrenIterInner::Synthetic(synth) => {
                synth.take().map(|s| Node(NodeInner::SyntheticText(s)))
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.0 {
            NodeChildrenIterInner::Empty => (0, Some(0)),
            #[cfg(feature = "ego-tree")]
            NodeChildrenIterInner::Ego { front, .. } => {
                if front.is_some() {
                    (1, None)
                } else {
                    (0, Some(0))
                }
            }
            #[cfg(feature = "rcdom")]
            NodeChildrenIterInner::Rc(iter) => iter.size_hint(),
            NodeChildrenIterInner::Synthetic(synth) => {
                let len = usize::from(synth.is_some());
                (len, Some(len))
            }
        }
    }

    #[inline]
    fn count(self) -> usize {
        match self.0 {
            NodeChildrenIterInner::Empty => 0,
            #[cfg(feature = "ego-tree")]
            NodeChildrenIterInner::Ego { .. } => self.fold(0, |acc, _| acc + 1),
            #[cfg(feature = "rcdom")]
            NodeChildrenIterInner::Rc(iter) => iter.count(),
            NodeChildrenIterInner::Synthetic(synth) => usize::from(synth.is_some()),
        }
    }

    #[inline]
    fn last(mut self) -> Option<Self::Item> {
        self.next_back()
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        match &mut self.0 {
            NodeChildrenIterInner::Empty => None,
            #[cfg(feature = "ego-tree")]
            NodeChildrenIterInner::Ego { .. } => {
                for _ in 0..n {
                    self.next()?;
                }
                self.next()
            }
            #[cfg(feature = "rcdom")]
            NodeChildrenIterInner::Rc(iter) => iter.nth(n).map(Node::from_rc_borrowed),
            NodeChildrenIterInner::Synthetic(synth) => {
                let s = synth.take()?;
                (n == 0).then_some(Node(NodeInner::SyntheticText(s)))
            }
        }
    }
}

impl DoubleEndedIterator for NodeChildrenIter<'_> {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            NodeChildrenIterInner::Empty => None,
            #[cfg(feature = "ego-tree")]
            NodeChildrenIterInner::Ego { front, back } => {
                let current = (*back)?;
                if front.is_some_and(|f| f == current) {
                    *front = None;
                    *back = None;
                } else {
                    *back = current.prev_sibling();
                }
                Some(Node::from_ego_ref(current))
            }
            #[cfg(feature = "rcdom")]
            NodeChildrenIterInner::Rc(iter) => iter.next_back().map(Node::from_rc_borrowed),
            NodeChildrenIterInner::Synthetic(synth) => {
                synth.take().map(|s| Node(NodeInner::SyntheticText(s)))
            }
        }
    }

    #[inline]
    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        match &mut self.0 {
            NodeChildrenIterInner::Empty => None,
            #[cfg(feature = "ego-tree")]
            NodeChildrenIterInner::Ego { .. } => {
                for _ in 0..n {
                    self.next_back()?;
                }
                self.next_back()
            }
            #[cfg(feature = "rcdom")]
            NodeChildrenIterInner::Rc(iter) => iter.nth_back(n).map(Node::from_rc_borrowed),
            NodeChildrenIterInner::Synthetic(synth) => {
                let s = synth.take()?;
                (n == 0).then_some(Node(NodeInner::SyntheticText(s)))
            }
        }
    }
}

impl std::iter::FusedIterator for NodeChildrenIter<'_> {}
