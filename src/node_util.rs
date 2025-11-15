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

// Check to see if node's parent's tag name matches the provided string.
pub(crate) fn parent_tag_name_equals(node: &Rc<Node>, tag_names: &Vec<&str>) -> bool {
    if let Some(parent) = get_parent_node(node)
        && let Some(actual_tag_name) = get_node_tag_name(&parent)
        && tag_names.contains(&actual_tag_name)
    {
        true
    } else {
        false
    }
}

pub(crate) fn get_node_children(node: &Rc<Node>) -> Vec<Rc<Node>> {
    let children = node.children.borrow();
    children.iter().cloned().collect()
}
