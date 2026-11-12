#[cfg(feature = "ego-tree")]
pub(crate) mod ego_tree;
#[cfg(feature = "rcdom")]
pub(crate) mod rcdom;

use std::io;

use html5ever::serialize::TraversalScope;
use html5ever::{Attribute, QualName};

use crate::{Node, NodeType};

#[derive(Clone)]
pub(crate) struct SyntheticTextElement<'a> {
    pub(crate) parent: Option<Node<'a>>,
    pub(crate) tag: &'a str,
    pub(crate) qual_name: &'a QualName,
    pub(crate) attrs: &'a [Attribute],
    pub(crate) text: &'a str,
}

pub(crate) trait ChildCursor<N>: Sized {
    fn next(&mut self) -> Option<N>;
    fn peek(&self) -> Option<N>;
}

pub(crate) trait DomNode: Copy {
    type Child<'c>: DomNode;
    type Cursor<'c>: ChildCursor<Self::Child<'c>>;

    fn node_type(self) -> NodeType;
    fn tag_name(&self) -> Option<&str>;
    fn as_node(&self) -> Node<'_>;
    fn with_text<R>(self, f: impl FnOnce(&str) -> R) -> R;
    fn with_comment<R>(self, f: impl FnOnce(&str) -> R) -> R;
    fn with_element<R>(self, f: impl FnOnce(&str, &QualName, &[Attribute]) -> R) -> R;
    fn with_children_cursor<R>(&self, f: impl for<'c> FnOnce(Self::Cursor<'c>) -> R) -> R;
    fn can_combine_with(self, other: Self) -> bool;
    fn push_single_text(self, out: &mut String);
    fn serialize_html(self, traversal_scope: TraversalScope) -> io::Result<String>;
}

pub(crate) fn combining_tag(tag: &str) -> &str {
    match tag {
        "i" => "em",
        "b" => "strong",
        _ => tag,
    }
}
