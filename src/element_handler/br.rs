use std::rc::Rc;

use markup5ever_rcdom::{Node, NodeData};

use crate::{
    Element,
    dom_walker::is_block_element,
    element_handler::{
        HandlerResult, Handlers,
        element_util::{in_raw_html, serialize_element},
    },
    node_util::{get_node_tag_name, get_parent_node},
    options::{BrStyle, HeadingStyle, TranslationMode},
    serialize_if_faithful,
    text_util::concat_strings,
};

pub(super) fn br_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_faithful!(handlers, element, 0);

    // Inside an element already being written as HTML, write the break as HTML
    // too, whatever `BrStyle` asks for. See `in_raw_html`.
    if in_raw_html() {
        return Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        });
    }

    // A `<code>` decides how all of its content is written, so it answers for
    // every break inside it. Faithful mode never arrives here: `code_handler`
    // serializes a `<code>` with a non-text child whole.
    match code_context(element.node) {
        // CommonMark turns a newline inside a code span into a space, so no
        // break syntax reaches inside one; anything written there is literal.
        Some(CodeContext::Span) => return Some("".into()),
        // A code block's lines carry real newlines, so the break is simply one;
        // break syntax would land in the code verbatim. What becomes of the
        // newline afterwards is the block style's business, and the two styles
        // differ: a fence keeps an empty first line, an indented block cannot
        // begin with one. `br_in_code_block_under_every_style` pins both.
        Some(CodeContext::Block) => return Some("\n".into()),
        None => {}
    }

    // What the output line holds on either side of the break decides every
    // question below, so both walks are made once here and passed down.
    let before = scan_line_before(element.node);
    let after = scan_line_after(element.node);

    // No break syntax is available, but the block has already put something on
    // the line (a heading's `#`, a cell's `|`), so the raw `<br>` faithful mode
    // falls back to stays inline rather than opening an HTML block.
    if !block_can_hold_break(handlers, element.node, &before, &after) {
        return Some(raw_br_or_drop(handlers, &element));
    }

    // A break ending a link label has nothing to break onto inside it, and
    // neither spelling survives there: the two spaces go with the rest of the
    // label's trailing whitespace, and the `\` is left as a literal backslash.
    // The label's `[` is already on the line, so the raw `<br>` stays inline.
    if after.ends_link_label {
        return Some(raw_br_or_drop(handlers, &element));
    }

    // With nothing after the break in the same block, neither spelling survives:
    // a line holding only two spaces is blank, and a trailing `\` is a literal
    // backslash.
    if !after.has_content {
        return Some(if before.has_content {
            raw_br_or_drop(handlers, &element)
        } else if is_at_document_root(element.node) {
            lone_br_or_drop(handlers, &element)
        } else {
            // Nothing on either side leaves nothing to write. Faithful mode
            // keeps the break by serializing the enclosing block instead (see
            // `block_holds_unwritable_br`), discarding this; pure mode drops it.
            "".into()
        });
    }

    match handlers.options().br_style {
        // Two spaces are invisible on an otherwise empty line, and the
        // whitespace-only line they leave behind is blank, which ends the block
        // instead of breaking it. The backslash form needs nothing ahead of it.
        BrStyle::TwoSpaces if !before.two_space_break_is_visible() => Some("\\\n".into()),
        BrStyle::TwoSpaces => Some("  \n".into()),
        BrStyle::Backslash => Some("\\\n".into()),
    }
}

/// What to write for a `<br>` that no break syntax reaches: faithful mode keeps
/// it as HTML, pure mode drops it.
///
/// The caller must have established that something already opened the output
/// line, since a raw `<br>` that opens one opens an HTML block instead.
fn raw_br_or_drop(handlers: &dyn Handlers, element: &Element) -> HandlerResult {
    if handlers.options().translation_mode == TranslationMode::Faithful {
        HandlerResult {
            content: serialize_element(handlers, element),
            // This was translated using HTML, not Markdown.
            markdown_translated: false,
        }
    } else {
        "".into()
    }
}

