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

    // Inside an element being written as HTML the break is written as HTML too,
    // whatever line it lands on and whatever `BrStyle` asks for. See
    // `in_raw_html`.
    if in_raw_html() {
        return Some(HandlerResult {
            content: serialize_element(handlers, &element),
            markdown_translated: false,
        });
    }

    // A `<code>` decides how its whole content is written, so it answers for
    // any break inside it — and the two kinds answer differently. Faithful mode
    // never arrives here: a `<code>` holding a `<br>` has a child that is not
    // text, which `code_handler` serializes whole.
    match code_context(element.node) {
        // A code span is literal text between two backticks: CommonMark turns a
        // newline inside one into a space, so no break syntax reaches inside it
        // and whatever is written there is literal rather than a break. Pure
        // mode drops the break, leaving the code span the same HTML without the
        // `<br>` gives.
        Some(CodeContext::Span) => return Some("".into()),
        // A code block's lines carry real newlines, so the break is simply one.
        // Break syntax would reach the output as code rather than as a break: a
        // `\` is a backslash the source never had, and two spaces are trailing
        // whitespace the block preserves verbatim.
        //
        // What becomes of the newline afterwards is the block style's business,
        // not the break's, and the two styles differ on one point: a fence keeps
        // an empty *first* line, while an indented block cannot begin with one —
        // a run of indent spaces and nothing else is a blank line, which the
        // block does not start at. `br_in_code_block_under_every_style` pins
        // both.
        Some(CodeContext::Block) => return Some("\n".into()),
        None => {}
    }

    // What the output line holds on either side of the break decides every
    // question below, so both walks are made once here and passed down.
    let before = scan_line_before(element.node);
    let after = scan_line_after(element.node);

    // A block that cannot carry a hard line break leaves no break syntax to
    // reach for, so neither style below applies. The block has already put
    // something on the line (a heading's `#`, a cell's `|`), so the raw `<br>`
    // faithful mode falls back to stays inline rather than opening an HTML
    // block.
    if !block_can_hold_break(handlers, element.node, &before, &after) {
        return Some(raw_br_or_drop(handlers, &element));
    }

    // A link label is the same story one level in: it too puts a marker — its
    // `[` — on the line ahead of the break, and it too can fail to carry one.
    // A break the label ends on has nothing to break onto inside it, and
    // neither spelling survives being written there anyway: the two spaces go
    // with the rest of the label's trailing whitespace, and the `\` is left in
    // the label as a literal backslash. The raw `<br>` fallback has the `[`
    // ahead of it, so it stays inline exactly as it does in a heading.
    if after.ends_link_label {
        return Some(raw_br_or_drop(handlers, &element));
    }

    // Neither break syntax survives with nothing after it in the same block: a
    // line holding only two spaces is blank, and a trailing `\` is a literal
    // backslash. That leaves the same fallback — but the raw `<br>` needs
    // content ahead of it on the line, since one that opens a line opens an
    // HTML block.
    if !after.has_content {
        return Some(if before.has_content {
            raw_br_or_drop(handlers, &element)
        } else if is_at_document_root(element.node) {
            // At the document root the raw `<br>` may open its line after all:
            // the HTML block it starts holds nothing but the `<br>` and ends at
            // the blank line separating it from what follows, so it parses back
            // as exactly this `<br>` with no enclosing element to lose.
            lone_br_or_drop(handlers, &element)
        } else {
            // With nothing on either side there is nothing to write at all.
            // Faithful mode keeps the break by serializing the whole enclosing
            // block instead (see `block_holds_unwritable_br`), which discards
            // whatever this returns; pure mode drops the break.
            "".into()
        });
    }

    match handlers.options().br_style {
        // Two trailing spaces break a line only when that line already holds
        // text: on an otherwise empty line they are invisible, and the
        // whitespace-only line they leave behind is blank, which ends the block
        // instead of breaking it. Where that happens, fall back to the
        // backslash form, which needs nothing ahead of it.
        BrStyle::TwoSpaces if !before.two_space_break_is_visible() => Some("\\\n".into()),
        BrStyle::TwoSpaces => Some("  \n".into()),
        BrStyle::Backslash => Some("\\\n".into()),
    }
}

/// What to write for a `<br>` that no break syntax reaches. Faithful mode keeps
/// it as HTML rather than lose it; pure mode drops it, leaving exactly what the
/// same HTML without the `<br>` produces.
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

