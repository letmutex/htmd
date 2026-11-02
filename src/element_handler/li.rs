use std::{cell::Cell, ops::Range, rc::Rc};

use html5ever::tokenizer::{BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer};
use markup5ever_rcdom::{Node, NodeData};

use crate::{
    Context, Element,
    element_handler::element_util::{
        list_marker_spacing, serialize_element, serialize_if_extra_attrs_or_inline,
    },
    element_handler::{HandlerResult, Handlers},
    options::{BulletListMarker, TranslationMode},
    util::{
        node::{get_node_tag_name, get_parent_node},
        text::{TrimDocumentWhitespace, concat_strings, indent_text_except_first_line},
    },
};

pub(super) fn list_item_handler(
    handlers: &dyn Handlers,
    element: &Element,
) -> Option<HandlerResult> {
    serialize_if_extra_attrs_or_inline!(handlers, element, 0);
    // A list item is a container block: its children begin a block context.
    let mut content = handlers.walk_children_content(
        element.node,
        Context {
            task_markers: true,
            ..Context::BLOCK
        },
    );
    let start = content.len() - content.trim_start_document_whitespace().len();
    if start > 0 {
        content.drain(..start);
    }

    let ul_li = || {
        let marker = if handlers.options().bullet_list_marker == BulletListMarker::Asterisk {
            "*"
        } else {
            "-"
        };
        let spacing = list_marker_spacing(handlers.options().ul_bullet_spacing.into());
        let content = indent_text_except_first_line(&content, marker.len() + spacing.len(), true);

        Some(concat_strings!("\n", marker, spacing, content).into())
    };

    let ol_li = || {
        // Marker will be added in the ol handler
        Some(concat_strings!("\n", content, "\n").into())
    };

    if let Some(parent) = get_parent_node(element.node)
        && let Some(parent_tag_name) = get_node_tag_name(&parent)
        && parent_tag_name == "ol"
    {
        ol_li()
    } else {
        ul_li()
    }
}

fn is_whitespace(node: &Rc<Node>) -> bool {
    matches!(
        &node.data,
        NodeData::Text { contents }
            if !has_non_whitespace_text(&contents.borrow())
    )
}

fn has_non_whitespace_text(text: &str) -> bool {
    text.chars()
        .any(|character| !character.is_ascii_whitespace())
}

fn is_ignorable_prefix(node: &Rc<Node>, mode: TranslationMode) -> bool {
    is_whitespace(node)
        || (mode == TranslationMode::Pure && matches!(&node.data, NodeData::Comment { .. }))
}

fn is_checkbox(node: &Rc<Node>) -> bool {
    matches!(&node.data, NodeData::Element { name, attrs, .. }
        if &name.local == "input" && attrs.borrow().iter().any(|attr|
            &attr.name.local == "type" && attr.value.eq_ignore_ascii_case("checkbox")))
}

fn count_checkboxes(node: &Rc<Node>) -> usize {
    let mut count = usize::from(is_checkbox(node));

    for child in node.children.borrow().iter() {
        count += count_checkboxes(child);
        if count >= 2 {
            return 2;
        }
    }

    count
}

fn has_task_description(description: &str, mode: TranslationMode) -> bool {
    let normalized = description.replace("\r\n", "\n").replace('\r', "\n");
    let mut paragraph = String::new();
    for (index, line) in normalized.split_inclusive('\n').enumerate() {
        if index > 0 && line.trim_matches([' ', '\t', '\n']).is_empty() {
            break;
        }
        paragraph.push_str(line);
    }
    let paragraph = paragraph.replace("\\\n", "");
    if !paragraph.contains(['<', '&']) {
        return has_non_whitespace_text(&paragraph);
    }

    let input = BufferQueue::default();
    input.push_back(paragraph.into());
    let tokenizer = Tokenizer::new(
        TaskDescription {
            has_content: Cell::new(false),
            mode,
        },
        Default::default(),
    );
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    tokenizer.sink.has_content.get()
}

struct TaskDescription {
    has_content: Cell<bool>,
    mode: TranslationMode,
}

impl TokenSink for TaskDescription {
    type Handle = ();

