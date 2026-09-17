//! Recognizes the [HTML block](https://spec.commonmark.org/0.31.2/#html-blocks)
//! start conditions in translated CommonMark content.
//!
//! A leaf block whose content opens an HTML block is dissolved by it: the
//! `<p>` or setext underline around the content is lost when the CommonMark is
//! read back. The "Special case for paragraphs" section of
//! `unsupported_html.md` asks the containers which are exposed to this — a
//! paragraph and a setext heading — to check their content for it.

use crate::util::node::{is_block_element, is_type_1_element};

/// Whether a CommonMark parser reading `content` at the start of a line would
/// open an HTML block.
///
/// Types 1-6 are tested by their start conditions, which match a prefix of the
/// line. Type 7 — a line holding one complete tag and nothing else — is tested
/// last, since a tag name belonging to an earlier type opens that type instead.
/// Type 7 cannot *interrupt* a paragraph, but it does open one, which is the
/// question asked here.
pub(crate) fn starts_html_block(content: &str) -> bool {
    // A start condition survives up to three spaces of indentation; a fourth
    // makes the line an indented code block instead. Every condition then
    // opens with a `<`, so content which does not is settled without looking
    // for the end of the first line.
    let indented = content.trim_start_matches(' ');
    if content.len() - indented.len() > 3 {
        return false;
    }
    let Some(after_angle) = indented.strip_prefix('<') else {
        return false;
    };
    // htmd must produce the entire block on a single line. Therefore, only
    // check the first (and only) line for an HTML block.
    let after_angle = after_angle.lines().next().unwrap_or_default();

    // Types 2-5, each named by the characters which follow the `<`. A line
    // opening `<!` or `<?` holds no tag name, so no later type can match it.
    if let Some(after_bang) = after_angle.strip_prefix('!') {
        return after_bang.starts_with("--")
            || after_bang.starts_with("[CDATA[")
            || after_bang.starts_with(|ch: char| ch.is_ascii_alphabetic());
    }
    if after_angle.starts_with('?') {
        return true;
    }

    let is_closing_tag = after_angle.starts_with('/');
    let after_slash = after_angle.strip_prefix('/').unwrap_or(after_angle);
    if let Some(after_name) = scan_tag_name(after_slash) {
        let name = after_slash[..after_slash.len() - after_name.len()].to_ascii_lowercase();
        // Type 1 ends its name at whitespace, `>`, or the line end — `/>` is
        // absent from its condition — and only an opening tag starts it.
        if !is_closing_tag
            && is_type_1_element(&name)
            && (after_name.is_empty() || after_name.starts_with(['>', ' ', '\t']))
        {
            return true;
        }
        // Type 6 takes either tag. Its tag list is `BLOCK_ELEMENTS` less the
        // type 1 names, which the spec leaves out of it.
        if is_block_element(&name)
            && !is_type_1_element(&name)
            && (after_name.is_empty()
                || after_name.starts_with(['>', ' ', '\t'])
                || after_name.starts_with("/>"))
        {
            return true;
        }
    }

    // Type 7.
    scan_tag(after_angle).is_some_and(|rest| rest.trim_start_matches([' ', '\t']).is_empty())
}

/// Scans an [open tag](https://spec.commonmark.org/0.31.2/#open-tag) or
/// [closing tag](https://spec.commonmark.org/0.31.2/#closing-tag) whose `<`
/// has already been consumed, returning what follows the tag.
fn scan_tag(after_angle: &str) -> Option<&str> {
    if let Some(after_slash) = after_angle.strip_prefix('/') {
        let after_name = scan_tag_name(after_slash)?;
        return strip_whitespace(after_name).strip_prefix('>');
    }

    let mut rest = scan_tag_name(after_angle)?;
    loop {
        let after_whitespace = strip_whitespace(rest);
        if let Some(after_tag) = after_whitespace
            .strip_prefix("/>")
            .or_else(|| after_whitespace.strip_prefix('>'))
        {
            return Some(after_tag);
        }
        // Whitespace is what separates an attribute from what precedes it, so
        // without any the tag ends here, unterminated.
        if after_whitespace.len() == rest.len() {
            return None;
        }
        rest = scan_attribute(after_whitespace)?;
    }
}

/// Scans a [tag name](https://spec.commonmark.org/0.31.2/#tag-name): an ASCII
/// letter followed by ASCII letters, digits, and hyphens.
fn scan_tag_name(text: &str) -> Option<&str> {
    if !text.starts_with(|ch: char| ch.is_ascii_alphabetic()) {
        return None;
    }
    Some(trim_start_while(text, |ch| {
        ch.is_ascii_alphanumeric() || ch == '-'
    }))
}

/// Scans an [attribute](https://spec.commonmark.org/0.31.2/#attribute): a name
/// and an optional value specification.
fn scan_attribute(text: &str) -> Option<&str> {
    let after_name = scan_attribute_name(text)?;
    let Some(after_equals) = strip_whitespace(after_name).strip_prefix('=') else {
        return Some(after_name);
    };
    scan_attribute_value(strip_whitespace(after_equals))
}

