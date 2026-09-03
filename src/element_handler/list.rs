use markup5ever_rcdom::NodeData;

use crate::{
    Element,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers, element_util::serialize_element_result},
    node_util::{get_node_tag_name, get_parent_node},
    options::{Options, TranslationMode},
    text_util::{append_block, concat_strings, frame_as_block, indent_text_except_first_line},
};

pub(super) fn list_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    // In faithful mode, ...
    if handlers.options().translation_mode == TranslationMode::Faithful {
        // ...make sure this element's attributes can be translated as markdown.
        let has_start = element
            .attrs
            .first()
            .is_some_and(|attr| &attr.name.local == "start");
        serialize_if_extra_attrs!(handlers, element, if has_start { 1 } else { 0 });

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
            return Some(serialize_element_result(handlers, &element));
        }
    }
    let parent = get_parent_node(element.node);
    let is_parent_li = parent
        .map(|p| get_node_tag_name(&p).is_some_and(|tag| tag == "li"))
        .unwrap_or(false);

    let result = if element.tag == "ol" {
        let (content, translated) = get_ol_content(handlers, &element);
        HandlerResult {
            content,
            markdown_translated: translated,
        }
    } else {
        handlers.walk_children(element.node)
    };

    if handlers.options().translation_mode == TranslationMode::Faithful
        && !result.markdown_translated
    {
        return Some(serialize_element_result(handlers, &element));
    }

    let trimmed = result.content.trim_matches(|ch| ch == '\n');
    if trimmed.is_empty() {
        return None;
    }

    if is_parent_li {
        Some(concat_strings!("\n", trimmed, "\n").into())
    } else {
        Some(frame_as_block(trimmed).into())
    }
}

struct ListChildContent {
    text: String,
    is_li: bool,
}

fn get_ol_content(handlers: &dyn Handlers, element: &Element) -> (String, bool) {
    let mut buffer: Vec<ListChildContent> = Vec::new();
    let mut li_count = 0;
    let mut all_translated = true;

    let start_idx = element
        .attrs
        .iter()
        .find(|attr| &attr.name.local == "start")
        .map(|attr| attr.value.to_string().parse::<i32>().unwrap_or(1).max(1) as usize)
        .unwrap_or(1);

    for child in element.node.children.borrow().iter() {
        let Some(res) = handlers.handle(child) else {
            continue;
        };
        if !res.markdown_translated {
            all_translated = false;
        }

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

    let capacity = buffer.iter().map(|content| content.text.len()).sum();
    let mut contents = String::with_capacity(capacity);
    for content in buffer {
        let rendered = if content.is_li {
            curr_li_idx += 1;
            add_ol_li_marker(
                handlers.options(),
                &content.text,
                curr_li_idx,
                highest_index,
            )
        } else {
            content.text
        };
        append_block(&mut contents, &rendered);
    }

    (contents, all_translated)
}

fn digits(num: usize) -> usize {
    if num == 0 {
        1
    } else {
        num.ilog10() as usize + 1
    }
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
    let content = content.trim_start_matches('\n');
    let content = indent_text_except_first_line(content, index_str.len() + 1 + spacing.len(), true);
    concat_strings!("\n", index_str, ".", spacing, content)
}

#[cfg(test)]
mod tests {
    use crate::element_handler::list::digits;

    #[test]
    fn test_count_digits() {
        assert_eq!(1, digits(1));
        assert_eq!(1, digits(0));
        assert_eq!(2, digits(45));
        assert_eq!(3, digits(450));
    }
}
