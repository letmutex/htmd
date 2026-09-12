use crate::{
    Element,
    element_handler::{
        HandlerResult, Handlers,
        anchor::LinkReferenceCheckpoint,
        element_util::{
            serialize_element_result, serialize_if_extra_attrs_or_inline,
            serialize_walked_element_when_faithful,
        },
    },
    node_util::get_node_tag_name,
    options::TranslationMode,
    text_util::frame_as_block,
};

pub(super) fn pre_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    // A code block is a CommonMark block, so it needs a block context. In an
    // inline context this `<pre>` is a raw HTML inline instead, its content
    // walked like that of any other one.
    serialize_if_extra_attrs_or_inline!(handlers, element, 0);
    // The only faithful translation for this is from
    // `<pre><code>blah</code></pre>` to a code block. So, check that this node
    // has only one element, a pure `<code>` element. Cases:
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
        // The walk is speculative: a child which only HTML can express sends
        // the whole `<pre>` out as HTML, discarding this content.
        let checkpoint = LinkReferenceCheckpoint::new();
        let result = handlers.walk_children(element.node, element.context);

        serialize_walked_element_when_faithful!(
            handlers,
            element,
            !result.markdown_translated,
            checkpoint
        );

        Some(frame_as_block(&result.content).into())
    } else {
        Some(serialize_element_result(handlers, &element))
    }
}
