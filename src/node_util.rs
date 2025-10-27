use std::rc::Rc;

use markup5ever_rcdom::{Node, NodeData};

use crate::{
    Element, element_handler::serialize_element, options::TranslationMode,
    text_util::concat_strings,
};

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

pub(crate) fn get_node_content(node: &Rc<Node>) -> String {
    let mut content = String::new();

    for child in get_node_children(node) {
        match &child.data {
            NodeData::Text { contents } => {
                content.push_str(&contents.borrow());
            }
            NodeData::Element { .. } => {
                content.push_str(&get_node_content(&child));
            }
            _ => {}
        }
    }

    content
}

// A handler for tags whose only criteria (for faithful translation) is the tag
// name of the parent.
pub(super) fn is_parent_handler(
    // The element to check.
    element: &Element,
    // A list of allowable tag names for this element's parent.
    tag_names: &Vec<&str>,
    // The value for `markdown_translate` to pass if this tag is markdown translatable.
    markdown_translated: bool,
) -> (Option<String>, bool) {
    // In faithful mode, only include these as HTML if they're not a child of the
    // `<tr>` tag.
    if element.options.translation_mode == TranslationMode::Faithful
        && !parent_tag_name_equals(element.node, tag_names)
    {
        (Some(serialize_element(element)), false)
    } else {
        (
            Some(concat_strings!("\n\n", element.content, "\n\n")),
            markdown_translated,
        )
    }
}