/// What to write for a `<br>` that stands alone at the document root, where the
/// raw `<br>` needs a line — and so a blank line on either side — of its own.
/// Faithful mode keeps it that way; pure mode drops it, leaving what the
/// document without the `<br>` produces.
///
/// The surrounding blank lines are what makes the HTML block the raw `<br>`
/// opens end at the `<br>` itself; the document's own leading and trailing
/// newlines are trimmed off at the end of the conversion.
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
/// `<html>` around it. Only there is a line-opening raw `<br>` safe, because
/// only there is there nothing around it to lose.
///
/// A raw `<br>` that opens a line opens an HTML block, which runs to the next
/// blank line and reads every line it covers as raw HTML. What that costs
/// depends on how the enclosing element is written. One with no syntax of its
/// own — a `<p>`, whose Markdown is nothing but its content — is simply gone,
/// since nothing is left to say a paragraph was there. One written with a line
/// marker survives: the `>` of a blockquote and the bullet of a list item are
/// stripped before the line's content is looked at, so the block opens *inside*
/// the quote or the item rather than replacing it. Either way the lines it
/// swallows stop being Markdown, which is loss enough.
///
/// At the root there is no enclosing element in the first place, and
/// [`lone_br_or_drop`] puts a blank line on each side so the block it opens
/// holds nothing but the `<br>` itself.
fn is_at_document_root(node: &Rc<Node>) -> bool {
    get_parent_node(node)
        .and_then(|parent| get_node_tag_name(&parent).map(|tag| matches!(tag, "body" | "html")))
        .unwrap_or(false)
}

/// The kind of `<code>` a `<br>` sits in, so far as writing the break goes.
enum CodeContext {
    /// A code span: a `<code>` outside a `<pre>`, all of which is written
    /// between one pair of backticks on one line.
    Span,
    /// The `<code>` of a `<pre>` code block, whose fenced or indented lines do
    /// carry newlines.
    Block,
}

/// Which kind of `<code>` this `<br>` sits in, if either. The `<code>` answers
/// for every break under it, however deeply nested, since it decides how all of
/// its content is written.
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

/// Whether this block holds a `<br>` that has to be kept by serializing the
/// block as HTML.
///
/// The test is the break's surroundings: nothing ahead of it on the line, and
/// nothing after it in the block. Both spellings need one or the other. A hard
/// break needs content after it in the same block, since a break with nothing to
/// break onto is not a break. The raw `<br>` faithful mode falls back to needs
/// content before it on the line, since a `<br>` that opens a line opens an HTML
/// block instead, which reads every line up to the next blank one as raw HTML —
/// and where the enclosing element has no syntax of its own, as a `<p>` has
/// none, leaves nothing to say it was ever there. A `<br>` with neither leaves
/// `br_handler` nothing to write at all, so the break is lost outright unless
/// the block around it is serialized.
///
/// Blocks that put a marker of their own ahead of the break — a heading's `#`, a
/// cell's `|` — always have content on the line and so never reach here.
///
/// # This reports a superset
///
/// It answers from the break's surroundings alone, so it does not know about the
/// bail-outs `br_handler` reaches first. Where the break sits inside an element
/// faithful mode serializes anyway — a `<code>` whose children are not all text,
/// a `<span>` carrying an attribute — [`in_raw_html`] has already settled it as a
/// raw `<br>` and nothing is at risk, yet this still reports the block as holding
/// an unwritable break:
///
/// ```text
/// <p><code><br></code></p>  →  <p><code><br></code></p>
///                              where <code><br></code> alone would have done
/// ```
///
/// The output is correct either way, only more HTML than the break needed, and
/// the reach is small: the condition already requires the block to hold nothing
/// ahead of the break on its line and nothing after it at all.
///
/// Consecutive breaks are *not* this case, easy as they are to mistake for it.
/// In `<p>x<br><br></p>` the second `<br>` really does produce nothing — the
/// first ends the line ahead of it, leaving neither spelling available — and
/// serializing the paragraph is the only thing that keeps it.
///
/// Narrowing this would mean re-deriving from outside which elements each
/// handler serializes: the attribute count behind `serialize_if_faithful!`,
/// `code_handler`'s all-children-are-text rule, `emphasis_handler`'s flanking
/// check. Duplicating all three is a worse bargain than the extra HTML. The
/// structural fix runs the other way — let the walk report that a break was lost
/// and serialize on that — but `markdown_translated` is not that signal, since a
/// raw `<br>` sets it too and a block must not serialize for those.
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
/// The quote's own children are judged as for any block, and a nested
/// `<blockquote>` is followed into as well — which is the conservative half of
/// this, not a necessity. The nested quote already answers for its own break by
/// serializing itself, and the `> ` this one would write around that HTML holds
/// up on its own: the marker is stripped before the line's content is read, so
/// `> <blockquote><br></blockquote>` parses back as the pair of quotes it came
/// from. Following into the nested quote trades that for one HTML block
/// covering both, which is easier to reason about and no less faithful.
///
/// So this is a choice, and `br_only_blockquote_faithful` is what pins it —
/// dropping the second clause leaves the round-trip sweep green and fails only
/// that expectation. Unlike a `<li>`, which hands its list the whole
/// serialization because a lone `<li>` of HTML really would leave the list's
/// markers to be read as Markdown around it, nothing forces the choice here.
pub(super) fn blockquote_holds_unwritable_br(node: &Rc<Node>) -> bool {
    block_holds_unwritable_br(node)
        || node.children.borrow().iter().any(|child| {
            get_node_tag_name(child) == Some("blockquote") && blockquote_holds_unwritable_br(child)
        })
}

