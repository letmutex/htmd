use crate::{
    Element,
    element_handler::{Chain, HandlerResult},
    node_util::{get_node_tag_name, get_parent_node},
    options::BulletListMarker,
    serialize_if_faithful,
    text_util::{TrimAsciiWhitespace, concat_strings, indent_text_except_first_line},
};

pub(super) fn list_item_handler(_chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(element, 0);
    let content = element.content.trim_start_ascii_whitespace().to_string();

    let ul_li = || {
        let marker = if element.options.bullet_list_marker == BulletListMarker::Asterisk {
            "*"
        } else {
            "-"
        };
        let spacing = " ".repeat(element.options.ul_bullet_spacing.into());
        let content = indent_text_except_first_line(&content, marker.len() + spacing.len(), true);

        Some(concat_strings!("\n", marker, spacing, content, "\n").into())
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
