use crate::{
    Element,
    element_handler::{
        HandlerResult, Handlers, br::block_holds_unwritable_br, element_util::serialize_element,
    },
    node_util::{get_node_tag_name, get_parent_node},
    options::{BulletListMarker, TranslationMode},
    serialize_if_faithful,
    text_util::{TrimDocumentWhitespace, concat_strings, indent_text_except_first_line},
};

pub(super) fn list_item_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    // An item holding a `<br>` that Markdown cannot write — one with nothing
    // ahead of it on the line and nothing after it in the item, as in
    // `<li><br><ul>…</ul></li>` — survives only as HTML. Reporting the item as
    // untranslated hands the decision to the list handler, which serializes the
    // whole list: a lone `<li>` of HTML would leave the surrounding list markers
    // to be read as Markdown around it.
    if handlers.options().translation_mode == TranslationMode::Faithful
        && block_holds_unwritable_br(element.node)
    {
        return Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        });
    }
    let mut content = handlers.walk_children(element.node).content;
    let start = content.len() - content.trim_start_document_whitespace().len();
    if start > 0 {
        content.drain(..start);
    }

    let ul_li = || {
        let marker = if handlers.options().bullet_list_marker == BulletListMarker::Asterisk {
            "*"
        } else {
            "-"
        };
        let spacing = " ".repeat(handlers.options().ul_bullet_spacing.into());
        // Indenting trims each line's trailing whitespace, save for a hard line
        // break: those two spaces are a `<br>`, not stray whitespace.
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
