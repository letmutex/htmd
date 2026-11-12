mod attrs;
mod children;
mod serialize;
mod text;
mod tree;

#[cfg(feature = "rcdom")]
use std::cell::Ref;
#[cfg(feature = "rcdom")]
use std::ops::ControlFlow;
#[cfg(feature = "rcdom")]
use std::rc::Rc;

use html5ever::QualName;
use html5ever::serialize::TraversalScope;

#[cfg(feature = "ego-tree")]
use crate::dom::ego_tree::EgoNodeData;
use crate::dom::{DomNode, SyntheticTextElement};

use attrs::NodeAttrsInner;
use children::NodeChildrenInner;
use text::NodeStrInner;

pub use attrs::{NodeAttrs, NodeAttrsIter};
pub use children::{NodeChildren, NodeChildrenIter};
pub use text::NodeStr;
pub use tree::Tree;
pub(crate) use tree::TreeInner;

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

#[cfg(feature = "rcdom")]
#[derive(Clone)]
pub(crate) enum RcNode<'a> {
    Borrowed(&'a Rc<markup5ever_rcdom::Node>),
    Owned(Rc<markup5ever_rcdom::Node>),
}

#[cfg(feature = "rcdom")]
impl RcNode<'_> {
    #[inline]
    pub(crate) fn as_rc(&self) -> &Rc<markup5ever_rcdom::Node> {
        match self {
            RcNode::Borrowed(node) => node,
            RcNode::Owned(node) => node,
        }
    }
}

#[derive(Clone)]
pub(crate) enum NodeInner<'a> {
    #[cfg(feature = "ego-tree")]
    Ego(ego_tree::NodeRef<'a, EgoNodeData>),
    #[cfg(feature = "rcdom")]
    Rc(RcNode<'a>),
    SyntheticElement(&'a SyntheticTextElement<'a>),
    SyntheticText(&'a SyntheticTextElement<'a>),
}

/// A lightweight handle to a node in a DOM [`Tree`].
///
/// Equality (`PartialEq`/`Eq`) and hashing (`Hash`) compare node identity.
#[derive(Clone)]
pub struct Node<'a>(pub(crate) NodeInner<'a>);

impl<'a> Node<'a> {
    #[cfg(feature = "ego-tree")]
    #[inline]
    pub(crate) fn from_ego_ref(node: ego_tree::NodeRef<'a, EgoNodeData>) -> Self {
        Self(NodeInner::Ego(node))
    }

    #[cfg(feature = "rcdom")]
    #[inline]
    pub(crate) fn from_rc_borrowed(node: &'a Rc<markup5ever_rcdom::Node>) -> Self {
        Self(NodeInner::Rc(RcNode::Borrowed(node)))
    }

    #[cfg(feature = "rcdom")]
    #[inline]
    pub(crate) fn from_rc_owned(node: Rc<markup5ever_rcdom::Node>) -> Self {
        Self(NodeInner::Rc(RcNode::Owned(node)))
    }

    #[inline]
    pub(crate) fn from_synthetic_element(synth: &'a SyntheticTextElement<'a>) -> Self {
        Self(NodeInner::SyntheticElement(synth))
    }