/// Whether the block holding this `<br>` can express it as a hard line break.
/// Only a heading or a table cell can fail to. An ATX heading is a single line,
/// so any break syntax ends the heading and turns what follows into a
/// paragraph, and a Setext heading — which does span lines — still needs its
/// break to have content both before it, to keep the break syntax visible and
/// leave the underline a line to attach to, and after it, since a break with
/// nothing to break onto is not a break at all. A cell is written between two
/// pipes on a single line and cannot hold a newline at all, so no break syntax
/// reaches inside one.
///
/// Anything else is left to the styles below, which handle their own fallbacks.
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

/// The block this `<br>` sits directly in: the block element enclosing the
/// break is what decides how the break is written.
///
/// A table cell is the exception. It is written between two pipes on a single
/// line, and that holds however deeply the break is nested inside it: a cell
/// wrapping its content in a `<p>` still cannot take the newline that `<p>`
/// would write, and flattening the cell onto its one line would leave the break
/// syntax behind as literal text. So a cell answers for every break under it,
/// while any other block answers only for the breaks directly in it.
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
    /// Whether anything ahead of the break writes content on the line. This is
    /// what the raw `<br>` needs to stay inline rather than open an HTML block.
    has_content: bool,
    /// Whether the break stands at the very start of a link label, right after
    /// the `[`, with nothing of the label's own ahead of it. The `[` itself is
    /// content on the line — it is why the raw `<br>` is safe here — but it is
    /// not content the label's whitespace survives being stripped against.
    starts_link_label: bool,
}

impl LineBefore {
    /// Whether two trailing spaces written here would reach the output as a
    /// break. They need content ahead of them on the line, and inside a link
    /// label that content has to be inside the label as well: the label's
    /// leading whitespace is stripped when the `[...]` around it is built, which
    /// would take the two spaces — and with them the break — along with it.
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
        // Nothing lies ahead of the break within this element. The line reaches
        // back past it only while it is inline: a block element starts a line of
        // its own, so the break is at the start of that line.
        if get_node_tag_name(&parent).is_none_or(is_block_element) {
            break;
        }
        // Nothing of the label lies ahead of the break either, so the break is
        // written against the `[`.
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

/// What a walk forward from a `<br>` toward the end of its block found. The
/// mirror of [`LineBefore`], and read the same way: one walk, two questions.
struct LineAfter {
    /// Whether inline content follows the break in the same block. A hard line
    /// break needs it: with nothing after it, the break has nothing to break
    /// onto.
    has_content: bool,
    /// Whether the break ends a link label, with nothing of the label's own
    /// after it. The label is written between a `[` and a `]`, and the `]` is
    /// not a line for the break to land on — so a label is as much of a wall to
    /// a break as the block around it, just one boundary sooner. A break outside
    /// any label is nobody's business here, so this stays false for it.
    ends_link_label: bool,
}

/// Walks forward from `node` through the inline elements around it, scanning
/// each one's siblings, until something reaches the output or a block boundary
/// ends the block.
///
/// A label cannot reach past its block, so the label question is answered along
/// the way rather than by a walk of its own.
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
        // Nothing lies after the break within this element. The block reaches
        // past it only while it is inline.
        if get_node_tag_name(&parent).is_none_or(is_block_element) {
            break;
        }
        // Nothing of the label lies after the break either, so the break is
        // written against the `]`.
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
/// `[` opens the line ahead of its content and whose brackets close around that
/// content once its leading and trailing whitespace is stripped. An `<a>`
/// without an `href` writes nothing of its own, so its content is simply the
/// content of whatever holds it.
///
/// An `<a>` faithful mode serializes as HTML instead — one carrying an
/// attribute Markdown has no place for — never reaches here: a `<br>` inside it
/// is written as HTML too, which [`in_raw_html`] settles first.
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
                // A `<br>` has no children, so it contributes nothing of its
                // own here and whatever follows it decides.
                if scan_forward(&node.children.borrow()) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}
