use crate::{
    Element,
    element_handler::{
        HandlerResult, Handlers, br::blockquote_holds_unwritable_br,
        element_util::serialize_element,
    },
    options::TranslationMode,
    serialize_if_faithful,
    text_util::{JoinOnStringIterator, TrimDocumentWhitespace, concat_strings},
};

pub(super) fn blockquote_handler(
    handlers: &dyn Handlers,
    element: Element,
) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    // A quote holding a `<br>` that Markdown cannot write, as in
    // `<blockquote><br></blockquote>`, survives only as HTML: `br_handler`
    // writes nothing for such a break, so the quote would convert to the empty
    // string and vanish along with it.
    //
    // Serializing the *whole* quote rather than writing a `> <br>` line, because
    // that line's raw `<br>` would open an HTML block inside the quote, reading
    // every line up to the next blank one as raw HTML.
    if handlers.options().translation_mode == TranslationMode::Faithful
        && blockquote_holds_unwritable_br(element.node)
    {
        return Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        });
    }
    let content = handlers.walk_children(element.node).content;
    let content = content.trim_start_matches('\n');
    let content = content
        .trim_end_document_whitespace()
        .lines()
        .map(|line| concat_strings!("> ", line))
        .join("\n");
    Some(concat_strings!("\n\n", content, "\n\n").into())
}