    /// The kind of this DOM node.
    #[inline]
    pub fn node_type(&self) -> NodeType {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => DomNode::node_type(*node),
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => DomNode::node_type(node.as_rc()),
            NodeInner::SyntheticElement(_) => NodeType::Element,
            NodeInner::SyntheticText(_) => NodeType::Text,
        }
    }

    /// The element tag name if this node is an HTML element, or `None` otherwise.
    #[inline]
    pub fn tag_name(&self) -> Option<&str> {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => match node.value() {
                EgoNodeData::Element(elem) => Some(&elem.name.local),
                _ => None,
            },
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => match &node.as_rc().data {
                markup5ever_rcdom::NodeData::Element { name, .. } => Some(&name.local),
                _ => None,
            },
            NodeInner::SyntheticElement(synth) => Some(synth.tag),
            NodeInner::SyntheticText(_) => None,
        }
    }

    /// The text contents if this node is a text node, or `None` otherwise.
    #[inline]
    pub fn text(&self) -> Option<NodeStr<'_>> {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => match node.value() {
                EgoNodeData::Text(contents) => Some(NodeStr(NodeStrInner::Borrowed(contents))),
                _ => None,
            },
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => match &node.as_rc().data {
                markup5ever_rcdom::NodeData::Text { contents } => Some(NodeStr(NodeStrInner::Ref(
                    Ref::map(contents.borrow(), |text| &**text),
                ))),
                _ => None,
            },
            NodeInner::SyntheticElement(_) => None,
            NodeInner::SyntheticText(synth) => Some(NodeStr(NodeStrInner::Borrowed(synth.text))),
        }
    }

    /// The value of the first attribute matching `name` if this node is an element,
    /// or `None` otherwise.
    #[inline]
    pub fn attr(&self, name: &str) -> Option<NodeStr<'_>> {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => match node.value() {
                EgoNodeData::Element(elem) => elem
                    .attrs
                    .iter()
                    .find(|attr| &*attr.name.local == name)
                    .map(|attr| NodeStr(NodeStrInner::Borrowed(&attr.value))),
                _ => None,
            },
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => match &node.as_rc().data {
                markup5ever_rcdom::NodeData::Element { attrs, .. } => {
                    Ref::filter_map(attrs.borrow(), |attrs| {
                        attrs
                            .iter()
                            .find(|attr| &*attr.name.local == name)
                            .map(|attr| &*attr.value)
                    })
                    .ok()
                    .map(|r| NodeStr(NodeStrInner::Ref(r)))
                }
                _ => None,
            },
            NodeInner::SyntheticElement(synth) => synth
                .attrs
                .iter()
                .find(|attr| &*attr.name.local == name)
                .map(|attr| NodeStr(NodeStrInner::Borrowed(&attr.value))),
            NodeInner::SyntheticText(_) => None,
        }
    }

    /// Borrow the element's attributes, or an empty view for non-elements.
    #[inline]
    pub fn attrs(&self) -> NodeAttrs<'_> {
        let inner = match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => match node.value() {
                EgoNodeData::Element(elem) => NodeAttrsInner::Slice(&elem.attrs),
                _ => NodeAttrsInner::Slice(&[]),
            },
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => match &node.as_rc().data {
                markup5ever_rcdom::NodeData::Element { attrs, .. } => {
                    NodeAttrsInner::Ref(Ref::map(attrs.borrow(), Vec::as_slice))
                }
                _ => NodeAttrsInner::Slice(&[]),
            },
            NodeInner::SyntheticElement(synth) => NodeAttrsInner::Slice(synth.attrs),
            NodeInner::SyntheticText(_) => NodeAttrsInner::Slice(&[]),
        };
        NodeAttrs(inner)
    }

    /// The parent node, if any.
    #[inline]
    pub fn parent(&self) -> Option<Node<'a>> {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => node.parent().map(Node::from_ego_ref),
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => get_rc_parent(node.as_rc()).map(Node::from_rc_owned),
            NodeInner::SyntheticElement(synth) => synth.parent.clone(),
            NodeInner::SyntheticText(synth) => Some(Node(NodeInner::SyntheticElement(synth))),
        }
    }

    /// The first direct child node, if any.
    #[inline]
    pub fn first_child(&self) -> Option<Node<'a>> {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => node.first_child().map(Node::from_ego_ref),
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => node
                .as_rc()
                .children
                .borrow()
                .first()
                .cloned()
                .map(Node::from_rc_owned),
            NodeInner::SyntheticElement(synth) => Some(Node(NodeInner::SyntheticText(synth))),
            NodeInner::SyntheticText(_) => None,
        }
    }

    /// Borrow a view of the direct child nodes.
    #[inline]
    pub fn children(&self) -> NodeChildren<'_> {
        let inner = match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => NodeChildrenInner::Ego(*node),
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => {
                NodeChildrenInner::Rc(Ref::map(node.as_rc().children.borrow(), Vec::as_slice))
            }
            NodeInner::SyntheticElement(synth) => NodeChildrenInner::Synthetic(synth),
            NodeInner::SyntheticText(_) => NodeChildrenInner::Empty,
        };
        NodeChildren(inner)
    }

    /// Return the first node in this subtree (including `self`) matching `predicate`.
    pub fn find(&self, mut predicate: impl FnMut(&Node<'_>) -> bool) -> Option<Node<'a>> {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => node
                .descendants()
                .map(Node::from_ego_ref)
                .find(|descendant| predicate(descendant)),
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => {
                if predicate(self) {
                    return Some(self.clone());
                }
                find_rc_children(node.as_rc(), &mut predicate).break_value()
            }
            NodeInner::SyntheticElement(synth) => {
                if predicate(self) {
                    return Some(self.clone());
                }
                let text = Node(NodeInner::SyntheticText(synth));
                predicate(&text).then_some(text)
            }
            NodeInner::SyntheticText(_) => predicate(self).then(|| self.clone()),
        }
    }

    /// Serialize this node and its descendants to HTML.
    pub fn html(&self) -> String {
        self.serialize_html(TraversalScope::IncludeNode)
            .unwrap_or_else(|e| panic!("Failed to serialize node: {}", e))
    }

    #[inline]
    pub(crate) fn comment(&self) -> Option<&str> {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => match node.value() {
                EgoNodeData::Comment(contents) => Some(contents),
                _ => None,
            },
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => match &node.as_rc().data {
                markup5ever_rcdom::NodeData::Comment { contents } => Some(contents),
                _ => None,
            },
            NodeInner::SyntheticElement(_) | NodeInner::SyntheticText(_) => None,
        }
    }

    #[inline]
    pub(crate) fn qual_name(&self) -> Option<&QualName> {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => match node.value() {
                EgoNodeData::Element(elem) => Some(&elem.name),
                _ => None,
            },
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => match &node.as_rc().data {
                markup5ever_rcdom::NodeData::Element { name, .. } => Some(name),
                _ => None,
            },
            NodeInner::SyntheticElement(synth) => Some(synth.qual_name),
            NodeInner::SyntheticText(_) => None,
        }
    }
}

