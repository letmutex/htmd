use markup5ever_rcdom::NodeData;

use crate::{
    Element,
    element_handler::{Chain, HandlerResult, serialize_element},
    node_util::{get_node_tag_name, get_parent_node},
    options::{Options, TranslationMode},
    serialize_if_faithful,
    text_util::{concat_strings, indent_text_except_first_line, join_contents},
};

pub(super) fn list_handler(chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
    // In faithful mode, ...
    if element.options.translation_mode == TranslationMode::Faithful {
        // ...make sure this element's attributes can be translated as markdown.
        let has_start = element
            .attrs
            .first()
            .is_some_and(|attr| &attr.name.local == "start");
        serialize_if_faithful!(element, if has_start { 1 } else { 0 });

        // ...all children must be translated as Markdown, and all children must
        // be li elements.
        if !element.markdown_translated
            || !element.node.children.borrow().iter().all(|node| {
                let tag_name = get_node_tag_name(node);
                // In addition to elements, there will be text nodes, generally
                // with whitespace; these should be ignored.
                tag_name == Some("li") || tag_name.is_none()
            })
        {
            return Some(HandlerResult {
                content: serialize_element(&element),
                markdown_translated: false,
            });
        }
    }
    let parent = get_parent_node(element.node);
    let is_parent_li = parent
        .map(|p| get_node_tag_name(&p).is_some_and(|tag| tag == "li"))
        .unwrap_or(false);

    let content = if element.tag == "ol" {
        &get_ol_content(chain, &element)
    } else {
        element.content
    };

    if is_parent_li {
        Some(concat_strings!("\n", content.trim_matches(|ch| ch == '\n'), "\n").into())
    } else {
        Some(concat_strings!("\n\n", content.trim_matches(|ch| ch == '\n'), "\n\n").into())
    }
}

struct ListChildContent {
    text: String,
    is_li: bool,
}

fn get_ol_content(chain: &dyn Chain, element: &Element) -> String {
    let mut buffer: Vec<ListChildContent> = Vec::new();
    let mut li_count = 0;

    let start_idx = element
        .attrs
        .iter()
        .find(|attr| &attr.name.local == "start")
        .map(|attr| attr.value.to_string().parse::<usize>().unwrap_or(1))
        .unwrap_or(1);

    for child in element.node.children.borrow().iter() {
        let Some(res) = chain.handle(child) else {
            continue;
        };

        if let NodeData::Element { ref name, .. } = child.data
            && &name.local == "li"
        {
            buffer.push(ListChildContent {
                text: res.content,
                is_li: true,
            });
            li_count += 1;
        } else {
            buffer.push(ListChildContent {
                text: res.content,
                is_li: false,
            });
        }
    }

    // `start_idx` is one-based, not zero-based
    let highest_index = start_idx + li_count - 1;

    let mut curr_li_idx = start_idx - 1;

    let contents = buffer
        .into_iter()
        .map(|content| {
            if content.is_li {
                curr_li_idx += 1;
                add_ol_li_marker(element.options, &content.text, curr_li_idx, highest_index)
            } else {
                content.text
            }
        })
        .collect::<Vec<String>>();

    join_contents(&contents)
}

// Add 1 before computing log10, then take the ceiling: it avoids log10(0) =
// Nan, and changes log10(10) = 1 into 2, log10(100) into 3, etc.
fn digits(num: usize) -> usize {
    ((num + 1) as f32).log10().ceil() as usize
}

fn add_ol_li_marker(
    options: &Options,
    content: &str,
    index: usize,
    highest_index: usize,
) -> String {
    let index_str = index.to_string();
    let spacing =
        " ".repeat(options.ol_number_spacing as usize + digits(highest_index) - index_str.len());
    let content = content.trim_start_matches(|c| c == '\n');
    let content = indent_text_except_first_line(content, index_str.len() + 1 + spacing.len(), true);
    concat_strings!("\n", index_str, ".", spacing, content)
}
