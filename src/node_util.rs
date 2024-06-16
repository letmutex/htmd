use std::rc::Rc;

use markup5ever_rcdom::{Node, NodeData};

pub(crate) fn get_node_tag_name(node: &Rc<Node>) -> Option<String> {
    match &node.data {
        NodeData::Document => Some("html".to_string()),
        NodeData::Element { name, .. } => Some(name.local.to_string()),
        _ => None,
    }
}

pub(crate) fn get_parent_node_tag_name(node: &Rc<Node>) -> Option<String> {
    let value = node.parent.take();
    let Some(parent) = value.as_ref() else {
        return None;
    };
    let Some(parent) = parent.upgrade() else {
        // Put the parent back
        node.parent.set(value);
        return None;
    };
    // Put the parent back
    node.parent.set(value);
    let tag = get_node_tag_name(&parent);
    tag
}