#[cfg(feature = "rcdom")]
#[inline]
fn get_rc_parent(node: &Rc<markup5ever_rcdom::Node>) -> Option<Rc<markup5ever_rcdom::Node>> {
    let value = node.parent.take();
    let parent = value.as_ref().and_then(|parent| parent.upgrade());
    node.parent.set(value);
    parent
}

#[cfg(feature = "rcdom")]
fn find_rc_children<'a>(
    node: &Rc<markup5ever_rcdom::Node>,
    predicate: &mut impl FnMut(&Node<'_>) -> bool,
) -> ControlFlow<Node<'a>> {
    for child in node.children.borrow().iter() {
        let wrapped = Node::from_rc_borrowed(child);
        if predicate(&wrapped) {
            return ControlFlow::Break(Node::from_rc_owned(Rc::clone(child)));
        }
        find_rc_children(child, predicate)?;
    }
    ControlFlow::Continue(())
}

impl PartialEq for Node<'_> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            #[cfg(feature = "ego-tree")]
            (NodeInner::Ego(a), NodeInner::Ego(b)) => a == b,
            #[cfg(feature = "rcdom")]
            (NodeInner::Rc(a), NodeInner::Rc(b)) => Rc::ptr_eq(a.as_rc(), b.as_rc()),
            (NodeInner::SyntheticElement(a), NodeInner::SyntheticElement(b))
            | (NodeInner::SyntheticText(a), NodeInner::SyntheticText(b)) => std::ptr::eq(*a, *b),
            _ => false,
        }
    }
}

impl Eq for Node<'_> {}

impl std::hash::Hash for Node<'_> {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => {
                0u8.hash(state);
                std::ptr::from_ref(node.tree()).hash(state);
                node.id().hash(state);
            }
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => {
                1u8.hash(state);
                Rc::as_ptr(node.as_rc()).hash(state);
            }
            NodeInner::SyntheticElement(synth) => {
                2u8.hash(state);
                std::ptr::from_ref(*synth).hash(state);
            }
            NodeInner::SyntheticText(synth) => {
                3u8.hash(state);
                std::ptr::from_ref(*synth).hash(state);
            }
        }
    }
}

impl std::fmt::Debug for Node<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => match node.value() {
                EgoNodeData::Document | EgoNodeData::DocumentFragment => {
                    f.debug_tuple("Node::Document").finish()
                }
                EgoNodeData::Doctype { name } => {
                    let name: &str = name;
                    f.debug_tuple("Node::Doctype").field(&name).finish()
                }
                EgoNodeData::Text(contents) => {
                    let text: &str = contents;
                    f.debug_tuple("Node::Text").field(&text).finish()
                }
                EgoNodeData::Comment(contents) => {
                    let comment: &str = contents;
                    f.debug_tuple("Node::Comment").field(&comment).finish()
                }
                EgoNodeData::Element(elem) => {
                    let tag: &str = &elem.name.local;
                    f.debug_tuple("Node::Element").field(&tag).finish()
                }
                EgoNodeData::ProcessingInstruction { target, .. } => {
                    let target: &str = target;
                    f.debug_tuple("Node::ProcessingInstruction")
                        .field(&target)
                        .finish()
                }
            },
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => match &node.as_rc().data {
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
            },
            NodeInner::SyntheticElement(synth) => {
                f.debug_tuple("Node::Element").field(&synth.tag).finish()
            }
            NodeInner::SyntheticText(synth) => {
                f.debug_tuple("Node::Text").field(&synth.text).finish()
            }
        }
    }
}