/// What to write for a `<br>` standing alone at the document root, where the raw
/// `<br>` needs a line — and so a blank line on either side — of its own.
///
/// Those blank lines are what makes the HTML block the raw `<br>` opens end at
/// the `<br>` itself; the document's own leading and trailing newlines are
/// trimmed off at the end of the conversion.
fn lone_br_or_drop(handlers: &dyn Handlers, element: &Element) -> HandlerResult {
    if handlers.options().translation_mode == TranslationMode::Faithful {
        HandlerResult {
            content: concat_strings!("\n\n", serialize_element(handlers, element), "\n\n"),
            // This was translated using HTML, not Markdown.
            markdown_translated: false,
        }
    } else {
        "".into()
    }
}

/// Whether this `<br>` sits at the document root, with nothing but `<body>` and
/// `<html>` around it — the only place a line-opening raw `<br>` is safe.
///
/// Elsewhere the [HTML block](https://spec.commonmark.org/0.31.2/#html-blocks)
/// it opens runs to the next blank line and reads every line it covers as raw
/// HTML. A `<p>` writes no syntax of its own, so nothing would be left to say a
/// paragraph was there; a `<blockquote>` or `<li>` survives, since its line
/// marker is stripped before the content is looked at, but the lines swallowed
/// stop being Markdown either way. At the root there is no enclosing element to
/// lose, and [`lone_br_or_drop`] blank-lines the block down to the `<br>`.
fn is_at_document_root(node: &Rc<Node>) -> bool {
    get_parent_node(node)
        .and_then(|parent| get_node_tag_name(&parent).map(|tag| matches!(tag, "body" | "html")))
        .unwrap_or(false)
}

/// The kind of `<code>` a `<br>` sits in, so far as writing the break goes.
enum CodeContext {
    /// A code span: a `<code>` outside a `<pre>`, written between one pair of
    /// backticks on one line.
    Span,
    /// The `<code>` of a `<pre>` code block, whose lines do carry newlines.
    Block,
}

/// Which kind of `<code>` this `<br>` sits in, if either. The `<code>` answers
/// for every break under it, however deeply nested.
fn code_context(node: &Rc<Node>) -> Option<CodeContext> {
    let mut current = node.clone();
    while let Some(parent) = get_parent_node(&current) {
        match get_node_tag_name(&parent) {
            Some("code") => {
                let is_block = get_parent_node(&parent)
                    .and_then(|grandparent| get_node_tag_name(&grandparent).map(|tag| tag == "pre"))
                    .unwrap_or(false);
                return Some(if is_block {
                    CodeContext::Block
                } else {
                    CodeContext::Span
                });
            }
            // A `<code>` is inline, so it cannot reach past a block boundary.
            Some(tag) if !is_block_element(tag) => current = parent,
            _ => return None,
        }
    }
    None
}

/// Whether this block holds a `<br>` that survives only by serializing the block
/// as HTML: one with nothing ahead of it on the line and nothing after it in the
/// block. A hard break needs content after it, the raw `<br>` fallback needs
/// content before it (see [`is_at_document_root`]), and with neither
/// `br_handler` writes nothing at all.
///
/// Blocks that put a marker of their own ahead of the break — a heading's `#`, a
/// cell's `|` — always have content on the line and so never reach here.
///
/// This deliberately reports a superset. Judging from the break's surroundings
/// alone means it cannot see the bail-outs `br_handler` reaches first, so
/// `<p><code><br></code></p>` serializes the whole paragraph where
/// `<code><br></code>` alone would have done. Narrowing it would mean
/// re-deriving out here which elements each handler serializes; the extra HTML
/// is the cheaper trade.
///
/// Consecutive breaks are *not* this case, easy as they are to mistake for it.
/// In `<p>x<br><br></p>` the second `<br>` really does produce nothing, and
/// serializing the paragraph is the only thing that keeps it.
pub(super) fn block_holds_unwritable_br(node: &Rc<Node>) -> bool {
    node.children
        .borrow()
        .iter()
        .any(|child| match get_node_tag_name(child) {
            Some("br") => {
                !scan_line_before(child).has_content && !scan_line_after(child).has_content
            }
            // A nested block writes its own content, so any break inside it is
            // that block's to answer for, not this one's.
            Some(tag) if is_block_element(tag) => false,
            _ => block_holds_unwritable_br(child),
        })
}

