use pretty_assertions::assert_eq;

use crate::common::{convert_faithful, round_trip};

// Multi-byte UTF-8 characters before a markdown ordered list dot must not
// cause a panic due to byte/char index confusion in escape_text.
#[test]
fn multibyte_ordered_list_escape_half() {
    let md = convert_faithful("<p>2½. Long shot</p>").unwrap();
    assert_eq!("2½. Long shot", md);
}

#[test]
fn multibyte_ordered_list_escape_accented() {
    // e-acute before dot -- not numeric, so the dot is not an ordered list marker
    let md = convert_faithful("<p>1é. text</p>").unwrap();
    assert_eq!(r"1é. text", md);
}

#[test]
fn multibyte_ordered_list_escape_trademark() {
    // trademark symbol is not numeric
    let md = convert_faithful("<p>3™. text</p>").unwrap();
    assert_eq!(r"3™. text", md);
}

#[test]
fn ascii_ordered_list_escape() {
    let md = convert_faithful("<p>10. normal</p>").unwrap();
    assert_eq!(r"10\. normal", md);
}

#[test]
fn multibyte_no_dot() {
    // No dot, should not be affected
    let md = convert_faithful("<p>2½</p>").unwrap();
    assert_eq!("2½", md);
}

#[test]
fn cjk_before_ordered_list() {
    // CJK chars are not numeric in Rust's is_numeric(), so this is not an ordered list pattern
    let md = convert_faithful("<p>日本語1. test</p>").unwrap();
    assert_eq!(r"日本語1. test", md);
}

#[test]
fn multibyte_atx_heading_escape() {
    let md = convert_faithful("<p># héading</p>").unwrap();
    assert_eq!(r"\# héading", md);
}

#[test]
fn multibyte_atx_heading_escape_umlaut() {
    let md = convert_faithful("<p>## über</p>").unwrap();
    assert_eq!(r"\## über", md);
}

/// Asserts both halves of the escapes below: the paragraph the Markdown reads
/// back as, and the spelling which carries it. The spelling alone proves
/// nothing -- it says which bytes came out, not that they still mean what the
/// HTML did -- so every case names the paragraph content it has to come back
/// holding.
///
/// The trip is asserted first so that an escape which stops working reports the
/// harm it does -- the paragraph dissolved into a heading or a list -- rather
/// than a mismatched string.
fn assert_paragraph_survives(html: &str, expected_markdown: &str, expected_content: &str) {
    assert_eq!(
        format!("<p>{expected_content}</p>\n"),
        round_trip(html),
        "round trip of {html}"
    );
    assert_eq!(
        expected_markdown,
        convert_faithful(html).unwrap(),
        "markdown for {html}"
    );
}

/// The "Paragraphs" section of `translation_approach.md` keeps a
/// paragraph a paragraph where its content would otherwise open an HTML block.
/// Content which opens any other leaf block dissolves it the same way, and for
/// the same reason: the block scan reaches the content because nothing precedes
/// it on the line. A marker needs no text after it to open its block, so the
/// end of the line closes it just as a space does.
#[test]
fn a_paragraph_of_leaf_block_syntax_stays_a_paragraph() {
    assert_paragraph_survives("<p>-</p>", r"\-", "-");
    assert_paragraph_survives("<p>*</p>", r"\*", "*");
    assert_paragraph_survives("<p>+</p>", r"\+", "+");
    assert_paragraph_survives("<p>---</p>", r"\---", "---");
    assert_paragraph_survives("<p>===</p>", r"\===", "===");
    assert_paragraph_survives("<p>1.</p>", r"1\.", "1.");
    assert_paragraph_survives("<p>1)</p>", r"1\)", "1)");
    assert_paragraph_survives("<p>- - -</p>", r"\- - -", "- - -");

    // A `-` run opens a break or an underline only where nothing else shares
    // the line, so text behind it needs no escape.
    assert_paragraph_survives("<p>--foo</p>", "--foo", "--foo");
}

/// A `#` run closed by the end of the line is an ATX heading holding nothing,
/// so it needs the escape although no text follows it.
#[test]
fn an_atx_marker_alone_in_a_paragraph_is_escaped() {
    assert_paragraph_survives("<p>#</p>", r"\#", "#");
    assert_paragraph_survives("<p>######</p>", r"\######", "######");

    // Seven `#` open no heading, so this escape is one the conversion does not
    // need; it costs a backslash and still reads back as the text it came from.
    assert_paragraph_survives("<p>#######</p>", r"\#######", "#######");

    // A `#` run which something other than a space or the line end closes is
    // not a marker at all.
    assert_paragraph_survives("<p>#x</p>", "#x", "#x");
}

