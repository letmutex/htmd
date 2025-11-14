use crate::{
    Element,
    element_handler::{Chain, serialize_element},
    node_util::get_node_tag_name,
    options::TranslationMode,
    serialize_if_faithful,
    text_util::concat_strings,
};

pub(super) fn pre_handler(_chain: &dyn Chain, element: Element) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    // The only faithful translation for this is from
    // `<pre><code>blah</code></pre>` to a code block. So, check that this node
    // has only one element, a pure `<code>` element. Cases:
    //
    // 1.  We're in pure translation mode. No special treatment.
    // 2.  We're in faithful mode:
    //     1.  The child is pure, consists of one element which is a code tag.
    //         No special treatment.
    //     2.  All other cases: produce HTML.
    let children = element.node.children.borrow();
    if element.options.translation_mode == TranslationMode::Pure
        || (element.markdown_translated
            && children.len() == 1
            && get_node_tag_name(&children[0]) == Some("code"))
    {
        (Some(concat_strings!("\n\n", element.content, "\n\n")), true)
    } else {
        (Some(serialize_element(&element)), false)
    }
}
