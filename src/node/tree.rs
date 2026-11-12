#[cfg(feature = "ego-tree")]
use crate::dom::ego_tree::EgoNodeData;

use super::Node;

pub(crate) enum TreeInner {
    #[cfg(feature = "ego-tree")]
    Ego(ego_tree::Tree<EgoNodeData>),
    #[cfg(feature = "rcdom")]
    #[cfg_attr(feature = "ego-tree", allow(dead_code))]
    Rc(std::rc::Rc<markup5ever_rcdom::Node>),
}

/// An owned, parsed HTML DOM tree.
pub struct Tree(pub(crate) TreeInner);

impl Tree {
    /// Return a handle to the root node of this DOM tree.
    #[inline]
    pub fn root(&self) -> Node<'_> {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            TreeInner::Ego(tree) => Node::from_ego_ref(tree.root()),
            #[cfg(feature = "rcdom")]
            TreeInner::Rc(node) => Node::from_rc_borrowed(node),
        }
    }
}

impl std::fmt::Debug for Tree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Tree").field(&self.root()).finish()
    }
}