// The tests below spell the cases above with the whitespace a pretty-printer
// writes: a line ending or a tab in place of the space which closes the
// marker, and spaces before the marker itself. The walk compresses the one to
// the single space the marker needs and trims the other, so each opens the
// same block its plainer spelling does and needs the same escape.

/// A bullet marker which a line ending or a tab closes opens a list once the
/// whitespace is compressed, so it needs the escape "Special case for
/// paragraphs" calls for.
#[test]
fn a_bullet_marker_closed_by_a_line_ending_is_escaped() {
    assert_paragraph_survives("<p>-\nfoo</p>", r"\- foo", "- foo");
    assert_paragraph_survives("<p>+\nfoo</p>", r"\+ foo", "+ foo");
    assert_paragraph_survives("<p>-\tfoo</p>", r"\- foo", "- foo");
}

/// An ATX marker which a line ending or a tab closes opens a heading once the
/// whitespace is compressed.
#[test]
fn an_atx_marker_closed_by_a_line_ending_is_escaped() {
    assert_paragraph_survives("<p>#\nfoo</p>", r"\# foo", "# foo");
    assert_paragraph_survives("<p>###\nfoo</p>", r"\### foo", "### foo");
    assert_paragraph_survives("<p>#\tfoo</p>", r"\# foo", "# foo");
}

/// An ordered item's delimiter which a line ending or a tab closes opens a list
/// once the whitespace is compressed.
#[test]
fn an_ordered_item_delimiter_closed_by_a_line_ending_is_escaped() {
    assert_paragraph_survives("<p>1.\nfoo</p>", r"1\. foo", "1. foo");
    assert_paragraph_survives("<p>1)\nfoo</p>", r"1\) foo", "1) foo");
}

/// Whitespace before a marker hides it only until the walk trims it: what a
/// block's content opens with is its first non-whitespace character, so the
/// marker starts the line after all. The last case is the shape a formatter
/// produces from a paragraph whose text happens to start with a number.
#[test]
fn a_marker_behind_leading_whitespace_is_escaped() {
    assert_paragraph_survives("<p> 1. foo</p>", r"1\. foo", "1. foo");
    assert_paragraph_survives("<p> - foo</p>", r"\- foo", "- foo");
    assert_paragraph_survives("<p>\n# foo</p>", r"\# foo", "# foo");
    assert_paragraph_survives(
        "<p>\n  1.\n  Item numbering\n</p>",
        r"1\. Item numbering",
        "1. Item numbering",
    );
}

/// Whitespace inside a line opens nothing, so the same text needs no escape
/// where something precedes it on the line.
#[test]
fn a_marker_which_starts_no_line_is_not_escaped() {
    assert_paragraph_survives("<p>a <em>b</em> - c</p>", "a *b* - c", "a <em>b</em> - c");
    assert_paragraph_survives(
        "<p>a <em>b</em> 1. c</p>",
        "a *b* 1. c",
        "a <em>b</em> 1. c",
    );
}

/// Only the first delimiter after a digit run can open a list: a second
/// delimiter closes the first, which leaves it no whitespace terminator. Both
/// spellings are the text they came from, so neither is escaped.
#[test]
fn a_run_of_ordered_item_delimiters_opens_no_list() {
    assert_paragraph_survives("<p>1)))</p>", "1)))", "1)))");
    assert_paragraph_survives("<p>1.)</p>", "1.)", "1.)");
}

/// An [ordered list marker](https://spec.commonmark.org/0.31.2/#ordered-list-marker)
/// counts ASCII digits alone, so a run holding a Unicode numeric character
/// opens no list and the paragraph carrying it needs no escape. A backslash
/// reads back as the text it came from, which is why the trips below hold
/// either way: what the escape costs is a character in the Markdown.
#[test]
fn a_paragraph_of_unicode_numerics_needs_no_escape() {
    assert_paragraph_survives("<p>2½. text</p>", "2½. text", "2½. text");
    assert_paragraph_survives("<p>1١. text</p>", "1١. text", "1١. text");
}
