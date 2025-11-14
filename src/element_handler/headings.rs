use crate::{
    Element, element_handler::Chain, options::HeadingStyle, serialize_if_faithful,
    text_util::TrimAsciiWhitespace,
};

pub(super) fn headings_handler(_chain: &dyn Chain, element: Element) -> (Option<String>, bool) {
    serialize_if_faithful!(element, 0);
    let level = element.tag.chars().nth(1).unwrap() as u32 - '0' as u32;
    let content = &element.content.trim_ascii_whitespace();

    let mut result = String::from("\n\n");
    if (level == 1 || level == 2) && element.options.heading_style == HeadingStyle::Setex {
        // Use the Setext heading style for h1 and h2
        result.push_str(content);
        result.push('\n');
        let ch = if level == 1 { "=" } else { "-" };
        result.push_str(&ch.repeat(content.chars().count()));
        result.push_str("\n\n");
    } else {
        result.push_str(&"#".repeat(level as usize));
        result.push(' ');
        result.push_str(content);
        result.push_str("\n\n");
    }
    (Some(result), true)
}
