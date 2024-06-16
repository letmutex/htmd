use std::{cell::RefCell, rc::Rc};

use html5ever::Attribute;
use markup5ever_rcdom::Node;

use crate::{
    options::{LinkReferenceStyle, LinkStyle, Options},
    text_util::{StripWhitespace, TrimAsciiWhitespace},
};

use super::ElementHandler;

pub(super) struct AnchorElementHandler {
    links: RefCell<Vec<String>>,
}

impl ElementHandler for AnchorElementHandler {
    fn append(&self) -> Option<String> {
        let mut links = self.links.borrow_mut();
        if links.is_empty() {
            return None;
        }
        let result = format!("\n\n{}\n\n", links.join("\n"));
        links.clear();
        Some(result)
    }

    fn on_visit(
        &self,
        _node: Rc<Node>,
        _tag: String,
        attrs: Vec<Attribute>,
        content: String,
        options: &Options,
    ) -> Option<String> {
        let mut link: Option<String> = None;
        let mut title: Option<String> = None;
        for attr in attrs.iter() {
            let name = &attr.name.local;
            if name == "href" {
                link = Some(attr.value.to_string())
            } else if name == "title" {
                title = Some(attr.value.to_string());
            }
        }

        let Some(link) = link else {
            return Some(content);
        };

        let process_title = |text: String| {
            text.lines()
                .map(|line| line.trim_ascii_whitespace().replace("\"", "\\\""))
                .filter(|line| !line.is_empty())
                .collect::<Vec<String>>()
                .join("\n")
        };

        // Handle new lines in title
        let title = title.map(process_title);

        let link = link.replace("(", "\\(").replace(")", "\\)");

        let md = if options.link_style == LinkStyle::Inlined {
            self.build_inlined_anchor(content, link, title)
        } else {
            self.build_referenced_anchor(content, link, &options.link_reference_style)
        };

        Some(md)
    }
}

impl AnchorElementHandler {
    pub(super) fn new() -> Self {
        Self {
            links: RefCell::new(vec![]),
        }
    }

    fn build_inlined_anchor(&self, content: String, link: String, title: Option<String>) -> String {
        let (content, leading_whitespace) = content.strip_leading_whitespace();
        let (content, trailing_whitespace) = content.strip_trailing_whitespace();
        format!(
            "{}[{}]({}{}){}",
            leading_whitespace.unwrap_or(""),
            content,
            link,
            title.map_or(String::new(), |t| format!(" \"{}\"", t)),
            trailing_whitespace.unwrap_or(""),
        )
    }

    fn build_referenced_anchor(
        &self,
        content: String,
        link: String,
        style: &LinkReferenceStyle,
    ) -> String {
        let (current, append) = match style {
            LinkReferenceStyle::Full => {
                let index = self.links.borrow().len() + 1;
                (
                    format!("[{}][{}]", content, index),
                    format!("[{}]: {}", index, link),
                )
            }
            LinkReferenceStyle::Collapsed => (
                format!("[{}][]", content),
                format!("[{}]: {}", content, link),
            ),
            LinkReferenceStyle::Shortcut => {
                (format!("[{}]", content), format!("[{}]: {}", content, link))
            }
        };
        self.links.borrow_mut().push(append);
        current
    }
}
