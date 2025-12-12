use crate::{
    Element,
    element_handler::{HandlerResult, Handlers},
    node_util::{get_node_tag_name, get_parent_node},
    options::BulletListMarker,
    serialize_if_faithful,
    text_util::{TrimDocumentWhitespace, concat_strings, indent_text_except_first_line},
};

pub(super) fn list_item_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    let content = handlers
        .walk_children(element.node)
        .content
        .trim_start_document_whitespace()
        .to_string();

    let ul_li = || {
        let marker = if handlers.options().bullet_list_marker == BulletListMarker::Asterisk {
            "*"
        } else {
            "-"
        };
        let spacing = " ".repeat(handlers.options().ul_bullet_spacing.into());
        let content = indent_text_except_first_line(&content, marker.len() + spacing.len(), true);

        Some(concat_strings!("\n", marker, spacing, content).into())
    };

    let ol_li = || {
        // Marker will be added in the ol handler
        Some(concat_strings!("\n", content, "\n").into())
    };

    if let Some(parent) = get_parent_node(element.node)
        && let Some(parent_tag_name) = get_node_tag_name(&parent)
        && parent_tag_name == "ol"
    {
        ol_li()
    } else {
        ul_li()
    }
}