/// Whether this `<blockquote>` is serialized as HTML to keep a `<br>` inside it.
///
/// Following into a nested `<blockquote>` is a choice rather than a necessity:
/// the nested quote already serializes itself, and `> ` around that HTML holds
/// up, since the marker is stripped before the line's content is read. One HTML
/// block covering both is easier to reason about and no less faithful.
/// `br_only_blockquote_faithful` is what pins the choice.
pub(super) fn blockquote_holds_unwritable_br(node: &Rc<Node>) -> bool {
    block_holds_unwritable_br(node)
        || node.children.borrow().iter().any(|child| {
            get_node_tag_name(child) == Some("blockquote") && blockquote_holds_unwritable_br(child)
        })
}

/// Whether the block holding this `<br>` can express it as a hard line break.
/// Only a heading or a table cell can fail to: an ATX heading is a single line,
/// so any break syntax ends it; a Setext heading does span lines, but needs
/// content before the break to keep the syntax visible and leave the underline a
/// line to attach to, and content after it to break onto; a cell is written
/// between two pipes on a single line and cannot hold a newline at all.
fn block_can_hold_break(
    handlers: &dyn Handlers,
    node: &Rc<Node>,
    before: &LineBefore,
    after: &LineAfter,
) -> bool {
    match enclosing_block(node) {
        EnclosingBlock::Heading(level) => {
            level <= 2
                && handlers.options().heading_style == HeadingStyle::Setex
                && before.has_content
                && after.has_content
        }
        EnclosingBlock::TableCell => false,
        EnclosingBlock::Other => true,
    }
}

/// The kind of block a `<br>` sits in, so far as writing the break goes.
enum EnclosingBlock {
    /// A heading of the given level.
    Heading(u32),
    /// A `<td>` or `<th>`.
    TableCell,
    /// Any other block, or none at all: either way the break is written as
    /// ordinary Markdown.
    Other,
}

/// The block this `<br>` sits directly in.
///
/// A table cell is the exception, answering for every break under it however
/// deeply nested: it is written between two pipes on a single line, so a break
/// inside a `<p>` inside the cell has no newline available either, and
/// flattening the cell would leave the break syntax behind as literal text.
fn enclosing_block(node: &Rc<Node>) -> EnclosingBlock {
    let mut nearest = None;
    let mut current = node.clone();
    while let Some(parent) = get_parent_node(&current) {
        let Some(tag) = get_node_tag_name(&parent) else {
            break;
        };
        if matches!(tag, "td" | "th") {
            return EnclosingBlock::TableCell;
        }
        if nearest.is_none() && is_block_element(tag) {
            nearest = Some(
                match tag.strip_prefix('h').and_then(|level| level.parse().ok()) {
                    Some(level @ 1..=6) => EnclosingBlock::Heading(level),
                    _ => EnclosingBlock::Other,
                },
            );
        }
        current = parent;
    }
    nearest.unwrap_or(EnclosingBlock::Other)
}

/// What a walk back from a `<br>` toward the start of its line ran into first.
enum Preceding {
    /// Something that puts text on the line ahead of the break.
    Content,
    /// A newline, so the break stands at the start of an empty line.
    LineBreak,
    /// Nothing that reaches the output; keep looking further back.
    Nothing,
}

/// What a walk back from a `<br>` toward the start of its output line found.
struct LineBefore {
    /// Whether anything ahead of the break writes content on the line — what a
    /// raw `<br>` needs to stay inline rather than open an HTML block.
    has_content: bool,
    /// Whether the break stands right after a link label's `[`, with nothing of
    /// the label's own ahead of it. The `[` is content on the line, but not
    /// content the label's whitespace survives being stripped against.
    starts_link_label: bool,
}

impl LineBefore {
    /// Whether two trailing spaces written here would reach the output as a
    /// break. Inside a link label the content ahead of them has to be inside the
    /// label as well: building the `[...]` strips the label's leading
    /// whitespace, which would take the two spaces — and the break — with it.
    fn two_space_break_is_visible(&self) -> bool {
        self.has_content && !self.starts_link_label
    }
}

/// Walks back from `node` through the inline elements around it, scanning each
/// one's siblings, until something reaches the output or a block boundary ends
/// the line.
fn scan_line_before(node: &Rc<Node>) -> LineBefore {
    let mut starts_link_label = false;
    let mut current = node.clone();
    while let Some(parent) = get_parent_node(&current) {
        let preceding = {
            let children = parent.children.borrow();
            let Some(index) = children.iter().position(|c| Rc::ptr_eq(c, &current)) else {
                break;
            };
            scan_back(&children[..index])
        };
        match preceding {
            Preceding::Content => {
                return LineBefore {
                    has_content: true,
                    starts_link_label,
                };
            }
            Preceding::LineBreak => break,
            Preceding::Nothing => {}
        }
        // The line reaches back past this element only while it is inline: a
        // block starts a line of its own, so the break is at that line's start.
        if get_node_tag_name(&parent).is_none_or(is_block_element) {
            break;
        }
        // Nothing of the label lies ahead of the break, so it is written against
        // the `[`.
        if writes_link_label(&parent) {
            starts_link_label = true;
        }
        current = parent;
    }
    LineBefore {
        has_content: false,
        starts_link_label,
    }
}

