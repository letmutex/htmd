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
    // A quote holding a `<br>` that Markdown cannot write — one with nothing
    // ahead of it on the line and nothing after it in the quote, as in
    // `<blockquote><br></blockquote>` — survives only as HTML.
    //
    // Nothing is written for such a break at all (see `br_handler`), so the
    // quote would be left with no content: `<blockquote><br></blockquote>`
    // converts to the empty string, taking the quote down with the break.
    // Serializing keeps both.
    //
    // It has to be the *whole* quote rather than a `> <br>` line holding the
    // serialized break, because that line's content still has to be Markdown
    // htmd is willing to stand behind, and a raw `<br>` opening the quote's
    // first line opens an HTML block inside the quote — which reads every line
    // up to the next blank one as raw HTML. The `<blockquote>` itself is not at
    // risk: the `>` marker is stripped before the line's content is looked at,
    // so the quote survives whatever its content turns out to be.
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
