use crate::{
    node_util::get_node_tag_name,
    options::BulletListMarker,
    text_util::{indent_text_except_first_line, TrimAsciiWhitespace},
    Element,
};
use markup5ever_rcdom::NodeData;
use std::rc::Rc;

pub(super) fn list_item_handler(element: Element) -> Option<String> {
    let content = element.content.trim_start_ascii_whitespace().to_string();
    let content = indent_text_except_first_line(&content, 4, true);

    let ul_li = || {
        let marker = if element.options.bullet_list_marker == BulletListMarker::Asterisk {
            "*"
        } else {
            "-"
        };
        Some(format!("\n{}   {}\n", marker, content))
    };

    let ol_li = |index: usize| Some(format!("\n{}.  {}\n", index, content));

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
        for child in parent_node.children.borrow().iter() {
            if Rc::ptr_eq(child, &element.node) {
                break;
            }
            if get_node_tag_name(child).is_some_and(|tag| tag == "li") {
                index += 1;
            }
        }

        let start = attrs
            .borrow()
            .iter()
            .find(|attr| &attr.name.local == "start")
            .map(|attr| attr.value.to_string().parse::<usize>().unwrap_or(1))
            .unwrap_or(1);

        return ol_li(start + index);
    } else {
        return ul_li();
    }
}
