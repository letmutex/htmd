//! Tests for `<br>` in positions where the usual hard-break syntax breaks down.
//!
//! A `<br>` surrounded by text is easy (see `basic_tests::br`). The hard cases
//! are a `<br>` at the *start* of a block, as a block's *only* content, or as an
//! inline element's only content. Three CommonMark rules drive all of them:
//!
//! 1.  A hard line break must be followed by content in the same block, so
//!     neither `  \n` nor `\\\n` can render a `<br>` that ends a block: a line
//!     holding only two spaces is blank, and a trailing `\` is a literal
//!     backslash.
//! 2.  `br` is not a block-level tag name, so a raw `<br>` at the start of a
//!     line opens an HTML block of type 7, which runs to the next blank line and
//!     swallows whatever follows. Raw `<br>` is safe only where something else
//!     already started the line, as in an ATX heading.
//! 3.  An emphasis delimiter run opens emphasis only if it is *left-flanking*:
//!     not followed by whitespace, and either not followed by punctuation or
//!     else itself preceded by whitespace or punctuation — mirror-image for
//!     closing. Every spelling of a `<br>` starts and ends in punctuation, so a
//!     marker against one dies whenever the character on its far side is
//!     alphanumeric: `a*<br>*b` is the literal text `a*`, a break, and `*b`.
//!     Only `*<br>*` alone between whitespace or block boundaries survives,
//!     which is why `# *<br>*` works and `# *<br>*a` does not.
//!
//! Where a backslash break still works the output is plain Markdown and both
//! translation modes agree. Where nothing works they diverge: `Faithful` keeps
//! the `<br>` as HTML, while `Pure` drops it, joining the text on either side.
//!
//! Headings and table cells are always in that group: an ATX heading is a single
//! line, a Setext heading's underline needs a non-empty line above it, and a
//! cell cannot contain a newline at all.
//!
//! Elements htmd writes with *markers* — `*` for `<em>`, `**` for `<strong>` —
//! answer to rule 3 as well, and can be unwritable in a block that carries the
//! break perfectly well. There `Faithful` serializes the element, writing the
//! `<br>` inside it as HTML too: break syntax inside a tag is invisible and
//! would split the element across lines, where a lone closing tag opens an HTML
//! block of its own. `Pure` instead moves the break outside the markers, which
//! renders identically and lets the element vanish where the break was all it
//! held.
//!
//! The file has two layers. Most tests pin exact output, one shape at a time.
//! Under them, four tests sweep `CORPUS` — every shape the file exercises — for
//! the properties that must hold whatever spelling any one case settles on.

use htmd::{
    HtmlToMarkdown,
    options::{BrStyle, CodeBlockFence, CodeBlockStyle, HeadingStyle, Options, TranslationMode},
};
use indoc::indoc;
use pretty_assertions::assert_eq;
use scraper::{Html, Node};

mod common;
use common::render;

/// A one-column, one-row table whose body cell holds `cell`.
fn table(cell: &str) -> String {
    format!(
        "<table><thead><tr><th>h</th></tr></thead><tbody><tr><td>{cell}</td></tr></tbody></table>"
    )
}

fn convert(
    html: &str,
    translation_mode: TranslationMode,
    br_style: BrStyle,
    heading_style: HeadingStyle,
) -> String {
    HtmlToMarkdown::builder()
        .options(Options {
            translation_mode,
            br_style,
            heading_style,
            ..Default::default()
        })
        .build()
        .convert(html)
        .unwrap()
}

fn faithful(html: &str, br_style: BrStyle) -> String {
    convert(html, TranslationMode::Faithful, br_style, HeadingStyle::Atx)
}

fn faithful_setext(html: &str, br_style: BrStyle) -> String {
    convert(
        html,
        TranslationMode::Faithful,
        br_style,
        HeadingStyle::Setex,
    )
}

fn pure(html: &str, br_style: BrStyle) -> String {
    convert(html, TranslationMode::Pure, br_style, HeadingStyle::Atx)
}

fn pure_setext(html: &str, br_style: BrStyle) -> String {
    convert(html, TranslationMode::Pure, br_style, HeadingStyle::Setex)
}

/// Both spellings a Markdown hard break can take.
const STYLES: [BrStyle; 2] = [BrStyle::TwoSpaces, BrStyle::Backslash];

// Most shapes in this file come out the same under either `BrStyle`, since the
// break is written as a raw `<br>` or not written at all. The four helpers below
// say so by checking both styles; the style-dependent tests call `faithful` and
// `pure` directly.

fn assert_both_styles(
    expected: &str,
    html: &str,
    translation_mode: TranslationMode,
    heading_style: HeadingStyle,
) {
    for br_style in STYLES {
        assert_eq!(
            expected,
            convert(html, translation_mode, br_style, heading_style),
            "{html:?}, {br_style:?}"
        );
    }
}

fn assert_faithful(expected: &str, html: &str) {
    assert_both_styles(expected, html, TranslationMode::Faithful, HeadingStyle::Atx);
}

fn assert_faithful_setext(expected: &str, html: &str) {
    assert_both_styles(
        expected,
        html,
        TranslationMode::Faithful,
        HeadingStyle::Setex,
    );
}

fn assert_pure(expected: &str, html: &str) {
    assert_both_styles(expected, html, TranslationMode::Pure, HeadingStyle::Atx);
}

fn assert_pure_setext(expected: &str, html: &str) {
    assert_both_styles(expected, html, TranslationMode::Pure, HeadingStyle::Setex);
}

// ---------------------------------------------------------------------------
// Representable: a `<br>` that starts a block but is followed by content in
// that same block needs a backslash break. The two-space form would leave a
// blank line and lose the break, so the backslash is required even under
// `BrStyle::TwoSpaces`. Being plain Markdown, the result is the same in both
// translation modes.
// ---------------------------------------------------------------------------

#[test]
fn br_at_block_start_uses_a_backslash_break() {
    for (html, expected) in [
        ("<p><br>Text</p>", "\\\nText"),
        ("<p><br><em>Text</em></p>", "\\\n*Text*"),
        ("<ul><li><br>x</li></ul>", "*   \\\n    x"),
        ("<blockquote><br>x</blockquote>", "> \\\n> x"),
    ] {
        assert_faithful(expected, html);
        assert_pure(expected, html);
    }
}

// ---------------------------------------------------------------------------
// Unrepresentable in Markdown: faithful mode keeps the `<br>` as HTML, while
// pure mode drops it, joining the text on either side. Where the raw `<br>`
// would start a line it takes its enclosing block with it, since a bare `<br>`
// line opens an HTML block that drops the enclosing element and swallows what
// follows.
// ---------------------------------------------------------------------------

/// A `<br>` alone in a heading, at every level. The `#` markers have already
/// started the line, so a raw `<br>` stays inline and round-trips exactly.
///
/// Under `HeadingStyle::Setex` an h1 or h2 falls back to ATX, the only lossless
/// option: the raw `<br>` would start the line and swallow the `===` / `---`
/// underline. Levels past 2 have no Setext spelling to fall back from.
#[test]
fn br_only_heading_faithful() {
    for level in 1..=6usize {
        let html = format!("<h{level}><br></h{level}>");
        let expected = format!("{} <br>", "#".repeat(level));
        assert_faithful(&expected, &html);
        assert_faithful_setext(&expected, &html);
    }
}

/// A heading is the one block that cannot hold a break in any syntax, so both
/// break styles reach the same drop by the same route.
#[test]
fn br_only_heading_pure() {
    for level in 1..=6usize {
        let html = format!("<h{level}><br></h{level}>");
        let hashes = "#".repeat(level);
        assert_pure(&hashes, &html);
        // A Setext h1 or h2 has no markers left once the `<br>` goes; past
        // level 2 there is no Setext spelling and the `#`s remain.
        let expected = if level <= 2 { "" } else { hashes.as_str() };
        assert_pure_setext(expected, &html);
    }
}

/// A `<br>` between two runs of text in a heading. An ATX heading is a single
/// line, so either break syntax would end it and turn the remainder into a
/// paragraph: faithful mode falls back to the raw `<br>` and pure mode drops it.
///
/// A Setext heading spans lines, so an interior break is representable and needs
/// no special handling in either mode.
#[test]
fn br_inside_heading() {
    assert_faithful("# a<br>b", "<h1>a<br>b</h1>");
    assert_pure("# ab", "<h1>a<br>b</h1>");
    // Source whitespace around the `<br>` collapses to the one space
    // `<h1>a b</h1>` would give.
    assert_pure("# a b", "<h1>a\n<br>b</h1>");

    for convert in [faithful_setext, pure_setext] {
        assert_eq!(
            "a  \nb\n=====",
            convert("<h1>a<br>b</h1>", BrStyle::TwoSpaces)
        );
        assert_eq!(
            "a\\\nb\n====",
            convert("<h1>a<br>b</h1>", BrStyle::Backslash)
        );
    }
}

