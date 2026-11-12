use phf::phf_set;

use crate::{Node, NodeType};

pub(crate) fn get_node_tag_name<'a>(node: &'a Node<'_>) -> Option<&'a str> {
    match node.node_type() {
        NodeType::Document => Some("html"),
        NodeType::Element => node.tag_name(),
        _ => None,
    }
}

pub(crate) fn parent_tag_name_equals(node: &Node<'_>, tag_names: &[&str]) -> bool {
    node.parent()
        .as_ref()
        .and_then(get_node_tag_name)
        .is_some_and(|tag| tag_names.contains(&tag))
}

/// Whether `tag` preserves whitespace and holds text which goes out unescaped.
pub(crate) fn is_pre_element(tag: &str) -> bool {
    tag == "pre" || tag == "code"
}

pub(crate) fn is_inside_pre(node: &Node<'_>) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        if get_node_tag_name(&parent).is_some_and(is_pre_element) {
            return true;
        }
        current = parent.parent();
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
