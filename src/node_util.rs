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
    let parent = value.as_ref()?;
    let Some(parent) = parent.upgrade() else {
        // Put the parent back
        node.parent.set(value);
        return None;
    };
    // Put the parent back
    node.parent.set(value);
    Some(parent)
}