/// A `<br>` in a table cell. A cell cannot contain a newline, so a raw `<br>` is
/// the only way to keep the break; the pipe has already started the line, so it
/// stays inline. Pure mode drops it, joining what stood on either side.
#[test]
fn br_in_table_cell() {
    assert_faithful(
        indoc! {"
            | h      |
            | ------ |
            | a<br>b |"},
        &table("a<br>b"),
    );
    assert_faithful(
        indoc! {"
            | h    |
            | ---- |
            | <br> |"},
        &table("<br>"),
    );
    assert_pure(
        indoc! {"
            | h  |
            | -- |
            | ab |"},
        &table("a<br>b"),
    );
    // Source whitespace around the `<br>` survives as the single space HTML
    // collapses it to.
    assert_pure(
        indoc! {"
            | h   |
            | --- |
            | a b |"},
        &table("a\n<br>b"),
    );
    assert_pure(
        indoc! {"
            | h |
            | - |
            |   |"},
        &table("<br>"),
    );
}

/// A `<th>` holding nothing but a `<br>`, the header-row counterpart of the
/// body cells above.
#[test]
fn br_only_header_cell() {
    let html =
        "<table><thead><tr><th><br></th></tr></thead><tbody><tr><td>x</td></tr></tbody></table>";
    assert_faithful(
        indoc! {"
            | <br> |
            | ---- |
            | x    |"},
        html,
    );
    assert_pure(
        indoc! {"
            |   |
            | - |
            | x |"},
        html,
    );
}

/// A `<br>` in a paragraph. With text ahead of it on the line the raw `<br>`
/// stays inline and only the break syntax is unavailable; with nothing ahead of
/// it the whole paragraph has to be serialized, since a bare `<br>` line would
/// parse back as a top-level `<br>`, losing the `<p>` wrapper.
#[test]
fn br_in_paragraph_faithful() {
    assert_faithful("<p><br></p>", "<p><br></p>");
    assert_faithful("a<br>", "<p>a<br></p>");
    // The second break of a run opens a line of its own, which takes the
    // paragraph with it.
    assert_faithful("<p>a<br><br></p>", "<p>a<br><br></p>");
}

#[test]
fn dropped_br_in_pure_mode() {
    assert_pure("", "<p><br></p>");
    assert_pure("a", "<p>a<br></p>");
    assert_pure("a", "<p>a<br><br></p>");
    assert_pure("*   *   x", "<ul><li><br><ul><li>x</li></ul></li></ul>");
}

/// A `<br>` that starts a list item and is followed by a nested list. No break
/// syntax works — a hard break needs inline content after it, not a block — and
/// a raw `<br>` would start the line and swallow the nested list, so the list
/// has to be serialized whole.
#[test]
fn br_before_nested_list_faithful() {
    let html = "<ul><li><br><ul><li>x</li></ul></li></ul>";
    assert_faithful(html, html);
}

// ---------------------------------------------------------------------------
// Whitespace in the source around a `<br>`. HTML collapses it, so it must not
// reach the output as a line of its own or as indentation deep enough to change
// how the following line parses.
// ---------------------------------------------------------------------------

/// A newline before the `<br>` collapses into the break. Under
/// `BrStyle::Backslash` the collapsed space survives ahead of the `\`, which is
/// harmless trailing whitespace, exactly as in the source.
#[test]
fn newline_before_br_in_paragraph() {
    assert_eq!(
        "a  \n*b*",
        faithful("<p>a\n<br><em>b</em></p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "a \\\n*b*",
        faithful("<p>a\n<br><em>b</em></p>", BrStyle::Backslash)
    );
    assert_eq!("a  \nb", faithful("<p>a\n<br>b</p>", BrStyle::TwoSpaces));
    assert_eq!("a \\\nb", faithful("<p>a\n<br>b</p>", BrStyle::Backslash));
}

/// A newline *after* the `<br>` leaves exactly one space of indentation on the
/// continuation line — well under the four-space threshold that would turn it
/// into an indented code block. Deeper source indentation collapses to that same
/// single space, as the tab-indented case below shows.
#[test]
fn newline_after_br_indents_the_continuation_line_by_one_space() {
    for input in ["<p>a\n<br>\nb</p>", "<p>\n\t\ta\n\t\t<br>\n\t\tb\n</p>"] {
        assert_eq!("a  \n b", faithful(input, BrStyle::TwoSpaces), "{input:?}");
        assert_eq!("a \\\n b", faithful(input, BrStyle::Backslash), "{input:?}");
    }
    let emphasis = "<p>\n    a\n    <br>\n    <em>b</em>\n</p>";
    assert_eq!("a  \n *b*", faithful(emphasis, BrStyle::TwoSpaces));
    assert_eq!("a \\\n *b*", faithful(emphasis, BrStyle::Backslash));
}

// ---------------------------------------------------------------------------
// Two-space breaks that get eaten after the fact. Neither of these needs a
// newline in the source — the newline probe merely exposed them.
// ---------------------------------------------------------------------------

/// A list item indents its content and trims each line's end as it goes, which
/// used to erase the two-space break and join the two lines. The indent now
/// spares a hard break, and runs once per item the break sits in, so a break
/// nested inside a `<p>`, a `<blockquote>` or a deeper list has to survive every
/// pass.
#[test]
fn br_inside_list_item() {
    assert_eq!(
        "*   a  \n    b",
        faithful("<ul><li>a<br>b</li></ul>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "*   a  \n    b",
        faithful("<ul><li>a\n<br>b</li></ul>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "1.  a  \n    b",
        faithful("<ol><li>a<br>b</li></ol>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "*   a  \n    b",
        faithful("<ul><li><p>a<br>b</p></li></ul>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "*   > a  \n    > b",
        faithful(
            "<ul><li><blockquote>a<br>b</blockquote></li></ul>",
            BrStyle::TwoSpaces
        )
    );
    assert_eq!(
        "*   *   a  \n        b",
        faithful(
            "<ul><li><ul><li>a<br>b</li></ul></li></ul>",
            BrStyle::TwoSpaces
        )
    );
    // A list nested in a blockquote rather than the other way round: the trim is
    // the item's, so the quote around it changes nothing.
    assert_eq!(
        "> *   a  \n>     b",
        faithful(
            "<blockquote><ul><li>a<br>b</li></ul></blockquote>",
            BrStyle::TwoSpaces
        )
    );
    // An inline element around the break does not cost the two spaces either,
    // so long as its markers stay clear of the break.
    assert_eq!(
        "*   a*b  \n    c*",
        faithful("<ul><li>a<em>b<br>c</em></li></ul>", BrStyle::TwoSpaces)
    );
    // The backslash form has nothing for the trim to take, and never had.
    assert_eq!(
        "*   a\\\n    b",
        faithful("<ul><li>a<br>b</li></ul>", BrStyle::Backslash)
    );
}

/// A blockquote prefixes its lines without trimming their ends, so the break
/// always survived there. It must keep doing so.
#[test]
fn br_inside_blockquote_two_spaces() {
    assert_eq!(
        "> a  \n> b",
        faithful("<blockquote>a<br>b</blockquote>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "> a  \n> b",
        faithful("<blockquote><p>a<br>b</p></blockquote>", BrStyle::TwoSpaces)
    );
}

/// The trim still takes everything that is not a break: whitespace at the end of
/// an item, and whitespace on a line with nothing to break onto after it.
#[test]
fn list_item_line_ends_are_still_trimmed() {
    // Ordinary trailing whitespace, which is not a break.
    assert_eq!(
        "*   a\n*   b",
        faithful("<ul><li>a </li><li>b  </li></ul>", BrStyle::TwoSpaces)
    );
    // A break the item ends on has no line to break onto, so nothing survives of
    // it — the same output the item without the `<br>` gives. (Faithful mode
    // keeps that one as a raw `<br>`; see `br_at_list_item_end`.)
    assert_pure("*   a", "<ul><li>a<br></li></ul>");
    assert_pure("*   a", "<ul><li><p>a<br></p></li></ul>");
}

/// Consecutive `<br>`s would put a whitespace-only line between the two breaks,
/// which is blank and so would end the paragraph instead of breaking it, landing
/// `a` and `b` in separate paragraphs. Every break but the first has nothing
/// ahead of it, so all of them take the backslash fallback.
///
/// Source whitespace between the `<br>`s is dropped rather than landing ahead of
/// the `\`. Whitespace before the run's *first* break stays, being trailing
/// whitespace on a line that holds text; `newline_before_br_in_paragraph` pins
/// it.
#[test]
fn consecutive_br_two_spaces() {
    assert_eq!(
        "a  \n\\\nb",
        faithful("<p>a<br><br>b</p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "a  \n\\\n b",
        faithful("<p>a\n<br>\n<br>\nb</p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "a  \n\\\n\\\n\\\nb",
        faithful("<p>a<br><br><br><br>b</p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "a  \n\\\n\\\n\\\n b",
        faithful("<p>a\n<br>\n<br>\n<br>\n<br>\nb</p>", BrStyle::TwoSpaces)
    );
}

/// The backslash style needs no fallback: a lone `\` line is a hard break, not a
/// blank line. The run's whitespace is dropped the same way regardless.
#[test]
fn consecutive_br_backslash() {
    assert_eq!(
        "a\\\n\\\nb",
        faithful("<p>a<br><br>b</p>", BrStyle::Backslash)
    );
    assert_eq!(
        "a\\\n\\\n\\\n\\\nb",
        faithful("<p>a<br><br><br><br>b</p>", BrStyle::Backslash)
    );
    assert_eq!(
        "a \\\n\\\n\\\n\\\n b",
        faithful("<p>a\n<br>\n<br>\n<br>\n<br>\nb</p>", BrStyle::Backslash)
    );
}

// ---------------------------------------------------------------------------
// Blocks that hold an unwritable `<br>`.
//
// A `<br>` that no syntax reaches is kept only by serializing the block around
// it, and each block has to ask for that itself: `<p>`, `<li>` and
// `<blockquote>` consult `block_holds_unwritable_br`, `<div>` and its kind are
// serialized wholesale in faithful mode regardless, and headings and cells
// always put a marker on the line ahead of the break.
//
// `<blockquote>` asks one question more: the check otherwise stops at a nested
// block, but a nested `<blockquote>` answers by serializing itself, and the
// outer quote's `> ` markers would then wrap an HTML block. So a quote
// serializes itself when a quote nested inside it has to.
// ---------------------------------------------------------------------------

/// A `<div>` holding nothing but a `<br>`, for contrast with the blockquote
/// cases below: faithful mode serializes every `<div>` regardless, so this
/// round-trips without the `<br>` handler being involved at all.
#[test]
fn br_only_div_faithful() {
    assert_faithful("<div><br></div>", "<div><br></div>");
    // The same where the `<br>` is followed by a block: the `<li>` case of
    // `br_before_nested_list_faithful` one element over.
    let html = "<div><br><ul><li>x</li></ul></div>";
    assert_faithful(html, html);
}

/// Pure mode drops the `<br>` and, with it, an empty `<div>`.
#[test]
fn br_only_div_pure() {
    assert_pure("", "<div><br></div>");
    assert_pure("*   x", "<div><br><ul><li>x</li></ul></div>");
}

/// A blockquote holding nothing but a `<br>`, the `<blockquote>` counterpart of
/// `br_in_paragraph_faithful`. The whole quote has to be serialized: a bare
/// `> <br>` line would put the raw `<br>` at the start of the quote's content,
/// where it opens an HTML block.
#[test]
fn br_only_blockquote_faithful() {
    let html = "<blockquote><br></blockquote>";
    assert_faithful(html, html);
    let nested = "<blockquote><blockquote><br></blockquote></blockquote>";
    assert_faithful(nested, nested);
}

/// The same quote followed by a paragraph, which shows the loss is silent: the
/// output is exactly what the document without the blockquote produces.
#[test]
fn br_only_blockquote_before_paragraph_faithful() {
    assert_faithful(
        "<blockquote><br></blockquote>\n\nx",
        "<blockquote><br></blockquote><p>x</p>",
    );
}

/// A `<br>` in a blockquote whose only following content is a block. No break
/// syntax works — a hard break needs inline content after it — and a raw `<br>`
/// would open the quote's first line, so the quote has to be serialized. This is
/// `br_before_nested_list_faithful` with a `<blockquote>` in place of the `<li>`.
#[test]
fn br_before_block_in_blockquote_faithful() {
    let html = "<blockquote><br><p>x</p></blockquote>";
    assert_faithful(html, html);
    let nested = "<blockquote><br><blockquote>x</blockquote></blockquote>";
    assert_faithful(nested, nested);
}

/// A `<br>` that trails a block inside a blockquote. The preceding block ends
/// its own line, so the break starts a line with nothing after it — the same
/// dead end from the other side.
#[test]
fn br_after_block_in_blockquote_faithful() {
    let html = "<blockquote><p>x</p><br></blockquote>";
    assert_faithful(html, html);
    let heading = "<blockquote><h1>a</h1><br></blockquote>";
    assert_faithful(heading, heading);
}

/// Consecutive `<br>`s at the end of a blockquote. The first has text ahead of
/// it and survives as a raw `<br>`; the second has neither text ahead of it nor
/// content after it, so it needs the quote serialized — which is what the
/// paragraph of the same shape gets in `br_in_paragraph_faithful`.
#[test]
fn consecutive_br_at_blockquote_end_faithful() {
    let html = "<blockquote>x<br><br></blockquote>";
    assert_faithful(html, html);
}

/// Where the blockquote *can* write the break it does: these are the neighbours
/// of the cases above and must keep working.
#[test]
fn br_in_blockquote_that_works() {
    // Text ahead of the break on the line, nothing after it in the quote:
    // the raw `<br>` stays inline.
    assert_faithful("> a<br>", "<blockquote>a<br></blockquote>");
    assert_pure("> a", "<blockquote>a<br></blockquote>");
    // Nothing ahead of it, inline content after it: a backslash break.
    assert_faithful("> \\\n> \\\n> x", "<blockquote><br><br>x</blockquote>");
}

/// Every quote above, in pure mode. Each one loses the break and keeps whatever
/// else it held, down to the quote itself where the `<br>` was all of it.
#[test]
fn dropped_br_in_blockquote_pure() {
    assert_pure("", "<blockquote><br></blockquote>");
    assert_pure("", "<blockquote><blockquote><br></blockquote></blockquote>");
    assert_pure("x", "<blockquote><br></blockquote><p>x</p>");
    assert_pure("> x", "<blockquote><br><p>x</p></blockquote>");
    assert_pure(
        "> > x",
        "<blockquote><br><blockquote>x</blockquote></blockquote>",
    );
    assert_pure("> x", "<blockquote><p>x</p><br></blockquote>");
    assert_pure("> # a", "<blockquote><h1>a</h1><br></blockquote>");
    assert_pure("> x", "<blockquote>x<br><br></blockquote>");
}

// ---------------------------------------------------------------------------
// A `<br>` directly at the document root. Nothing needs serializing here: a raw
// `<br>` on a line of its own is an HTML block that parses back as exactly that
// `<br>`. This is the one place a line-opening raw `<br>` is safe, since the
// block it opens ends at the blank line separating it from what follows.
// ---------------------------------------------------------------------------

#[test]
fn br_at_document_root_faithful() {
    assert_faithful("<br>", "<br>");
    assert_faithful("a\n\n<br>", "<p>a</p><br>");
    assert_faithful("a\n\n<br>\n\nb", "<p>a</p><br><p>b</p>");
    // Each of a run gets its own line and its own blank line, since two on one
    // line would be an HTML block holding both and two on adjacent lines would
    // be one block holding both.
    assert_faithful("<br>\n\n<br>", "<br><br>");
}

/// Pure mode drops it, leaving what the document without the `<br>` produces.
#[test]
fn br_at_document_root_pure() {
    assert_pure("", "<br>");
    assert_pure("", "<br><br>");
    assert_pure("a\n\nb", "<p>a</p><br><p>b</p>");
}

/// A root-level `<br>` followed by inline content needs no HTML at all: the two
/// become a paragraph with a backslash break, which is plain Markdown and so
/// reads the same in both modes.
#[test]
fn br_at_document_root_before_text() {
    assert_faithful("\\\nx", "<br>x");
    assert_pure("\\\nx", "<br>x");
}

// ---------------------------------------------------------------------------
// An inline element whose *only* content is a `<br>`. Two separate things go
// wrong, and only elements htmd writes with *markers* — `<em>`/`<i>` as `*`,
// `<strong>`/`<b>` as `**` — hit either; one htmd has no handler for is
// serialized whole in faithful mode, which sidesteps both (see
// `br_alone_in_untranslated_inline_element_faithful`).
//
// First, neither break syntax reaches inside the markers: two spaces are
// whitespace, which a marker cannot flank against, and a backslash leaves `*\*`,
// a literal `*` and an escaped one.
//
// Second — and this outlives any fix to the first — the raw `<br>` between the
// markers fails the flanking rule of point 3 in the module docs. In `a*<br>*b`
// the opening `*` sits between the alphanumeric `a` and the punctuation `<`, so
// it is not left-flanking, and the closing `*` fails the mirror test. Only
// `*<br>*` alone between whitespace or block boundaries parses.
//
// So the markers are written only where they flank; everywhere else faithful
// mode serializes the element and pure mode moves the break out from between the
// markers. Link labels have no flanking rule, so `[<br>](u)` works in every
// position (`br_alone_in_link_faithful`), and an element holding a `<br>` *and*
// something else is mostly fine (`br_inside_inline_element`).
// ---------------------------------------------------------------------------

/// The inline elements htmd writes with emphasis markers: `*` for the first
/// two, `**` for the last two.
const EMPHASIS: [&str; 4] = ["em", "i", "strong", "b"];

/// The inline elements htmd has no handler for at all, which faithful mode
/// therefore serializes whole. No Markdown marker is ever written next to a
/// break inside one of these.
const UNTRANSLATED_INLINE: [&str; 10] = [
    "del", "s", "sup", "sub", "mark", "u", "small", "q", "cite", "kbd",
];

#[test]
fn br_alone_in_inline_element_faithful() {
    for tag in EMPHASIS {
        // Text on both sides. The markers cannot be used at all here — a `*`
        // between `a` and `<` is not left-flanking — so the element is
        // serialized and the raw `<br>` rides along inside it.
        assert_faithful(
            &format!("a<{tag}><br></{tag}>b"),
            &format!("<p>a<{tag}><br></{tag}>b</p>"),
        );
        // Nothing ahead of it. The serialized element opens the line, but an
        // inline tag with content after it on the same line is not an HTML
        // block, so this stays a paragraph.
        assert_faithful(
            &format!("<{tag}><br></{tag}>x"),
            &format!("<p><{tag}><br></{tag}>x</p>"),
        );
        // Nothing after it.
        assert_faithful(
            &format!("x<{tag}><br></{tag}>"),
            &format!("<p>x<{tag}><br></{tag}></p>"),
        );
    }
}

/// Pure mode writes the break outside the markers, which leaves the element
/// holding nothing and so writing nothing. The break itself survives, since it
/// renders the same just inside an `<em>` as just before it. Only where no
/// syntax can carry it at all — nothing after it in the block, as in the third
/// case — is it dropped.
#[test]
fn br_alone_in_inline_element_pure() {
    for tag in EMPHASIS {
        assert_eq!(
            "a  \nb",
            pure(&format!("<p>a<{tag}><br></{tag}>b</p>"), BrStyle::TwoSpaces)
        );
        assert_eq!(
            "a\\\nb",
            pure(&format!("<p>a<{tag}><br></{tag}>b</p>"), BrStyle::Backslash)
        );
        // Nothing ahead of the break on the line, so the two-space form falls
        // back to the backslash one in both styles.
        assert_pure("\\\nx", &format!("<p><{tag}><br></{tag}>x</p>"));
        // Nothing after it in the block: no break syntax works, and pure mode
        // has no HTML to fall back on, so the break goes.
        assert_pure("x", &format!("<p>x<{tag}><br></{tag}></p>"));
    }
}

/// Nesting changes nothing: the markers still end up against the break, so both
/// elements are serialized — the inner one because its own markers would sit
/// against the break, the outer one because its markers would then sit against
/// the inner element's tags. Pure mode moves the break out through both.
#[test]
fn br_alone_in_nested_inline_elements() {
    assert_faithful(
        "a<em><strong><br></strong></em>b",
        "<p>a<em><strong><br></strong></em>b</p>",
    );
    assert_eq!(
        "a  \nb",
        pure(
            "<p>a<em><strong><br></strong></em>b</p>",
            BrStyle::TwoSpaces
        )
    );
    assert_eq!(
        "a\\\nb",
        pure(
            "<p>a<em><strong><br></strong></em>b</p>",
            BrStyle::Backslash
        )
    );
}

/// Two of them in a row. Faithful mode serializes both elements as it does for
/// one; pure mode moves both breaks out, leaving the elements with nothing to
/// wrap, and spells them as it would with no `<em>`s in the way — the same pair
/// `basic_tests::br` pins for a bare `<br><br>`.
#[test]
fn consecutive_br_alone_in_inline_elements() {
    let html = "<p>a<em><br></em><em><br></em>b</p>";
    assert_faithful("a<em><br></em><em><br></em>b", html);
    assert_eq!("a  \n\\\nb", pure(html, BrStyle::TwoSpaces));
    assert_eq!("a\\\n\\\nb", pure(html, BrStyle::Backslash));
}

/// The same `<em><br></em>` in every other block. A heading or a cell puts a
/// marker on the line first, so the raw `<br>` reaches the output there — but
/// the flanking rule does not care what block the emphasis sits in. `*<br>*` as
/// a heading's or a cell's whole content is the one case that parses;
/// `br_inside_inline_element` keeps it.
#[test]
fn br_alone_in_inline_element_in_other_blocks_faithful() {
    assert_faithful("# a<em><br></em>b", "<h1>a<em><br></em>b</h1>");
    assert_faithful("# <em><br></em>a", "<h1><em><br></em>a</h1>");
    assert_faithful("*   a<em><br></em>b", "<ul><li>a<em><br></em>b</li></ul>");
    assert_faithful("*   <em><br></em>x", "<ul><li><em><br></em>x</li></ul>");
    assert_faithful(
        "> a<em><br></em>b",
        "<blockquote>a<em><br></em>b</blockquote>",
    );
    assert_faithful(
        "> <em><br></em>x",
        "<blockquote><em><br></em>x</blockquote>",
    );
    assert_faithful(
        indoc! {"
            | h               |
            | --------------- |
            | a<em><br></em>b |"},
        &table("a<em><br></em>b"),
    );
}

/// The same blocks in pure mode, where the emptied `<em>` vanishes and the
/// break is left to whatever the block can carry: nothing in a heading or a
/// cell, an ordinary hard break in a list item or a quote.
#[test]
fn br_alone_in_inline_element_in_other_blocks_pure() {
    assert_pure("# ab", "<h1>a<em><br></em>b</h1>");
    assert_pure("# a", "<h1><em><br></em>a</h1>");
    assert_eq!(
        "*   a  \n    b",
        pure("<ul><li>a<em><br></em>b</li></ul>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "*   a\\\n    b",
        pure("<ul><li>a<em><br></em>b</li></ul>", BrStyle::Backslash)
    );
    assert_pure("*   \\\n    x", "<ul><li><em><br></em>x</li></ul>");
    assert_eq!(
        "> a  \n> b",
        pure(
            "<blockquote>a<em><br></em>b</blockquote>",
            BrStyle::TwoSpaces
        )
    );
    assert_eq!(
        "> a\\\n> b",
        pure(
            "<blockquote>a<em><br></em>b</blockquote>",
            BrStyle::Backslash
        )
    );
    assert_pure("> \\\n> x", "<blockquote><em><br></em>x</blockquote>");
    assert_pure(
        indoc! {"
            | h  |
            | -- |
            | ab |"},
        &table("a<em><br></em>b"),
    );
}

/// A `<br>` alone in a *link*, the one marked-up inline element the flanking
/// rule spares: `[<br>](u)` parses as a link holding a break wherever it lands.
#[test]
fn br_alone_in_link_faithful() {
    assert_faithful("a[<br>](u)b", "<p>a<a href=\"u\"><br></a>b</p>");
    assert_faithful("[<br>](u)x", "<p><a href=\"u\"><br></a>x</p>");
}

/// Pure mode drops the break but keeps the link, which still has an `href` to
/// carry.
#[test]
fn br_alone_in_link_pure() {
    assert_pure("a[](u)b", "<p>a<a href=\"u\"><br></a>b</p>");
    assert_pure("x[](u)", "<p>x<a href=\"u\"><br></a></p>");
}

/// A `<br>` that *starts* a link's content, with more content after it inside
/// the link. A hard break works inside a link label, but the two-space form
/// cannot be the first thing in one — the label's leading whitespace is
/// stripped — so both styles fall back to the backslash. That is plain Markdown,
/// so both modes agree.
#[test]
fn br_at_link_start() {
    assert_faithful("a[\\\nc](u)b", "<p>a<a href=\"u\"><br>c</a>b</p>");
    assert_pure("a[\\\nc](u)b", "<p>a<a href=\"u\"><br>c</a>b</p>");
}

/// A `<br>` that *ends* a link's content, the link's answer to
/// `br_at_inline_element_end`. No break syntax reaches there, but the label has
/// no flanking rule and its `[` has already opened the line, so the raw `<br>`
/// stays inside the brackets — where the `<em>` of the same shape has to be
/// serialized whole.
#[test]
fn br_at_link_end() {
    assert_faithful("a[c<br>](u)b", "<p>a<a href=\"u\">c<br></a>b</p>");
    assert_faithful("[c<br>](u)", "<p><a href=\"u\">c<br></a></p>");
    assert_pure("a[c](u)b", "<p>a<a href=\"u\">c<br></a>b</p>");
    assert_pure("[c](u)", "<p><a href=\"u\">c<br></a></p>");
}

/// An inline element faithful mode serializes whole rather than writing with
/// Markdown markers — every inline tag htmd has no handler for, plus `<span>`,
/// whose handler serializes in faithful mode anyway. No marker lands next to the
/// break, so nothing here turns on the flanking rule: the contrast showing that
/// the emphasis cases above are the markers' doing, not the `<br>`'s.
///
/// The break inside is a raw `<br>` because the element around it is HTML, which
/// keeps the element on one line and the output independent of `BrStyle`.
#[test]
fn br_alone_in_untranslated_inline_element_faithful() {
    // `<span>` joins them here: its handler serializes in faithful mode, which
    // is what having no handler at all gets the rest.
    for tag in UNTRANSLATED_INLINE.into_iter().chain(["span"]) {
        assert_faithful(
            &format!("a<{tag}><br></{tag}>b"),
            &format!("<p>a<{tag}><br></{tag}>b</p>"),
        );
    }
}

/// Pure mode strips those elements and keeps the break, which is representable
/// here. `<span>` has a handler where the others have none, but in pure mode it
/// writes nothing of its own, so the break comes through the same way.
#[test]
fn br_alone_in_untranslated_inline_element_pure() {
    for tag in UNTRANSLATED_INLINE.into_iter().chain(["span"]) {
        assert_eq!(
            "a  \nb",
            pure(&format!("<p>a<{tag}><br></{tag}>b</p>"), BrStyle::TwoSpaces),
            "{tag}"
        );
        assert_eq!(
            "a\\\nb",
            pure(&format!("<p>a<{tag}><br></{tag}>b</p>"), BrStyle::Backslash),
            "{tag}"
        );
    }
}

/// An inline element holding a `<br>` alongside other content, which works: the
/// break is written where it falls, inside the markers when the element has
/// content after it and outside them when it does not.
#[test]
fn br_inside_inline_element() {
    // The paragraph's only content, so the paragraph is serialized whole.
    assert_faithful("<p><em><br></em></p>", "<p><em><br></em></p>");
    assert_faithful(
        "<p><a href=\"u\"><br></a></p>",
        "<p><a href=\"u\"><br></a></p>",
    );
    // Content after the break inside the element: an ordinary hard break. The
    // markers are at the block's edges here, so they flank cleanly.
    assert_faithful("*\\\na*", "<p><em><br>a</em></p>");
    // A heading or a cell whose whole content is the element: `*<br>*` between
    // the block's boundaries is the one spelling that parses.
    assert_faithful("# *<br>*", "<h1><em><br></em></h1>");
    // A break *between* content inside the element keeps the markers away from
    // the break's punctuation, so both styles are fine in every position.
    assert_eq!(
        "*a  \nb*",
        faithful("<p><em>a<br>b</em></p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "*a\\\nb*",
        faithful("<p><em>a<br>b</em></p>", BrStyle::Backslash)
    );
    assert_eq!(
        "a*b  \nc*d",
        faithful("<p>a<em>b<br>c</em>d</p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "a*b\\\nc*d",
        faithful("<p>a<em>b<br>c</em>d</p>", BrStyle::Backslash)
    );
    // A break inside a link label, likewise.
    assert_eq!(
        "a[c  \nd](u)b",
        faithful("<p>a<a href=\"u\">c<br>d</a>b</p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "a[c\\\nd](u)b",
        faithful("<p>a<a href=\"u\">c<br>d</a>b</p>", BrStyle::Backslash)
    );
}

/// A `<br>` that *starts* an inline element which has content after it — the
/// one position `br_inside_inline_element` cannot reach, since the break has to
/// be written immediately after the opening marker.
///
/// Neither spelling survives there. `a*\` puts the `*` between `a` and `\`,
/// which is not left-flanking; `a*  ` puts it against whitespace, which cannot
/// open emphasis at all. Writing the break *outside* the markers renders
/// identically but moves the `<br>` out of the `<em>`, which faithful mode may
/// not do — so the element is serialized. Pure mode, which may, is in
/// `br_at_inline_element_start_pure`.
#[test]
fn br_at_inline_element_start_faithful() {
    assert_faithful("a<em><br>b</em>", "<p>a<em><br>b</em></p>");
    // The same in a list item, where the break is inside a tag and so leaves the
    // item's indent no two-space break to spare. Compare the emphasis *around* a
    // break in `br_inside_list_item`, which keeps its two spaces.
    assert_faithful("*   a<em><br>b</em>", "<ul><li>a<em><br>b</em></li></ul>");
}

/// The same shape in pure mode, where moving the break out from between the
/// markers is allowed and keeps it representable.
#[test]
fn br_at_inline_element_start_pure() {
    assert_eq!(
        "a  \n*b*",
        pure("<p>a<em><br>b</em></p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "a\\\n*b*",
        pure("<p>a<em><br>b</em></p>", BrStyle::Backslash)
    );
}

/// A `<br>` that *ends* an inline element which has content after it in the
/// block — the mirror of `br_at_inline_element_start_faithful`. A closing marker
/// against a break never works: the content ends with the break's newline, and a
/// marker preceded by whitespace does not close emphasis. The backslash spelling
/// is worse than inert — `*a\*` escapes the closing marker outright, leaving the
/// emphasis unterminated. So faithful mode serializes here too, while pure mode
/// moves the break past the closing marker.
#[test]
fn br_at_inline_element_end() {
    assert_faithful("<em>a<br></em>b", "<p><em>a<br></em>b</p>");
    assert_faithful("a<em>b<br></em>c", "<p>a<em>b<br></em>c</p>");
    // With nothing after it in the block the break cannot be written at all, so
    // the raw `<br>` is what the element holds — and a marker may close against
    // the `>` of a tag when the block ends there, since the block edge counts
    // as whitespace.
    assert_faithful("*a<br>*", "<p><em>a<br></em></p>");
    assert_eq!(
        "*a*  \nb",
        pure("<p><em>a<br></em>b</p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "*a*\\\nb",
        pure("<p><em>a<br></em>b</p>", BrStyle::Backslash)
    );
}

// ---------------------------------------------------------------------------
// A `<br>` inside a `<code>`. Which kind of `<code>` decides everything: a code
// span is written between one pair of backticks on a single line and cannot
// hold a newline at all, while a code block's lines carry real ones.
// ---------------------------------------------------------------------------

/// A `<br>` alone in a code span. Faithful mode already serializes any `<code>`
/// whose children are not all text, so it round-trips.
#[test]
fn br_alone_in_code_faithful() {
    assert_faithful("a<code><br></code>b", "<p>a<code><br></code>b</p>");
    let html = "<p><code><br></code></p>";
    assert_faithful(html, html);
}

/// A code span cannot hold a line break at all — CommonMark turns a newline
/// inside one into a space — so pure mode drops the `<br>` as it does in a table
/// cell.
///
/// For a span the `<br>` was all of, that leaves an empty pair of backticks,
/// which CommonMark reads as two literal characters rather than an empty code
/// span. The `<br>` is not what breaks this: the second case pins the same
/// output from `<br>`-less source.
#[test]
fn br_alone_in_code_pure() {
    assert_pure("a``b", "<p>a<code><br></code>b</p>");
    assert_pure("a``b", "<p>a<code></code>b</p>");
    assert_pure("``", "<p><code><br></code></p>");
}

/// A `<br>` in a code span that holds text as well. Pure mode drops it and the
/// text closes up — `xy`, not `x y` — since the span has no newline to put
/// there and nothing else to stand in for one.
#[test]
fn br_inside_code_span() {
    assert_faithful("a<code>x<br>y</code>b", "<p>a<code>x<br>y</code>b</p>");
    assert_faithful("a<code>x<br></code>b", "<p>a<code>x<br></code>b</p>");
    assert_pure("a`xy`b", "<p>a<code>x<br>y</code>b</p>");
    assert_pure("a`x`b", "<p>a<code>x<br></code>b</p>");
}

/// A `<br>` in a `<pre><code>` code block, which faithful mode serializes for
/// the same reason it serializes a code span: a `<code>` with a child that is
/// not text is written as HTML.
#[test]
fn br_in_code_block_faithful() {
    for html in [
        "<pre><code>a<br>b</code></pre>",
        "<pre><code><br>a</code></pre>",
        "<pre><code>a<br></code></pre>",
        "<pre><code><br></code></pre>",
    ] {
        assert_faithful(html, html);
    }
}

/// A code block's lines carry real newlines, so the break is simply one. Break
/// syntax must not reach here: inside a fence it would land in the output as
/// *code*, not as a break. Nothing here depends on `BrStyle`.
#[test]
fn br_in_code_block_pure() {
    assert_pure("```\na\nb\n```", "<pre><code>a<br>b</code></pre>");
    assert_pure("```\n\na\n```", "<pre><code><br>a</code></pre>");
    assert_pure("```\na\n```", "<pre><code>a<br></code></pre>");
    assert_pure("```\na\n\nb\n```", "<pre><code>a<br><br>b</code></pre>");
    assert_pure("```\n\n```", "<pre><code><br></code></pre>");
    // The language survives, and so does the indented spelling of a block.
    assert_pure(
        "```rs\na\nb\n```",
        "<pre><code class=\"language-rs\">a<br>b</code></pre>",
    );
    let indented = HtmlToMarkdown::builder()
        .options(Options {
            translation_mode: TranslationMode::Pure,
            code_block_style: CodeBlockStyle::Indented,
            ..Default::default()
        })
        .build();
    assert_eq!(
        "    a\n    b",
        indented.convert("<pre><code>a<br>b</code></pre>").unwrap()
    );
    // Which is to say: a `<br>` in a code block gives what a source newline in
    // the same place gives.
    for style in STYLES {
        assert_eq!(
            pure("<pre><code>a\nb</code></pre>", style),
            pure("<pre><code>a<br>b</code></pre>", style)
        );
    }
}

// ---------------------------------------------------------------------------
// How the `<br>` itself is written in the source, which is the parser's business
// rather than the break's — except in faithful mode, where the raw `<br>` it
// falls back to has to carry whatever the source `<br>` held.
// ---------------------------------------------------------------------------

/// The self-closing spellings are the same element, so they give the same
/// output as `<br>`.
#[test]
fn br_spellings_are_equivalent() {
    for html in ["<p>a<br/>b</p>", "<p>a<br />b</p>", "<p>a<BR>b</p>"] {
        for style in STYLES {
            assert_eq!(faithful("<p>a<br>b</p>", style), faithful(html, style));
            assert_eq!(pure("<p>a<br>b</p>", style), pure(html, style));
        }
    }
}

/// A `<br>` with attributes. Faithful mode writes them out wherever it falls
/// back to the raw tag; pure mode drops them along with everything else Markdown
/// has no place for.
#[test]
fn br_attributes() {
    assert_faithful("a<br class=\"x\">b", "<p>a<br class=\"x\">b</p>");
    assert_faithful("<p><br class=\"x\"></p>", "<p><br class=\"x\"></p>");
    assert_faithful("# <br class=\"x\">", "<h1><br class=\"x\"></h1>");
    assert_eq!(
        "a  \nb",
        pure("<p>a<br class=\"x\">b</p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "a\\\nb",
        pure("<p>a<br class=\"x\">b</p>", BrStyle::Backslash)
    );
    assert_pure("", "<p><br class=\"x\"></p>");
    assert_pure("#", "<h1><br class=\"x\"></h1>");
}

// ---------------------------------------------------------------------------
// Positions that already work, kept here so a fix to the cases above does not
// quietly cost them.
// ---------------------------------------------------------------------------

/// A list item holding nothing but a `<br>`, in either list type. The `<li>`
/// reports itself untranslated and the whole list is serialized.
#[test]
fn br_only_list_item_faithful() {
    for html in [
        "<ul><li><br></li></ul>",
        "<ol><li><br></li></ol>",
        "<ul><li>a</li><li><br></li></ul>",
        // A `<br>` trailing a block inside the item: the `<blockquote>` case of
        // `br_after_block_in_blockquote_faithful`, which the `<li>` gets right.
        "<ul><li><p>x</p><br></li></ul>",
    ] {
        assert_faithful(html, html);
    }
}

/// Pure mode empties the item instead, leaving a bare list marker.
#[test]
fn br_only_list_item_pure() {
    assert_pure("*", "<ul><li><br></li></ul>");
    assert_pure("1.", "<ol><li><br></li></ol>");
    assert_pure("*   a\n*", "<ul><li>a</li><li><br></li></ul>");
    assert_pure("*   x", "<ul><li><p>x</p><br></li></ul>");
    // With a block after the list the marker keeps the padding it would have
    // written content into — trailing whitespace on a line CommonMark reads as
    // an empty item either way.
    assert_pure("*   \n\nx", "<ul><li><br></li></ul><p>x</p>");
}

/// A `<br>` that ends a list item, and one that separates the item's text from a
/// nested list. Both have text ahead of them on the line, so the raw `<br>` stays
/// inline and the item needs no serializing.
#[test]
fn br_at_list_item_end() {
    assert_faithful("*   a<br>", "<ul><li>a<br></li></ul>");
    assert_faithful(
        "*   a<br>\n    *   x",
        "<ul><li>a<br><ul><li>x</li></ul></li></ul>",
    );
    // Pure mode drops the break; `list_item_line_ends_are_still_trimmed` has the
    // first of these, which is where the trim it turns on is pinned.
    assert_pure(
        "*   a\n    *   x",
        "<ul><li>a<br><ul><li>x</li></ul></li></ul>",
    );
}

/// A `<br>` at either *edge* of a Setext heading. At the end, text already opens
/// the line, so the raw `<br>` stays inline and the underline still has a line
/// to attach to; at the start there is no such line, so the heading falls back
/// to ATX.
#[test]
fn br_at_setext_heading_edges_faithful() {
    assert_faithful_setext("a<br>\n=====", "<h1>a<br></h1>");
    assert_faithful_setext("a<br>\n-----", "<h2>a<br></h2>");
    assert_faithful_setext("# <br>a", "<h1><br>a</h1>");
    assert_faithful_setext("## <br>a", "<h2><br>a</h2>");
}

/// Pure mode drops the break and writes the heading from what is left, so the
/// ATX fallback is no longer needed and both edges give the same answer.
#[test]
fn br_at_setext_heading_edges_pure() {
    assert_pure_setext("a\n=", "<h1>a<br></h1>");
    assert_pure_setext("a\n-", "<h2>a<br></h2>");
    assert_pure_setext("a\n=", "<h1><br>a</h1>");
    assert_pure_setext("a\n-", "<h2><br>a</h2>");
}

/// A heading inside a blockquote still writes its own marker first, so the raw
/// `<br>` stays inline there too.
#[test]
fn br_in_heading_inside_blockquote() {
    assert_faithful("> # <br>", "<blockquote><h1><br></h1></blockquote>");
    assert_faithful("> # a<br>b", "<blockquote><h1>a<br>b</h1></blockquote>");
    assert_pure("> #", "<blockquote><h1><br></h1></blockquote>");
    assert_pure("> # ab", "<blockquote><h1>a<br>b</h1></blockquote>");
}

// ---------------------------------------------------------------------------
// A `<br>` in a table cell whose content is wrapped in a block. A cell answers
// for every break under it, however deeply nested: were the break left to the
// enclosing `<p>`, that `<p>` would write an ordinary hard break, whose newline
// is then flattened into the cell's single line, leaving the break syntax behind
// as literal text.
//
// Faithful mode never reaches this — a cell holding block elements is serialized
// as HTML — so only pure mode exercises it.
// ---------------------------------------------------------------------------

#[test]
fn br_in_wrapped_table_cell_faithful() {
    for cell in ["<p>a<br>b</p>", "<ul><li>a<br>b</li></ul>", "<p><br></p>"] {
        let html = table(cell);
        assert_faithful(&html, &html);
    }
}

/// The cell cannot hold a newline whatever wraps its content, so pure mode owes
/// the same dropped `<br>` it gives an unwrapped cell in `br_in_table_cell`.
#[test]
fn dropped_br_in_wrapped_table_cell_pure() {
    assert_pure(
        indoc! {"
            | h  |
            | -- |
            | ab |"},
        &table("<p>a<br>b</p>"),
    );
    assert_pure(
        indoc! {"
            | h      |
            | ------ |
            | *   ab |"},
        &table("<ul><li>a<br>b</li></ul>"),
    );
}

// ---------------------------------------------------------------------------
// The remaining branches: elements the scans single out by name, and the
// enclosing blocks that write their content some other way. Each of these is a
// path the handlers take a deliberate decision about, so each gets a shape.
// ---------------------------------------------------------------------------

/// An `<img>` is childless, so the scans cannot find content by recursing into
/// it — yet it does write a link to the line, which is why both name it. An
/// `<img>` before the break makes the raw `<br>` safe; one after it gives a hard
/// break something to break onto.
#[test]
fn br_next_to_an_img() {
    // Nothing follows the break, so it falls back to the raw `<br>` — which the
    // image ahead of it makes safe. Pure mode drops it and keeps the image.
    assert_faithful("![](i)<br>", r#"<p><img src="i"><br></p>"#);
    assert_pure("![](i)", r#"<p><img src="i"><br></p>"#);

    // The image after the break is content to break onto, so a break is
    // written; nothing precedes it on the line, so it is the backslash form.
    for expected_style in STYLES {
        assert_eq!(
            "\\\n![](i)",
            faithful(r#"<p><br><img src="i"></p>"#, expected_style)
        );
    }
    assert_pure("\\\n![](i)", r#"<p><br><img src="i"></p>"#);

    // An ordinary break, with the image only supplying the line's content.
    assert_eq!(
        "a![](i)  \nb",
        faithful(r#"<p>a<img src="i"><br>b</p>"#, BrStyle::TwoSpaces)
    );
    assert_eq!(
        "a![](i)\\\nb",
        faithful(r#"<p>a<img src="i"><br>b</p>"#, BrStyle::Backslash)
    );
}

/// A `<br>` with an inline element on each side. The scans have to see through
/// the element to the text inside it: an `<em>` writes markers, but the content
/// that decides the break is the text they wrap.
#[test]
fn br_between_inline_elements() {
    let html = "<p>a<em>b</em><br><em>c</em>d</p>";
    assert_eq!("a*b*  \n*c*d", faithful(html, BrStyle::TwoSpaces));
    assert_eq!("a*b*\\\n*c*d", faithful(html, BrStyle::Backslash));
    assert_eq!("a*b*  \n*c*d", pure(html, BrStyle::TwoSpaces));
    assert_eq!("a*b*\\\n*c*d", pure(html, BrStyle::Backslash));
}

/// An `<a>` with no `href` is not written as a link — it has no destination to
/// write one with — so it puts no `[` on the line and walls nothing off. The
/// break inside it is written exactly as it would be with no `<a>` there at
/// all, unlike the `href` case in `br_alone_in_link_faithful`.
#[test]
fn br_in_an_anchor_without_href() {
    assert_eq!(
        "a  \nb",
        faithful("<p>a<a><br></a>b</p>", BrStyle::TwoSpaces)
    );
    assert_eq!(
        "a\\\nb",
        faithful("<p>a<a><br></a>b</p>", BrStyle::Backslash)
    );
    assert_eq!("a  \nb", pure("<p>a<a><br></a>b</p>", BrStyle::TwoSpaces));
    assert_eq!("a\\\nb", pure("<p>a<a><br></a>b</p>", BrStyle::Backslash));

    // With nothing on either side the break is unwritable, and the paragraph is
    // serialized whole — the `<a>` no more able to carry it than a `<span>`.
    assert_faithful("<p><a><br></a></p>", "<p><a><br></a></p>");
    assert_pure("", "<p><a><br></a></p>");
}

/// A `<code>` answers for every break under it however deeply nested, so the
/// walk that finds it has to climb through inline elements. Faithful mode never
/// consults that walk — a `<code>` whose children are not all text is serialized
/// whole — so only pure mode reaches it.
#[test]
fn br_in_a_code_span_through_an_inline_element() {
    assert_faithful(
        "a<code><em><br></em></code>b",
        "<p>a<code><em><br></em></code>b</p>",
    );
    assert_pure("a``b", "<p>a<code><em><br></em></code>b</p>");
}

/// A list that serializes itself inside a blockquote, reaching by another route
/// the shape the nested-`<blockquote>` rule is built for. Here the `> ` markers
/// may wrap the HTML block after all: a `<ul>` written as HTML is a single line,
/// and CommonMark reads a quoted HTML block back into the quote.
#[test]
fn br_only_list_item_in_blockquote_faithful() {
    assert_faithful(
        "> <ul><li><br></li></ul>",
        "<blockquote><ul><li><br></li></ul></blockquote>",
    );
    assert_pure("> *", "<blockquote><ul><li><br></li></ul></blockquote>");

    // With content ahead of it the break is writable as a raw `<br>`, so the
    // list stays Markdown and only the break is HTML.
    assert_faithful(
        "> *   a<br>",
        "<blockquote><ul><li>a<br></li></ul></blockquote>",
    );
    assert_pure(
        "> *   a",
        "<blockquote><ul><li>a<br></li></ul></blockquote>",
    );
}

/// A `<pre>` with no `<code>` inside it. The code walk keys on `<code>`, so it
/// finds nothing here and the break is written as an ordinary one — safe only
/// because faithful mode serializes the `<pre>` whole first. Pure mode drops the
/// preformatting entirely, as it does for any `<pre>`.
#[test]
fn br_in_pre_without_code() {
    assert_faithful("<pre>a<br>b</pre>", "<pre>a<br>b</pre>");
    assert_eq!("a  \nb", pure("<pre>a<br>b</pre>", BrStyle::TwoSpaces));
    assert_eq!("a\\\nb", pure("<pre>a<br>b</pre>", BrStyle::Backslash));

    assert_faithful("<pre><br></pre>", "<pre><br></pre>");
    assert_pure("", "<pre><br></pre>");
}

/// The break inside a code block is a real newline under every way of writing
/// the block. The two styles part company only on a block that *opens* with a
/// break: a fence keeps the empty first line, while indenting drops it, since a
/// run of spaces is not a line an indented block can begin with.
#[test]
fn br_in_code_block_under_every_style() {
    let convert_code = |html: &str, mode, style, fence| {
        HtmlToMarkdown::builder()
            .options(Options {
                translation_mode: mode,
                code_block_style: style,
                code_block_fence: fence,
                ..Default::default()
            })
            .build()
            .convert(html)
            .unwrap()
    };
    for (style, fence, interior, leading) in [
        (
            CodeBlockStyle::Indented,
            CodeBlockFence::Backticks,
            "    a\n    b",
            "    \n    a",
        ),
        (
            CodeBlockStyle::Fenced,
            CodeBlockFence::Tildes,
            "~~~\na\nb\n~~~",
            "~~~\n\na\n~~~",
        ),
    ] {
        for (html, expected) in [
            ("<pre><code>a<br>b</code></pre>", interior),
            ("<pre><code><br>a</code></pre>", leading),
        ] {
            assert_eq!(
                expected,
                convert_code(html, TranslationMode::Pure, style, fence),
                "{html:?}, {style:?}, {fence:?}"
            );
            assert_eq!(
                html,
                convert_code(html, TranslationMode::Faithful, style, fence),
                "{html:?}, {style:?}, {fence:?}"
            );
        }
    }
}

/// A `<caption>` is an ordinary block so far as the break is concerned: it is
/// not a cell, so the break inside it is spelled the usual way. htmd writes the
/// caption as a paragraph ahead of the table, `<br>` or no `<br>`.
#[test]
fn br_in_table_caption() {
    let html = "<table><caption>a<br>b</caption><thead><tr><th>h</th></tr></thead>\
                <tbody><tr><td>x</td></tr></tbody></table>";
    let expected = |br: &str| format!("a{br}\nb\n| h |\n| - |\n| x |");
    assert_eq!(expected("  "), faithful(html, BrStyle::TwoSpaces));
    assert_eq!(expected("\\"), faithful(html, BrStyle::Backslash));
    assert_eq!(expected("  "), pure(html, BrStyle::TwoSpaces));
}

/// A `<br>` alone in a Setext heading, but wrapped in emphasis. The bare case
/// falls back to ATX (`br_only_heading_faithful`) because the raw `<br>` would
/// open the line and swallow the underline; here the `*` opens it instead, and
/// the markers flank because the whole line is the emphasis.
#[test]
fn br_in_emphasis_in_setext_heading() {
    assert_faithful_setext("*<br>*\n======", "<h1><em><br></em></h1>");
    assert_pure_setext("", "<h1><em><br></em></h1>");
}

/// A backslash break that ends up alone on a Setext heading's first line. The
/// underline attaches to the whole paragraph above it, so Setext still works,
/// and reaching for ATX instead would split the heading.
///
/// It takes an `<img>` with no `src` to get here: `scan_back` counts any `<img>`
/// as content on the line, so the break is spelled for a line with text ahead of
/// it, and then the image writes nothing.
#[test]
fn br_after_an_img_that_writes_nothing_in_a_setext_heading() {
    assert_eq!(
        "\\\nx\n===",
        faithful_setext("<h1><img><br>x</h1>", BrStyle::Backslash)
    );
    // Under two spaces the break is lost instead: the heading trims leading
    // document whitespace, and the two spaces opening the content are exactly
    // that. Pinned as it stands, not as it should be.
    assert_eq!(
        "x\n=",
        faithful_setext("<h1><img><br>x</h1>", BrStyle::TwoSpaces)
    );
}

// ---------------------------------------------------------------------------
// Invariants that must hold over *every* shape in this file, whatever spelling
// the cases above settle on. These four sweep `CORPUS`, so a shape added there
// is checked by all of them.
//
// `faithful_output_round_trips` is the property the other three are each a piece
// of. They stay because they name the failure — the round-trip check reports
// only that two trees differ — and because they cover pure mode, which the
// round-trip check does not apply to.
// ---------------------------------------------------------------------------

/// Every `<br>` shape this file exercises, minus the root-level ones, which
/// `ROOT_CORPUS` holds: a raw `<br>` alone at the document root legitimately
/// opens a line, the one exception to the first invariant below.
///
/// The shapes that vary over a list — every heading level, every tag in
/// `EMPHASIS` and `UNTRANSLATED_INLINE`, and the tables — are appended by
/// `corpus`, so extending one of those lists extends the sweeps with it.
const CORPUS: &[&str] = &[
    // Paragraphs.
    "<p><br>Text</p>",
    "<p><br><em>Text</em></p>",
    "<p><br></p>",
    "<p>a<br></p>",
    "<p>a<br>b</p>",
    "<p>a<br><br>b</p>",
    "<p>a<br><br><br><br>b</p>",
    "<p>a\n<br>\n<br>\n<br>\n<br>\nb</p>",
    "<p>a<br><br></p>",
    "<p>a\n<br>b</p>",
    "<p>a\n<br><em>b</em></p>",
    "<p>a\n<br>\nb</p>",
    "<p>a\n<br>\n<br>\nb</p>",
    "<p>\n    a\n    <br>\n    <em>b</em>\n</p>",
    "<p>\n<br>b</p>",
    "<p>\n\t\ta\n\t\t<br>\n\t\tb\n</p>",
    // Lists.
    "<ul><li><br>x</li></ul>",
    "<ul><li><br></li></ul>",
    "<ol><li><br></li></ol>",
    "<ul><li>a</li><li><br></li></ul>",
    "<ul><li><br></li></ul><p>x</p>",
    "<ul><li><p>x</p><br></li></ul>",
    "<ul><li>a<br></li></ul>",
    "<ul><li>a<br><ul><li>x</li></ul></li></ul>",
    "<ul><li><br><ul><li>x</li></ul></li></ul>",
    "<ul><li>a<br>b</li></ul>",
    "<ul><li>a\n<br>b</li></ul>",
    "<ol><li>a<br>b</li></ol>",
    "<ul><li><p>a<br>b</p></li></ul>",
    "<ul><li><p>a<br></p></li></ul>",
    "<ul><li><blockquote>a<br>b</blockquote></li></ul>",
    "<ul><li><ul><li>a<br>b</li></ul></li></ul>",
    "<ul><li>a<em><br>b</em></li></ul>",
    "<ul><li>a<em>b<br>c</em></li></ul>",
    // Blockquotes.
    "<blockquote><br>x</blockquote>",
    "<blockquote>a<br>b</blockquote>",
    "<blockquote>a\n<br>b</blockquote>",
    "<blockquote><p>a<br>b</p></blockquote>",
    "<blockquote><br></blockquote>",
    "<blockquote><em><br></em>x</blockquote>",
    "<h1><em><br></em>a</h1>",
    "<blockquote><blockquote><br></blockquote></blockquote>",
    "<blockquote><br></blockquote><p>x</p>",
    "<blockquote><br><p>x</p></blockquote>",
    "<blockquote><br><blockquote>x</blockquote></blockquote>",
    "<blockquote><p>x</p><br></blockquote>",
    "<blockquote><h1>a</h1><br></blockquote>",
    "<blockquote>x<br><br></blockquote>",
    "<blockquote>a<br></blockquote>",
    "<blockquote><br><br>x</blockquote>",
    "<blockquote><h1><br></h1></blockquote>",
    "<blockquote><h1>a<br>b</h1></blockquote>",
    "<blockquote><ul><li>a<br>b</li></ul></blockquote>",
    // Divs.
    "<div><br></div>",
    "<div><br><ul><li>x</li></ul></div>",
    // Links.
    "<p>a<a href=\"u\"><br></a>b</p>",
    "<p><a href=\"u\"><br></a>x</p>",
    "<p>x<a href=\"u\"><br></a></p>",
    "<p><a href=\"u\"><br></a></p>",
    "<p>a<a href=\"u\"><br>c</a>b</p>",
    "<p>a<a href=\"u\">c<br></a>b</p>",
    "<p><a href=\"u\">c<br></a></p>",
    "<p>a<a href=\"u\">c<br>d</a>b</p>",
    // Code, inline and block.
    "<p>a<code><br></code>b</p>",
    "<p><code><br></code></p>",
    "<p>a<code>x<br>y</code>b</p>",
    "<p>a<code>x<br></code>b</p>",
    "<pre><code>a<br>b</code></pre>",
    "<pre><code>a<br><br>b</code></pre>",
    "<pre><code><br>a</code></pre>",
    "<pre><code>a<br></code></pre>",
    "<pre><code><br></code></pre>",
    "<pre><code class=\"language-rs\">a<br>b</code></pre>",
    "<pre>a<br>b</pre>",
    "<pre><br></pre>",
    "<p>a<code><em><br></em></code>b</p>",
    // Inline elements on both sides of the break, and a list that serializes
    // itself inside a quote.
    "<p>a<em>b</em><br><em>c</em>d</p>",
    "<blockquote><ul><li><br></li></ul></blockquote>",
    "<blockquote><ul><li>a<br></li></ul></blockquote>",
    // Not here, though each is pinned above, since all three would fail the
    // round trip for reasons unrelated to the `<br>`: the `<img>` shapes, since
    // `shape` compares attributes exactly and `![](i)` reads back as
    // `<img src="i" alt="">`; the hrefless `<a>`, which writes nothing of its
    // own; and the `<caption>`, which htmd writes ahead of the table.
    //
    // A `<br>` written some other way, or carrying attributes faithful mode has
    // to keep.
    "<p>a<br/>b</p>",
    "<p>a<br class=\"x\">b</p>",
    "<p><br class=\"x\"></p>",
    "<h1><br class=\"x\"></h1>",
];

/// The shapes that put a `<br>` at the document root, where a raw `<br>` opens
/// its own line by design.
///
/// `a_raw_br_never_opens_an_output_line` skips these, being exactly its
/// counterexamples. The other two sweeps want them: the round-trip check because
/// it is the whole point that these still parse back, and
/// `a_lone_tag_never_opens_an_output_line` because a lone `<br>` on a line is
/// the only thing in this file exercising its void-element exception.
const ROOT_CORPUS: &[&str] = &[
    "<br>",
    "<br><br>",
    "<br>x",
    "<p>a</p><br>",
    "<p>a</p><br><p>b</p>",
];

/// `CORPUS` plus the shapes built at runtime from the lists they vary over.
fn corpus() -> Vec<String> {
    let mut shapes: Vec<String> = CORPUS.iter().map(|html| html.to_string()).collect();
    shapes.extend(
        [
            "a<br>b",
            "<br>",
            "a\n<br>b",
            "a<em><br></em>b",
            "<p>a<br>b</p>",
            "<p><br></p>",
            "<ul><li>a<br>b</li></ul>",
        ]
        .map(table),
    );
    shapes.push(
        "<table><thead><tr><th><br></th></tr></thead><tbody><tr><td>x</td></tr></tbody></table>"
            .to_string(),
    );
    for level in 1..=6usize {
        for content in [
            "<br>",
            "a<br>b",
            "a\n<br>b",
            "a<br>",
            "<br>a",
            "<em><br></em>",
        ] {
            shapes.push(format!("<h{level}>{content}</h{level}>"));
        }
    }
    for tag in EMPHASIS {
        for content in [
            format!("a<{tag}><br></{tag}>b"),
            format!("<{tag}><br></{tag}>x"),
            format!("x<{tag}><br></{tag}>"),
            format!("<{tag}><br></{tag}>"),
            format!("<{tag}><br>a</{tag}>"),
            format!("a<{tag}><br>b</{tag}>"),
            format!("<{tag}>a<br></{tag}>b"),
            format!("a<{tag}>b<br></{tag}>c"),
            format!("<{tag}>a<br></{tag}>"),
            format!("<{tag}>a<br>b</{tag}>"),
            format!("a<{tag}>b<br>c</{tag}>d"),
        ] {
            shapes.push(format!("<p>{content}</p>"));
        }
        shapes.push(format!("<h1>a<{tag}><br></{tag}>b</h1>"));
        shapes.push(format!("<ul><li><{tag}><br></{tag}>x</li></ul>"));
        shapes.push(format!("<blockquote>a<{tag}><br></{tag}>b</blockquote>"));
        shapes.push(table(&format!("a<{tag}><br></{tag}>b")));
    }
    shapes.push("<p>a<em><strong><br></strong></em>b</p>".to_string());
    shapes.push("<p>a<em><br></em><em><br></em>b</p>".to_string());
    for tag in UNTRANSLATED_INLINE.into_iter().chain(["span"]) {
        shapes.push(format!("<p>a<{tag}><br></{tag}>b</p>"));
    }
    shapes
}

/// [`corpus`] plus [`ROOT_CORPUS`], for the sweeps that want the root-level
/// shapes as well.
fn corpus_with_root() -> Vec<String> {
    let mut shapes = corpus();
    shapes.extend(ROOT_CORPUS.iter().map(|html| html.to_string()));
    shapes
}

/// Every conversion this file asks for: both translation modes and both heading
/// styles, under both `BrStyle`s. Each invariant holds over all of them.
fn conversions(html: &str) -> Vec<(TranslationMode, String, String)> {
    let mut out = Vec::new();
    for mode in [TranslationMode::Faithful, TranslationMode::Pure] {
        for style in STYLES {
            for heading in [HeadingStyle::Atx, HeadingStyle::Setex] {
                out.push((
                    mode,
                    format!("{html:?}, {mode:?}, {style:?}, {heading:?}"),
                    convert(html, mode, style, heading),
                ));
            }
        }
    }
    out
}

/// Runs `check` over every conversion of every shape in `shapes`. `check` is
/// handed the input HTML, the mode that produced this conversion, a label naming
/// the whole option set for the failure message, and the Markdown.
fn sweep(shapes: Vec<String>, mut check: impl FnMut(&str, TranslationMode, &str, &str)) {
    for input in shapes {
        for (mode, label, md) in conversions(&input) {
            check(&input, mode, &label, &md);
        }
    }
}

/// A line with the Markdown that opens its block stripped off — a blockquote's
/// `>` markers, however deep, and a list item's bullet. An HTML block opens on
/// that content rather than on the raw line, so `> <br>` is as much of an HTML
/// block as a bare `<br>` is.
fn block_content(line: &str) -> &str {
    let mut rest = line.trim_start();
    loop {
        if let Some(quoted) = rest.strip_prefix('>') {
            rest = quoted.trim_start();
            continue;
        }
        // A bullet is a marker followed by whitespace, which is what tells the
        // `*   x` of a list item from the emphasis in `*x*`.
        let after_marker = match rest.strip_prefix(['*', '-', '+']) {
            Some(after) => Some(after),
            None => {
                let digits = rest.trim_start_matches(|ch: char| ch.is_ascii_digit());
                (digits.len() < rest.len())
                    .then(|| digits.strip_prefix(['.', ')']))
                    .flatten()
            }
        };
        // A nested list writes both its markers on the item's first line, hence
        // the loop. An empty item leaves its marker with nothing after it.
        match after_marker {
            Some(after) if after.is_empty() || after.starts_with([' ', '\t']) => {
                rest = after.trim_start();
            }
            _ => return rest,
        }
    }
}

/// The tag name of `text`, where `text` is a complete open or closing tag and
/// nothing else — the shape CommonMark's [HTML block type
/// 7](https://spec.commonmark.org/0.31.2/#html-blocks) keys on.
fn lone_tag(text: &str) -> Option<&str> {
    let inner = text.strip_prefix('<')?.strip_suffix('>')?;
    // Anything else on the line, including a second tag, leaves a `>` behind.
    if inner.contains('>') {
        return None;
    }
    let name = inner.strip_prefix('/').unwrap_or(inner);
    let name = &name[..name.find([' ', '\t', '/']).unwrap_or(name.len())];
    name.chars()
        .next()
        .is_some_and(char::is_alphabetic)
        .then_some(name)
}

/// The HTML elements with no content, whose tag is therefore a whole HTML block
/// on its own rather than the start of one.
const VOID_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

/// A raw `<br>` must never be what opens an output line: CommonMark reads that
/// as an HTML block, which swallows every following line up to the next blank
/// one.
#[test]
fn a_raw_br_never_opens_an_output_line() {
    sweep(corpus(), |_, _, label, md| {
        for line in md.lines() {
            assert!(
                !block_content(line).starts_with("<br"),
                "a raw <br> opens a line of {md:?} (from {label}); CommonMark parses \
                 that as an HTML block and swallows what follows it"
            );
        }
    });
}

/// The companion to the rule above, and the reason a serialized inline element
/// is written on a single line. A line holding a complete open *or closing* tag
/// and nothing else is an HTML block of type 7: an open tag alone swallows the
/// element's own content, a closing tag alone swallows what follows the element.
///
/// A void element's tag is the exception, having no content to swallow — but
/// only where a blank line ends the block before any Markdown follows, which is
/// what makes the root-level `<br>` of `br_at_document_root_faithful` safe.
#[test]
fn a_lone_tag_never_opens_an_output_line() {
    sweep(corpus_with_root(), |_, _, label, md| {
        let lines: Vec<&str> = md.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            let Some(tag) = lone_tag(block_content(line)) else {
                continue;
            };
            let ends_at_a_blank_line = lines
                .get(index + 1)
                .is_none_or(|next| block_content(next).is_empty());
            assert!(
                VOID_ELEMENTS.contains(&tag) && ends_at_a_blank_line,
                "the tag {tag:?} is alone on a line of {md:?} (from {label}); CommonMark \
                 parses that as an HTML block and swallows every line up to the next \
                 blank one"
            );
        }
    });
}

/// Everything a delimiter run needs to be judged: CommonMark's flanking rules
/// look only at the characters on either side of it, with the ends of the run's
/// own line or block counting as whitespace — a `None` here.
struct Run {
    /// Where the run starts, for reporting.
    start: usize,
    /// How many markers it holds. A run closes only what a run of its own
    /// length opened.
    length: usize,
    before: Option<char>,
    after: Option<char>,
}

impl Run {
    /// Whether this run can open emphasis: not followed by whitespace, and
    /// either not followed by punctuation or else preceded by whitespace or
    /// punctuation.
    fn is_left_flanking(&self) -> bool {
        self.after.is_some_and(|after| {
            !after.is_whitespace()
                && (!is_punctuation(after)
                    || self
                        .before
                        .is_none_or(|b| b.is_whitespace() || is_punctuation(b)))
        })
    }

    /// Whether this run can close emphasis, the mirror of
    /// [`Run::is_left_flanking`].
    fn is_right_flanking(&self) -> bool {
        self.before.is_some_and(|before| {
            !before.is_whitespace()
                && (!is_punctuation(before)
                    || self
                        .after
                        .is_none_or(|a| a.is_whitespace() || is_punctuation(a)))
        })
    }
}

/// Everything that is neither alphanumeric nor whitespace. CommonMark's own list
/// is Unicode punctuation and symbols; the difference only makes this stricter.
fn is_punctuation(ch: char) -> bool {
    !ch.is_alphanumeric() && !ch.is_whitespace()
}

/// The `*` delimiter runs of `text`, in order, with the escaped ones left out —
/// a `\*` is literal text rather than a marker, and an odd number of
/// backslashes ahead of the run is what escapes it.
fn emphasis_runs(text: &str) -> Vec<Run> {
    let chars: Vec<char> = text.chars().collect();
    let mut runs = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        if chars[start] != '*' {
            start += 1;
            continue;
        }
        let end = chars[start..]
            .iter()
            .position(|ch| *ch != '*')
            .map_or(chars.len(), |offset| start + offset);
        let backslashes = chars[..start]
            .iter()
            .rev()
            .take_while(|ch| **ch == '\\')
            .count();
        if backslashes % 2 == 0 {
            runs.push(Run {
                start,
                length: end - start,
                before: chars[..start].last().copied(),
                after: chars.get(end).copied(),
            });
        }
        start = end;
    }
    runs
}

/// The stretches of output an emphasis pair can span: a run of lines that hold
/// text, with the Markdown that opens each line's block stripped off, and each
/// table cell on its own. Emphasis reaches across the lines of a paragraph but
/// never across a blank line, a list marker or a cell wall.
fn emphasis_scopes(md: &str) -> Vec<String> {
    let mut scopes: Vec<String> = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    for line in md.lines() {
        let content = block_content(line);
        if let Some(row) = content.strip_prefix('|') {
            scopes.extend(paragraph.drain(..).map(str::to_string));
            scopes.extend(row.split('|').map(|cell| block_content(cell).to_string()));
        } else if content.is_empty() {
            scopes.extend(paragraph.drain(..).map(str::to_string));
        } else {
            paragraph.push(content);
        }
    }
    if !paragraph.is_empty() {
        scopes.push(paragraph.join("\n"));
    }
    scopes
}

/// The rule behind every emphasis failure in this file: emphasis htmd writes has
/// to be emphasis CommonMark reads. Every `*` run must pair with another that
/// flanks the way its side needs, or it is silently literal text.
///
/// Pairing is what it takes to judge a marker against a break, since the run
/// alone does not say which side it is. In `*a*\` the run against the `\` is a
/// closer, which is exactly right; in `a*\` it would be an opener, which is the
/// bug — and the characters around the run are identical in both.
#[test]
fn every_emphasis_marker_pairs() {
    sweep(corpus(), |_, _, label, md| {
        for scope in emphasis_scopes(md) {
            let mut open: Vec<&Run> = Vec::new();
            let runs = emphasis_runs(&scope);
            for run in &runs {
                let closes = run.is_right_flanking()
                    && open.last().is_some_and(|last| last.length == run.length);
                if closes {
                    open.pop();
                } else if run.is_left_flanking() {
                    open.push(run);
                } else {
                    panic!(
                        "in {md:?} (from {label}) the marker at {} of {scope:?} neither \
                         opens nor closes emphasis — it sits between {:?} and {:?}, so \
                         CommonMark reads it as literal text",
                        run.start, run.before, run.after
                    );
                }
            }
            assert!(
                open.is_empty(),
                "in {md:?} (from {label}) the marker at {} of {scope:?} opens emphasis \
                 that nothing closes",
                open[0].start
            );
        }
    });
}

/// One piece of a [`shape`]: a tag, or a run of text.
#[derive(Debug, PartialEq, Eq)]
enum Piece {
    Tag(String),
    Text(String),
}

/// A normalized reading of an HTML fragment: the elements and text it holds,
/// with everything that says nothing about a `<br>` left out.
///
/// Whitespace collapses the way HTML collapses it, and adjacent text merges,
/// since a dropped element leaves the text on either side of it touching. `<p>`
/// is ignored outright, coming and going with CommonMark's tight-and-loose list
/// rules; `<i>` and `<b>` read as `<em>` and `<strong>`. Everything else,
/// including every attribute, has to match.
fn shape(html: &str) -> Vec<Piece> {
    fn walk(node: ego_tree::NodeRef<'_, Node>, out: &mut Vec<Piece>) {
        for child in node.children() {
            match child.value() {
                Node::Text(text) => out.push(Piece::Text(text.to_string())),
                Node::Element(element) => {
                    let name = match element.name() {
                        "i" => "em",
                        "b" => "strong",
                        other => other,
                    };
                    // `<html>` is the wrapper the fragment parser adds; `<p>` is
                    // the one both sides may disagree about for free.
                    if name == "html" || name == "p" {
                        walk(child, out);
                        continue;
                    }
                    let mut attrs: Vec<String> = element
                        .attrs()
                        .map(|(key, value)| format!(" {key}=\"{value}\""))
                        .collect();
                    attrs.sort();
                    out.push(Piece::Tag(format!("<{name}{}>", attrs.concat())));
                    // An element with no children is a whole tag on its own,
                    // which keeps a `<br>` reading as one in the output below.
                    if child.has_children() {
                        walk(child, out);
                        out.push(Piece::Tag(format!("</{name}>")));
                    }
                }
                _ => {}
            }
        }
    }

    let mut pieces = Vec::new();
    walk(Html::parse_fragment(html).tree.root(), &mut pieces);

    let mut merged: Vec<Piece> = Vec::new();
    for piece in pieces {
        match (&mut merged.last_mut(), piece) {
            (Some(Piece::Text(previous)), Piece::Text(text)) => previous.push_str(&text),
            (_, piece) => merged.push(piece),
        }
    }
    merged
        .into_iter()
        .filter_map(|piece| match piece {
            Piece::Text(text) => {
                let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
                (!collapsed.is_empty()).then_some(Piece::Text(collapsed))
            }
            tag => Some(tag),
        })
        .collect()
}

/// What faithful mode undertakes: its output, read back as CommonMark, is the
/// HTML it was given. Checked against a real CommonMark parser over every shape
/// in the corpus, including the root-level ones the three rules above leave out.
///
/// Pure mode is exempt by definition, dropping what Markdown cannot carry; what
/// it drops is pinned case by case above.
#[test]
fn faithful_output_round_trips() {
    sweep(corpus_with_root(), |input, mode, label, md| {
        if mode == TranslationMode::Faithful {
            assert_eq!(
                shape(input),
                shape(&render(md)),
                "{label} became {md:?}, which reads back as different HTML"
            );
        }
    });
}
