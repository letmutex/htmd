use crate::{
    Element,
    element_handler::{HandlerResult, Handlers, element_util::serialize_element},
    node_util::get_node_tag_name,
    options::TranslationMode,
    serialize_if_faithful,
    text_util::concat_strings,
};

pub(super) fn pre_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);
    // The only faithful translation for this is from
    // `<pre><code>blah</code></pre>` to a code block. So, check that this node
    // has only one element, a pure `<code>` element. Cases:
    //
    // 1.  We're in pure translation mode. No special treatment.
    // 2.  We're in faithful mode:
    //     1.  The child is pure, consists of one element which is a code tag.
    //         No special treatment.
    //     2.  All other cases: produce HTML.
    let is_simple_code_block = {
        let children = element.node.children.borrow();
        element.markdown_translated
            && children.len() == 1
            && get_node_tag_name(&children[0]) == Some("code")
    };

    if handlers.options().translation_mode == TranslationMode::Pure || is_simple_code_block {
        let result = handlers.walk_children(element.node);

        if handlers.options().translation_mode == TranslationMode::Faithful
            && !result.markdown_translated
        {
            return Some(HandlerResult {
                content: serialize_element(handlers, &element),
                markdown_translated: false,
            });
        }

        let content = result.content.trim_matches('\n');
        Some(concat_strings!("\n\n", content, "\n\n").into())
    } else {
        Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        })
    }
}
