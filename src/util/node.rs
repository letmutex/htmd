use std::rc::Rc;

use markup5ever_rcdom::{Node, NodeData};
use phf::phf_set;

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

/// Whether `tag` preserves whitespace and holds text which goes out unescaped.
pub(crate) fn is_pre_element(tag: &str) -> bool {
    tag == "pre" || tag == "code"
}

pub(crate) fn is_inside_pre(node: &Rc<Node>) -> bool {
    let mut current = get_parent_node(node);
    while let Some(parent) = current {
        if get_node_tag_name(&parent).is_some_and(is_pre_element) {
            return true;
        }
        current = get_parent_node(&parent);
    }
    false
}

/// CommonMark's [HTML block](https://spec.commonmark.org/0.31.2/#html-blocks)
/// type 1 tag list, the block-level half of [`BLOCK_ELEMENTS`] which ends at
/// the line holding its closing tag rather than at a blank line.
static TYPE_1_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "pre", "script", "style", "textarea",
};

pub(crate) fn is_type_1_element(tag: &str) -> bool {
    TYPE_1_ELEMENTS.contains(tag)
}

/// The tags which frame their content as a block rather than an inline run.
///
/// This is exactly CommonMark's
/// [HTML block](https://spec.commonmark.org/0.31.2/#html-blocks)
/// [type 1 list](TYPE_1_ELEMENTS) plus its type 6 list, and
/// `element_util::try_serialize_element` relies on that: a tag outside both
/// lists could only open a type 7 block, so it has to be written as a raw HTML
/// inline instead. Keep this set in step with the spec — adding a tag here
/// because it reads as block-like would silently change that classification.
pub(crate) static BLOCK_ELEMENTS: phf::Set<&'static str> = phf_set! {
    "address", "article", "aside", "base", "basefont", "blockquote", "body", "caption",
    "center", "col", "colgroup", "dd", "details", "dialog", "dir", "div", "dl", "dt",
    "fieldset", "figcaption", "figure", "footer", "form", "frame", "frameset", "h1", "h2",
    "h3", "h4", "h5", "h6", "head", "header", "hr", "html", "iframe", "legend", "li",
    "link", "main", "menu", "menuitem", "nav", "noframes", "ol", "optgroup", "option", "p",
    "param", "pre", "script", "search", "section", "style", "summary", "table", "tbody", "td",
    "textarea", "tfoot", "th", "thead", "title", "tr", "track", "ul",
};

pub(crate) fn is_block_element(tag: &str) -> bool {
    BLOCK_ELEMENTS.contains(tag)
}
