use std::cell::RefCell;

use crate::{
    Element, ElementHandler,
    element_handler::{Chain, HandlerResult},
    options::{LinkReferenceStyle, LinkStyle},
    serialize_if_faithful,
    text_util::{JoinOnStringIterator, StripWhitespace, TrimAsciiWhitespace, concat_strings},
};

pub(super) struct AnchorElementHandler {}

impl AnchorElementHandler {
    thread_local! {
        static LINKS: RefCell<Vec<String>> = const { RefCell::new(vec![]) };
    }
}

impl ElementHandler for AnchorElementHandler {
    fn append(&self) -> Option<String> {
        AnchorElementHandler::LINKS.with(|links| {
            let mut links = links.borrow_mut();
            if links.is_empty() {
                return None;
            }
            let result = concat_strings!("\n\n", links.join("\n"), "\n\n");
            links.clear();
            Some(result)
        })
    }

    fn handle(&self, _chain: &dyn Chain, element: Element) -> Option<HandlerResult> {
        let mut link: Option<String> = None;
        let mut title: Option<String> = None;
        for attr in element.attrs.iter() {
            let name = &attr.name.local;
            if name == "href" {
                link = Some(attr.value.to_string())
            } else if name == "title" {
                title = Some(attr.value.to_string());
            } else {
                // This is an attribute which can't be translated to Markdown.
                serialize_if_faithful!(element, 0);
            }
        }

        let Some(link) = link else {
            return Some(element.content.into());
        };

        let process_title = |text: String| {
            text.lines()
                .map(|line| line.trim_ascii_whitespace().replace('"', "\\\""))
                .filter(|line| !line.is_empty())
                .join("\n")
        };

        // Handle new lines in title
        let title = title.map(process_title);

        let link = link.replace('(', "\\(").replace(')', "\\)");

        let md = match element.options.link_style {
            LinkStyle::Inlined => self.build_inlined_anchor(element.content, link, title, false),
            LinkStyle::InlinedPreferAutolinks => {
                self.build_inlined_anchor(element.content, link, title, true)
            }
            LinkStyle::Referenced => self.build_referenced_anchor(
                element.content,
                link,
                title,
                &element.options.link_reference_style,
            ),
        };

        Some(md.into())
    }
}

impl AnchorElementHandler {
    pub(super) fn new() -> Self {
        Self {}
    }

    fn build_inlined_anchor(
        &self,
        content: &str,
        link: String,
        title: Option<String>,
        prefer_autolinks: bool,
    ) -> String {
        if prefer_autolinks && content == link {
            return concat_strings!("<", link, ">");
        }

        let has_spaces_in_link = link.contains(' ');
        let (content, _) = content.strip_leading_whitespace();
        let (content, trailing_whitespace) = content.strip_trailing_whitespace();
        concat_strings!(
            "[",
            content,
            "](",
            if has_spaces_in_link { "<" } else { "" },
            link,
            title
                .as_ref()
                .map_or(String::new(), |t| concat_strings!(" \"", t, "\"")),
            if has_spaces_in_link { ">" } else { "" },
            ")",
            trailing_whitespace.unwrap_or("")
        )
    }

    fn build_referenced_anchor(
        &self,
        content: &str,
        link: String,
        title: Option<String>,
        style: &LinkReferenceStyle,
    ) -> String {
        AnchorElementHandler::LINKS.with(|links| {
            let title = title.map_or(String::new(), |t| concat_strings!(" \"", t, "\""));
            let (current, append) = match style {
                LinkReferenceStyle::Full => {
                    let index = links.borrow().len() + 1;
                    (
                        concat_strings!("[", content, "][", index.to_string(), "]"),
                        concat_strings!("[", index.to_string(), "]: ", link, title),
                    )
                }
                LinkReferenceStyle::Collapsed => (
                    concat_strings!("[", content, "][]"),
                    concat_strings!("[", content, "]: ", link, title),
                ),
                LinkReferenceStyle::Shortcut => (
                    concat_strings!("[", content, "]"),
                    concat_strings!("[", content, "]: ", link, title),
                ),
            };
            links.borrow_mut().push(append);
            current
        })
    }
}
