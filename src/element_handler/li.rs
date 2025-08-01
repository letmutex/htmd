use crate::{
    node_util::get_node_tag_name,
    options::BulletListMarker,
    text_util::{concat_strings, indent_text_except_first_line, TrimAsciiWhitespace},
    Element,
};
use markup5ever_rcdom::NodeData;
use std::rc::Rc;

pub(super) fn list_item_handler(element: Element) -> Option<String> {
    let content = element.content.trim_start_ascii_whitespace().to_string();

    let ul_li = || {
        let marker = if element.options.bullet_list_marker == BulletListMarker::Asterisk {
            "*"
        } else {
            "-"
        };
        let spacing = " ".repeat(element.options.ul_bullet_spacing.into());
        let content = indent_text_except_first_line(&content, marker.len() + spacing.len(), true);
        Some(concat_strings!("\n", marker, spacing, content, "\n"))
    };

    // Add 1 before computing log10, then take the ceiling: it avoids log10(0) = Nan, and changes log10(10) = 1 into 2,
    // log10(100) into 3, etc.
    let digits = |num: usize| ((num + 1) as f32).log10().ceil() as usize;

    let ol_li = |index: usize, highest_list_item: usize| {
        let index_str = index.to_string();
        let spacing = " ".repeat(
            element.options.ol_number_spacing as usize + digits(highest_list_item)
                - index_str.len(),
        );
        let content =
            indent_text_except_first_line(&content, index_str.len() + 1 + spacing.len(), true);
        Some(concat_strings!(
            "\n", index_str, ".", spacing, content, "\n"
        ))
    };

    let parent_value = element.node.parent.take();

    let Some(weak) = parent_value.as_ref() else {
        return ul_li();
    };

    let Some(parent_node) = weak.upgrade() else {
        // Put the parent back
        element.node.parent.set(parent_value);
        return ul_li();
    };

    // Put the parent back
    element.node.parent.set(parent_value);

    if let NodeData::Element {
        ref name,
        ref attrs,
        ..
    } = parent_node.data
    {
        if &name.local != "ol" {
            return ul_li();
        }

        let mut index = 0;
        let mut total_list_items = 0;
        for child in parent_node.children.borrow().iter() {
            // Because `index` is assigned before `total_list_items` is incremented,
            // it is zero-based: the first list item is `index == 0`, the second
            // is `index == 1`, etc.
            if Rc::ptr_eq(child, element.node) {
                index = total_list_items;
            }
            if get_node_tag_name(child).is_some_and(|tag| tag == "li") {
                total_list_items += 1;
            }
        }

        let start = attrs
            .borrow()
            .iter()
            .find(|attr| &attr.name.local == "start")
            .map(|attr| attr.value.to_string().parse::<usize>().unwrap_or(1))
            .unwrap_or(1);

        // The highest list index is `start + total_list_items - 1`, since `start` is one-based, not zero-based.
        // For example, given `start = 5` and `total_list_items = 2` (a list of 5, 6), the highest index is 6.
        ol_li(start + index, start + total_list_items - 1)
    } else {
        ul_li()
    }
}
