use crate::{
    Context, Element,
    element_handler::{
        HandlerResult, Handlers,
        code::{code_block, find_language_from_class},
        element_util::{
            serialize_element, serialize_element_result, serialize_if_extra_attrs_or_inline,
            serialize_when_faithful,
        },
    },
    options::TranslationMode,
    util::{node::get_node_tag_name, text::frame_as_block},
};

pub(super) fn pre_handler(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
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
        let children_view = element.node.children();
        let mut children = children_view.iter();
        children.next().is_some_and(|first| {
            children.next().is_none() && get_node_tag_name(&first) == Some("code")
        })
    };

    if handlers.options().translation_mode == TranslationMode::Pure || is_simple_code_block {
        let result = handlers.walk_children(
            element.node,
            Context {
                literal: true,
                ..element.context
            },
        );

        serialize_when_faithful!(
            handlers,
            !result.markdown_translated,
            serialize_element(handlers, element)
        );

        // A `<code>` child fences itself (`code_handler`); text and `<span>`s
        // straight inside `<pre>`, as Pygments and GitHub write them, do not.
        let has_code_child = element
            .node
            .children()
            .iter()
            .any(|child| get_node_tag_name(&child) == Some("code"));
        if has_code_child {
            return Some(frame_as_block(&result.content).into());
        }
        let content = result.content.strip_suffix('\n').unwrap_or(&result.content);
        let language = element
            .attrs
            .iter()
            .find(|attr| &attr.name.local == "class")
            .and_then(|attr| find_language_from_class(&attr.value));
        Some(frame_as_block(&code_block(handlers, content, language.as_deref())).into())
    } else {
        Some(serialize_element_result(handlers, element))
    }
}
