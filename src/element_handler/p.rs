use crate::{
    Element,
    element_handler::{
        HandlerResult, Handlers, br::block_holds_unwritable_br, element_util::serialize_element,
    },
    options::TranslationMode,
    serialize_if_faithful,
    text_util::concat_strings,
};

pub(super) fn p_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    // A paragraph holding a `<br>` that Markdown cannot write, as in
    // `<p><br></p>`, survives only as HTML. The whole paragraph has to be
    // serialized: a bare `<br>` line would parse back as a top-level `<br>`,
    // losing the `<p>` around it.
    if handlers.options().translation_mode == TranslationMode::Faithful
        && block_holds_unwritable_br(element.node)
    {
        return Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        });
    }
    let content = handlers.walk_children(element.node).content;
    let content = content.trim_matches('\n');
    Some(concat_strings!("\n\n", content, "\n\n").into())
}