/// Scans an [attribute name](https://spec.commonmark.org/0.31.2/#attribute-name):
/// an ASCII letter, `_`, or `:` followed by ASCII letters, digits, `_`, `.`,
/// `:`, and hyphens.
fn scan_attribute_name(text: &str) -> Option<&str> {
    if !text.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_' || ch == ':') {
        return None;
    }
    Some(trim_start_while(text, |ch| {
        ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | ':' | '-')
    }))
}

/// Scans an [attribute value](https://spec.commonmark.org/0.31.2/#attribute-value),
/// which is unquoted, single-quoted, or double-quoted.
fn scan_attribute_value(text: &str) -> Option<&str> {
    match text.chars().next()? {
        quote @ ('"' | '\'') => {
            let quoted = &text[quote.len_utf8()..];
            let end = quoted.find(quote)?;
            Some(&quoted[end + quote.len_utf8()..])
        }
        _ => {
            let rest = trim_start_while(text, |ch| !" \t\"'=<>`".contains(ch));
            // An unquoted value holds at least one character.
            (rest.len() < text.len()).then_some(rest)
        }
    }
}

fn strip_whitespace(text: &str) -> &str {
    text.trim_start_matches([' ', '\t'])
}

fn trim_start_while(text: &str, accept: impl Fn(char) -> bool) -> &str {
    let end = text
        .char_indices()
        .find(|(_, ch)| !accept(*ch))
        .map_or(text.len(), |(index, _)| index);
    &text[end..]
}

#[cfg(test)]
mod tests {
    use super::starts_html_block;

    #[test]
    fn recognizes_type_1_through_6_prefixes() {
        assert!(starts_html_block("<pre>a"));
        assert!(starts_html_block("<script src=\"s\">a"));
        assert!(starts_html_block("<TEXTAREA>a"));
        assert!(starts_html_block("<!-- c -->a"));
        assert!(starts_html_block("<?php a"));
        assert!(starts_html_block("<!DOCTYPE html>a"));
        assert!(starts_html_block("<![CDATA[a]]>b"));
        assert!(starts_html_block("<div>a</div>"));
        assert!(starts_html_block("<iframe src=\"u\">a</iframe>"));
        assert!(starts_html_block("</div>a"));
        assert!(starts_html_block("<hr/>a"));
    }

    /// Type 1 ends its tag name at whitespace, `>`, or the line end, so `/>`
    /// leaves it to the type 7 test, which a trailing `a` then fails.
    #[test]
    fn applies_the_type_1_and_6_name_terminators() {
        assert!(!starts_html_block("<pre/>a"));
        assert!(!starts_html_block("<prefix>a"));
        assert!(!starts_html_block("<divider>a"));
        assert!(!starts_html_block("<!1>a"));
    }

    /// The line end terminates a type 1 or 6 tag name as a space or a `>`
    /// would, whether or not it ends the content as well.
    #[test]
    fn ends_a_type_1_or_6_name_at_the_line_end() {
        assert!(starts_html_block("<pre"));
        assert!(starts_html_block("<div"));
        assert!(starts_html_block("<pre\na"));
        assert!(starts_html_block("<div\na"));
    }

    #[test]
    fn recognizes_a_lone_type_7_tag() {
        assert!(starts_html_block("<br>"));
        assert!(starts_html_block("<br>  \t"));
        assert!(starts_html_block("<br/>"));
        assert!(starts_html_block("</span>"));
        assert!(starts_html_block("<a href=\"u\" title='t' data-x=y>"));
        assert!(starts_html_block("<a\thref=\"u\" >"));
        assert!(starts_html_block("<a href = \"u\">"));
        assert!(starts_html_block("<a href\t=\tu>"));
    }

    /// A type 1 name is left out of the type 6 tag list, and type 1 itself
    /// takes only an opening tag, so a closing one falls through to type 7.
    #[test]
    fn recognizes_a_lone_closing_type_1_tag() {
        assert!(starts_html_block("</pre>"));
        assert!(!starts_html_block("</pre>a"));
    }

    #[test]
    fn rejects_a_type_7_tag_which_is_not_alone() {
        assert!(!starts_html_block("<br><br>"));
        assert!(!starts_html_block("<br>a"));
        assert!(!starts_html_block("<span>a</span>"));
        assert!(!starts_html_block("a<br>"));
        assert!(!starts_html_block("<del><br></del>"));
    }

    #[test]
    fn rejects_an_incomplete_or_invalid_tag() {
        assert!(!starts_html_block("<br"));
        assert!(!starts_html_block("<a href=>"));
        assert!(!starts_html_block("<a href=\"u>"));
        assert!(!starts_html_block("<a href=\"u\"title=\"t\">"));
        assert!(!starts_html_block("<1br>"));
        assert!(!starts_html_block("</br a>"));
        assert!(!starts_html_block(""));
        assert!(!starts_html_block("plain text"));
    }

    /// A tab is four columns of indentation wherever it starts, so a line
    /// holding one before its `<` is an indented code block.
    #[test]
    fn allows_three_spaces_of_indentation() {
        assert!(starts_html_block("   <br>"));
        assert!(!starts_html_block("    <br>"));
        assert!(!starts_html_block("\t<br>"));
        assert!(!starts_html_block(" \t<br>"));
    }

    /// htmd must produce the entire block on a single line, so only the first
    /// line is checked.
    #[test]
    fn tests_only_the_first_line() {
        assert!(!starts_html_block("a\n<br>"));
        assert!(starts_html_block("<br>\na"));
    }
}