/// Scans `nodes` — the siblings ahead of a `<br>`, in document order — back to
/// front, stopping at the first one that reaches the output.
fn scan_back(nodes: &[Rc<Node>]) -> Preceding {
    for node in nodes.iter().rev() {
        match &node.data {
            NodeData::Text { contents } => {
                if !contents.borrow().trim().is_empty() {
                    return Preceding::Content;
                }
            }
            NodeData::Element { name, .. } => {
                let tag = &*name.local;
                if tag == "br" || is_block_element(tag) {
                    // Both end the line they sit on.
                    return Preceding::LineBreak;
                }
                if tag == "img" {
                    // Childless, but it still writes a link to the line.
                    return Preceding::Content;
                }
                match scan_back(&node.children.borrow()) {
                    Preceding::Nothing => {}
                    found => return found,
                }
            }
            _ => {}
        }
    }
    Preceding::Nothing
}

/// What a walk forward from a `<br>` toward the end of its block found; the
/// mirror of [`LineBefore`].
struct LineAfter {
    /// Whether inline content follows the break in the same block. A break with
    /// nothing to break onto is not a break.
    has_content: bool,
    /// Whether the break ends a link label, with nothing of the label's own
    /// after it: the `]` is not a line for the break to land on, making the
    /// label as much of a wall as the block, one boundary sooner. False for a
    /// break outside any label.
    ends_link_label: bool,
}

/// Walks forward from `node` through the inline elements around it, scanning
/// each one's siblings, until something reaches the output or a block boundary
/// ends the block. A label cannot reach past its block, so the label question is
/// answered along the way rather than by a walk of its own.
fn scan_line_after(node: &Rc<Node>) -> LineAfter {
    let mut ends_link_label = false;
    let mut current = node.clone();
    while let Some(parent) = get_parent_node(&current) {
        let found = {
            let children = parent.children.borrow();
            let Some(index) = children.iter().position(|c| Rc::ptr_eq(c, &current)) else {
                break;
            };
            scan_forward(&children[index + 1..])
        };
        if found {
            return LineAfter {
                has_content: true,
                ends_link_label,
            };
        }
        // The block reaches past this element only while it is inline.
        if get_node_tag_name(&parent).is_none_or(is_block_element) {
            break;
        }
        // Nothing of the label lies after the break, so it is written against
        // the `]`.
        if writes_link_label(&parent) {
            ends_link_label = true;
        }
        current = parent;
    }
    LineAfter {
        has_content: false,
        ends_link_label,
    }
}

/// Whether this element is written as a Markdown link, `[content](href)`, whose
/// brackets close around the content once its leading and trailing whitespace is
/// stripped. An `<a>` without an `href` writes nothing of its own.
///
/// An `<a>` that faithful mode serializes as HTML never reaches here:
/// [`in_raw_html`] settles a `<br>` inside it first.
fn writes_link_label(node: &Rc<Node>) -> bool {
    get_node_tag_name(node) == Some("a")
        && matches!(&node.data, NodeData::Element { attrs, .. }
            if attrs.borrow().iter().any(|attr| &attr.name.local == "href"))
}

/// Scans `nodes` — the siblings after a `<br>`, in document order — for content
/// that lands on the line the break opens.
fn scan_forward(nodes: &[Rc<Node>]) -> bool {
    for node in nodes {
        match &node.data {
            NodeData::Text { contents } => {
                if !contents.borrow().trim().is_empty() {
                    return true;
                }
            }
            NodeData::Element { name, .. } => {
                let tag = &*name.local;
                if is_block_element(tag) {
                    // The block opens a line of its own, so neither it nor
                    // anything after it lands on the line this break opens.
                    return false;
                }
                if tag == "img" {
                    return true;
                }
                if scan_forward(&node.children.borrow()) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}
