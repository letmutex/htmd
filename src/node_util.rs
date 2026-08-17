use std::rc::Rc;

use markup5ever_rcdom::{Node, NodeData};

pub(crate) fn get_node_tag_name(node: &Rc<Node>) -> Option<&str> {
    match &node.data {
        NodeData::Document => Some("html"),
        NodeData::Element { name, .. } => Some(&name.local),
        _ => None,
    }
}

pub(crate) fn get_parent_node(node: &Rc<Node>) -> Option<Rc<Node>> {
    let value = node.parent.take();
    let parent = value.as_ref().and_then(|parent| parent.upgrade());
    node.parent.set(value);
    parent
}

pub(crate) fn parent_tag_name_equals(node: &Rc<Node>, tag_names: &[&str]) -> bool {
    get_parent_node(node)
        .as_ref()
        .and_then(get_node_tag_name)
        .is_some_and(|tag| tag_names.contains(&tag))
}