    fn process_token(&self, token: Token, _: u64) -> TokenSinkResult<()> {
        let has_content = match token {
            Token::CharacterTokens(text) => has_non_whitespace_text(&text),
            Token::TagToken(tag) => {
                // HTML tokenization also reads Markdown autolinks as tags.
                let is_autolink_or_text = tag
                    .name
                    .bytes()
                    .any(|byte| !byte.is_ascii_alphanumeric() && byte != b'-');
                is_autolink_or_text
                    || (tag.kind == TagKind::StartTag
                        && match tag.name.as_ref() {
                            "br" | "svg" => self.mode == TranslationMode::Faithful,
                            "img" => {
                                self.mode == TranslationMode::Faithful
                                    || tag.attrs.iter().any(|attr| &attr.name.local == "src")
                            }
                            _ => false,
                        })
            }
            _ => false,
        };
        self.has_content.set(self.has_content.get() || has_content);
        TokenSinkResult::Continue
    }
}

/// A provisional task marker awaiting conversion of its first-paragraph description.
pub(crate) struct PendingTask {
    checkbox: Rc<Node>,
    marker_range: Range<usize>,
}

impl PendingTask {
    /// Records a marker emitted by an input at the start of a paragraph.
    pub(crate) fn from_output(
        checkbox: &Rc<Node>,
        marker_start: usize,
        output: &str,
    ) -> Option<Self> {
        if is_checkbox(checkbox)
            && !has_non_whitespace_text(&output[..marker_start])
            && matches!(&output[marker_start..], "[ ] " | "[x] ")
        {
            Some(Self {
                checkbox: Rc::clone(checkbox),
                marker_range: marker_start..output.len(),
            })
        } else {
            None
        }
    }

    /// Removes empty task markers, returning whether the result remains Markdown.
    pub(crate) fn finish(self, handlers: &dyn Handlers, output: &mut String) -> bool {
        if has_task_description(
            &output[self.marker_range.end..],
            handlers.options().translation_mode,
        ) {
            return true;
        }

        let faithful = handlers.options().translation_mode == TranslationMode::Faithful;
        let checkbox_html = if faithful {
            let NodeData::Element { attrs, .. } = &self.checkbox.data else {
                unreachable!("a task marker belongs to an input element")
            };
            serialize_element(
                handlers,
                &Element {
                    node: &self.checkbox,
                    tag: "input",
                    attrs: &attrs.borrow(),
                    context: Context::INLINE,
                    skipped_handlers: 0,
                },
            )
        } else {
            String::new()
        };
        output.replace_range(self.marker_range, &checkbox_html);
        !faithful
    }
}

fn task_checkbox(item: &Rc<Node>, candidate: &Rc<Node>, mode: TranslationMode) -> Option<Rc<Node>> {
    let children = item.children.borrow();
    let first = children
        .iter()
        .find(|node| !is_ignorable_prefix(node, mode))?;
    let paragraph = if get_node_tag_name(first) == Some("p") {
        if mode == TranslationMode::Faithful
            && let NodeData::Element { attrs, .. } = &first.data
            && !attrs.borrow().is_empty()
        {
            return None;
        }
        first
    } else {
        item
    };

    let content = paragraph.children.borrow();
    let checkbox = content
        .iter()
        .find(|node| !is_ignorable_prefix(node, mode))?;
    if !Rc::ptr_eq(checkbox, candidate) || !is_checkbox(checkbox) {
        return None;
    }
    let count = content
        .iter()
        .take_while(|node| {
            !get_node_tag_name(node).is_some_and(crate::util::node::is_block_element)
        })
        .try_fold(0usize, |count, node| {
            let count = count + count_checkboxes(node);
            (count < 2).then_some(count)
        });
    if count != Some(1) {
        return None;
    }
    if mode == TranslationMode::Faithful
        && let NodeData::Element { attrs, .. } = &checkbox.data
        && attrs
            .borrow()
            .iter()
            .any(|attr| !matches!(attr.name.local.as_ref(), "type" | "checked"))
    {
        return None;
    }
    Some(Rc::clone(checkbox))
}

pub(super) fn input_handler(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    let parent = get_parent_node(element.node);
    let item = parent.and_then(|parent| {
        if get_node_tag_name(&parent) == Some("p") {
            get_parent_node(&parent)
        } else {
            Some(parent)
        }
    });
    if element.context.task_markers
        && item.as_ref().is_some_and(|item| {
            get_node_tag_name(item) == Some("li")
                && task_checkbox(item, element.node, handlers.options().translation_mode).is_some()
        })
    {
        let checked = element
            .attrs
            .iter()
            .any(|attr| &attr.name.local == "checked");
        Some(if checked { "[x] " } else { "[ ] " }.into())
    } else if handlers.options().translation_mode == TranslationMode::Pure {
        Some("".into())
    } else {
        Some(super::element_util::serialize_element_result(
            handlers, element,
        ))
    }
}
