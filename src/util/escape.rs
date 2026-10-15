use std::borrow::Cow;

/// Escape sequences that Markdown would treat as raw HTML or an autolink so that they stay as
/// literal text.
pub(crate) fn escape_html(text: Cow<'_, str>) -> Cow<'_, str> {
    let src = text.as_ref();
    if !src.contains('<') {
        return text;
    }

    let mut escaped = String::with_capacity(src.len());
    let mut modified = false;
    let mut last_copy_index = 0;

    for (idx, ch) in src.char_indices() {
        if ch != '<' {
            continue;
        }

        if should_escape_html_like_sequence(&src[idx..]) {
            escaped.push_str(&src[last_copy_index..idx]);
            escaped.push('\\');
            escaped.push('<');
            modified = true;
            last_copy_index = idx + 1;
        }
    }

    if !modified {
        return text;
    }

    escaped.push_str(&src[last_copy_index..]);
    Cow::Owned(escaped)
}

/// Whether the `<` opening `fragment` may open raw HTML or an autolink.
///
/// The end of `fragment` may be only the end of a text node, which an inline
/// sibling continues -- in pure mode, `&lt;<span>div</span> class&gt;` reads
/// as `<div class>` -- so a sequence which that end cuts short is escaped too.
/// That costs a backslash, never a wrong translation.
fn should_escape_html_like_sequence(fragment: &str) -> bool {
    let mut chars = fragment.chars();
    let Some('<') = chars.next() else {
        return false;
    };

    let Some(next) = chars.next() else {
        return true;
    };

    match next {
        // `<!\` opens no comment, CDATA section or declaration, whatever
        // follows, nor an email autolink, whose local part holds no `\`. It is
        // the spelling `escape_if_needed` gives a CDATA section, whose brackets
        // it escapes before this runs.
        '!' => !chars.as_str().starts_with('\\'),
        '?' => true,
        '/' if chars.next().is_none_or(|c| c.is_ascii_alphabetic()) => true,
        // A URI autolink's scheme opens with a letter, which the tag arm
        // covers; an email autolink may open with a digit or a symbol.
        c if c.is_ascii_alphabetic() => true,
        _ => may_start_email_autolink(fragment),
    }
}

/// Whether `fragment` opens with an
/// [email autolink](https://spec.commonmark.org/0.31.2/#email-autolink), or
/// with a prefix of one which the end of `fragment` cuts short.
///
/// Neither scan crosses a `<`, so escaping a whole string stays linear.
fn may_start_email_autolink(fragment: &str) -> bool {
    let Some(address) = fragment.strip_prefix('<') else {
        return false;
    };
    let local_part_len = address
        .bytes()
        .take_while(|&byte| byte.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~-".contains(&byte))
        .count();
    if local_part_len == address.len() {
        return true;
    }
    if local_part_len == 0 {
        return false;
    }
    let Some(domain) = address[local_part_len..].strip_prefix('@') else {
        return false;
    };
    let domain_len = domain
        .bytes()
        .take_while(|&byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.'))
        .count();
    if domain_len == domain.len() {
        return true;
    }
    domain[domain_len..].starts_with('>') && domain[..domain_len].split('.').all(is_domain_label)
}

/// Whether `label`, holding only ASCII alphanumerics and `-`, is one label of
/// an email autolink's domain.
fn is_domain_label(label: &str) -> bool {
    let bytes = label.as_bytes();
    bytes.len() <= 63
        && bytes.first().is_some_and(u8::is_ascii_alphanumeric)
        && bytes.last().is_some_and(u8::is_ascii_alphanumeric)
}

/// Whether `text` holds a line ending: the one place the set of line endings
/// the escapes here and in `element_util` know about is written down.
pub(crate) fn has_line_ending(text: &str) -> bool {
    text.contains(['\r', '\n'])
}

/// Pushes `ch` onto `output`, writing a line ending as the character reference
/// a CommonMark parser decodes back into it. Encoding this way lets a line
/// ending sit inside a leaf block — a raw HTML inline, a link destination, a
/// quoted title — which a bare one would end.
///
/// Being per-character, this is no use where a CRLF pair has to count as the
/// one line ending it is; see `element_util::escape_html_block_blank_lines`.
pub(crate) fn push_encoding_line_ending(output: &mut String, ch: char) {
    match ch {
        '\r' => output.push_str("&#13;"),
        '\n' => output.push_str("&#10;"),
        _ => output.push(ch),
    }
}

/// Writes every line ending in `text` as the character reference
/// [`push_encoding_line_ending`] gives it.
pub(crate) fn encode_line_endings(text: &str) -> Cow<'_, str> {
    if !has_line_ending(text) {
        return Cow::Borrowed(text);
    }
    let mut encoded = String::with_capacity(text.len() + 8);
    for ch in text.chars() {
        push_encoding_line_ending(&mut encoded, ch);
    }
    Cow::Owned(encoded)
}

/// Backslash-escapes each character of `text` which `needs_backslash` selects,
/// and encodes its line endings.
fn escape_inline_string(text: &str, needs_backslash: fn(char) -> bool) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        if needs_backslash(ch) {
            escaped.push('\\');
            escaped.push(ch);
        } else {
            push_encoding_line_ending(&mut escaped, ch);
        }
    }
    escaped
}

/// Escapes a [link destination](https://spec.commonmark.org/0.31.2/#link-destination),
/// switching to the pointy bracket form where the bare form cannot hold it. See
/// the links section of `overall_translation_approach.md`.
pub(crate) fn escape_link_destination(destination: &str) -> String {
    let escaped = escape_inline_string(destination, |ch| matches!(ch, '\\' | '&' | '(' | ')'));
    let needs_pointy_brackets = escaped.is_empty()
        || escaped.starts_with('<')
        || escaped.contains(|ch: char| ch.is_ascii_control() || ch == ' ');
    if !needs_pointy_brackets {
        return escaped;
    }

    let mut bracketed = String::with_capacity(escaped.len() + 4);
    bracketed.push('<');
    for ch in escaped.chars() {
        if matches!(ch, '<' | '>') {
            bracketed.push('\\');
        }
        bracketed.push(ch);
    }
    bracketed.push('>');
    bracketed
}

/// Escapes a [link title](https://spec.commonmark.org/0.31.2/#link-title) for
/// writing between double quotes.
pub(crate) fn escape_link_title(title: &str) -> String {
    escape_inline_string(title, |ch| matches!(ch, '"' | '\\' | '&'))
}

/// Escapes an `alt` attribute value as the plain text of an
/// [image description](https://spec.commonmark.org/0.31.2/#images), giving it
/// the escapes `dom_walker::escape_if_needed` gives a text node.
pub(crate) fn escape_image_description(alt: &str) -> String {
    let escaped = escape_inline_string(alt, |ch| {
        matches!(ch, '\\' | '&' | '*' | '_' | '`' | '[' | ']')
    });
    // `should_escape_html_like_sequence` reads a CDATA section in the spelling
    // the backslash pass leaves behind, so that pass comes first.
    escape_html(Cow::Owned(escaped)).into_owned()
}

/// Whether `text` opens an
/// [ATX heading](https://spec.commonmark.org/0.31.2/#atx-headings).
///
/// The `#` run needs no text after it: the end of `text` closes the marker
/// just as a space does, so text holding nothing but the run is a heading too.
/// That reading holds only where `text` runs to the end of its line; the
/// caller owes it that, and gets a yes for a run which more inline content
/// follows.
///
/// A run longer than the six levels CommonMark defines opens no heading but
/// still answers yes. Both cases cost the caller a backslash it does not need,
/// never a wrong translation.
pub(crate) fn is_markdown_atx_heading(text: &str) -> bool {
    let mut is_prev_ch_hash = false;
    for ch in text.chars() {
        if ch == '#' {
            is_prev_ch_hash = true;
        } else if ch == ' ' {
            return is_prev_ch_hash;
        } else {
            return false;
        }
    }
    is_prev_ch_hash
}

/// The byte offset of the `.` or `)` which makes `text` open an
/// [ordered list item](https://spec.commonmark.org/0.31.2/#list-items), if it
/// does.
///
/// The delimiter needs no text after it: the end of `text` closes it just as a
/// space does. As for [`is_markdown_atx_heading`], that reading holds only
/// where `text` runs to the end of its line, which the caller owes it.
///
/// Only the first delimiter after the digits can open a list -- a second one
/// closes the first, leaving it no whitespace terminator. A digit run longer
/// than the nine places CommonMark allows opens no list but still answers
/// with an offset, which costs the caller a backslash it does not need.
///
/// A marker counts ASCII digits alone, so a Unicode numeric character is
/// ordinary text and the run holding it opens nothing.
pub(crate) fn index_of_markdown_ordered_item_delimiter(text: &str) -> Option<usize> {
    let mut is_prev_ch_digit = false;
    let mut delimiter_byte_offset = 0;
    let mut is_prev_ch_delimiter = false;
    for (byte_offset, ch) in text.char_indices() {
        if ch.is_ascii_digit() {
            if is_prev_ch_delimiter {
                return None;
            }
            is_prev_ch_digit = true;
        } else if ch == '.' || ch == ')' {
            if !is_prev_ch_digit || is_prev_ch_delimiter {
                return None;
            }
            delimiter_byte_offset = byte_offset;
            is_prev_ch_delimiter = true;
        } else if ch == ' ' {
            if is_prev_ch_delimiter {
                return Some(delimiter_byte_offset);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }
    is_prev_ch_delimiter.then_some(delimiter_byte_offset)
}

#[cfg(test)]
mod tests {
    use super::{
        escape_html, escape_image_description, escape_link_destination, escape_link_title,
        index_of_markdown_ordered_item_delimiter,
    };

    #[test]
    fn escapes_basic_tags() {
        assert_eq!(escape_html("<p>".into()), "\\<p>");
        assert_eq!(escape_html("</p>".into()), "\\</p>");
        assert_eq!(
            escape_html("<div>content</div>".into()),
            "\\<div>content\\</div>"
        );
    }

    #[test]
    fn escapes_misc_sequences() {
        assert_eq!(escape_html("<!-- comment -->".into()), "\\<!-- comment -->");
        assert_eq!(
            escape_html("<?xml version=\"1.0\"?>".into()),
            "\\<?xml version=\"1.0\"?>"
        );
        assert_eq!(escape_html("<!DOCTYPE html>".into()), "\\<!DOCTYPE html>");
        assert_eq!(escape_html("<pre".into()), "\\<pre");
    }

    #[test]
    fn leaves_non_html_sequences() {
        assert_eq!(escape_html("< not html".into()), "< not html");
        assert_eq!(escape_html("<123>".into()), "<123>");
        assert_eq!(escape_html("< >".into()), "< >");
    }

    /// An [email autolink](https://spec.commonmark.org/0.31.2/#email-autolink)
    /// may follow its `<` with a digit, a `/` or a symbol, where a tag needs a
    /// letter.
    #[test]
    fn escapes_an_email_autolink() {
        assert_eq!(escape_html("<1foo@bar.com>".into()), "\\<1foo@bar.com>");
        assert_eq!(escape_html("</1@b.co>".into()), "\\</1@b.co>");
        assert_eq!(escape_html("<+a@b.co>".into()), "\\<+a@b.co>");
    }

    /// Each of these breaks the email autolink grammar in one place.
    #[test]
    fn leaves_a_near_miss_email_autolink() {
        for text in [
            "<@b.co>",
            "<1@>",
            "<1@-b.co>",
            "<1@b-.co>",
            "<1@b..co>",
            "<1@b.co.>",
            "<1 @b.co>",
            "<\\_a@b.co>",
        ] {
            assert_eq!(escape_html(text.into()), text);
        }
        let longest_label = "a".repeat(63);
        let autolink = format!("<1@{longest_label}.co>");
        assert_eq!(
            escape_html(autolink.as_str().into()),
            format!("\\{autolink}")
        );
        let too_long = format!("<1@{longest_label}a.co>");
        assert_eq!(escape_html(too_long.as_str().into()), too_long);
    }

    /// The text which follows may finish a tag or an autolink which the end of
    /// the string cuts short; `a_sequence_finished_by_a_sibling_stays_text` in
    /// `tests/basic_tests.rs` shows where that text comes from.
    #[test]
    fn escapes_a_sequence_cut_short() {
        for text in ["a <", "</", "<1", "<1@", "<1@b.co"] {
            assert_eq!(escape_html(text.into()), text.replacen('<', "\\<", 1));
        }
        assert_eq!(escape_html("a <= b".into()), "a <= b");
    }

    /// A [CDATA section](https://spec.commonmark.org/0.31.2/#cdata-section) is
    /// raw HTML, so one written as literal text needs the escape every other
    /// tag gets. Left unescaped it passes through to the HTML output, where a
    /// renderer reads it as a bogus comment and shows nothing;
    /// `a_literal_cdata_section_is_lost_unescaped` in `tests/basic_tests.rs`
    /// records that.
    ///
    /// No caller in this crate reaches this arm: `escape_if_needed` escapes the
    /// brackets first, so a CDATA section arrives in the spelling
    /// `leaves_an_escaped_cdata_section` covers. The escape is what a caller
    /// which skipped that pass would need.
    #[test]
    fn escapes_a_literal_cdata_section() {
        assert_eq!(
            escape_html("<![CDATA[character data]]>".into()),
            "\\<![CDATA[character data]]>"
        );
    }

    /// `<!\[` opens neither a CDATA section, which needs a literal `[CDATA[`,
    /// nor a [declaration](https://spec.commonmark.org/0.31.2/#declaration),
    /// which needs an ASCII letter after the `<!`. The escaped spelling is
    /// already literal text, whatever follows it, and it is the one the crate
    /// emits, so escaping it again would put a stray backslash in every CDATA
    /// section htmd converts.
    #[test]
    fn leaves_an_escaped_cdata_section() {
        for text in ["<!\\[CDATA\\[already escaped]]>", "<!\\[CDATA"] {
            assert_eq!(escape_html(text.into()), text);
        }
    }

    /// A backslash in a destination escapes whatever follows it, the `)` or `>`
    /// which closes the destination included, so it is escaped itself.
    /// `a_link_destination_escapes_its_backslashes` in `tests/link_tests.rs`
    /// holds what an unescaped one costs.
    #[test]
    fn escapes_a_link_destination_backslash() {
        assert_eq!(r"a\\", escape_link_destination(r"a\"));
        assert_eq!(r"a\\\(b", escape_link_destination(r"a\(b"));
        assert_eq!("ab", escape_link_destination("ab"));
    }

    /// Only the pointy bracket form needs its `<` and `>` escaped; see the
    /// links section of `overall_translation_approach.md`.
    #[test]
    fn brackets_a_link_destination_the_bare_form_cannot_hold() {
        assert_eq!("<>", escape_link_destination(""));
        assert_eq!(r"<\<a\>>", escape_link_destination("<a>"));
        assert_eq!(r"<a b\<c\>>", escape_link_destination("a b<c>"));
        assert_eq!("<a\tb>", escape_link_destination("a\tb"));
        assert_eq!("a<b>c", escape_link_destination("a<b>c"));
        // An encoded line ending is no longer a control character.
        assert_eq!("a&#10;b", escape_link_destination("a\nb"));
    }

    #[test]
    fn escapes_a_link_destination_ampersand() {
        assert_eq!(r"a\&copy;", escape_link_destination("a&copy;"));
    }

    /// A backslash in a title escapes whatever follows it, the `"` which closes
    /// the title included, so it is escaped itself.
    #[test]
    fn escapes_a_title_backslash() {
        assert_eq!(r#"a\\"#, escape_link_title(r#"a\"#));
        assert_eq!(r#"a\\\"b"#, escape_link_title(r#"a\"b"#));
    }

    #[test]
    fn escapes_a_title_ampersand_and_line_endings() {
        assert_eq!(r"a\&copy;", escape_link_title("a&copy;"));
        assert_eq!("a&#13;&#10;&#10;", escape_link_title("a\r\n\n"));
    }

    #[test]
    fn escapes_an_image_description_as_text() {
        assert_eq!(
            r"a\&b\\c\*d\_e\`f\[g\]h&#10;\<em>",
            escape_image_description("a&b\\c*d_e`f[g]h\n<em>")
        );
    }

    #[test]
    fn test_index_of_markdown_ordered_item_delimiter() {
        assert_eq!(None, index_of_markdown_ordered_item_delimiter("16.1¾ "));
        assert_eq!(Some(1), index_of_markdown_ordered_item_delimiter("1. "));
        assert_eq!(Some(2), index_of_markdown_ordered_item_delimiter("12. "));
        assert_eq!(Some(5), index_of_markdown_ordered_item_delimiter("12345. "));
        assert_eq!(Some(1), index_of_markdown_ordered_item_delimiter("1. \n"));
        assert_eq!(None, index_of_markdown_ordered_item_delimiter(". "));
        assert_eq!(None, index_of_markdown_ordered_item_delimiter("abc. "));
        assert_eq!(None, index_of_markdown_ordered_item_delimiter("1 . "));
        assert_eq!(None, index_of_markdown_ordered_item_delimiter(" 1. "));
        assert_eq!(None, index_of_markdown_ordered_item_delimiter("1.a "));
        assert_eq!(Some(1), index_of_markdown_ordered_item_delimiter("1."));
        assert_eq!(Some(1), index_of_markdown_ordered_item_delimiter("1)"));
    }

    /// An [ordered list marker](https://spec.commonmark.org/0.31.2/#ordered-list-marker)
    /// counts ASCII digits alone. A Unicode numeric character -- U+00BD (½),
    /// general category No, or U+0661 (١), an Arabic-Indic digit -- is ordinary
    /// text, and a run holding one opens no list to escape.
    #[test]
    fn a_unicode_numeric_opens_no_ordered_item() {
        assert_eq!(None, index_of_markdown_ordered_item_delimiter("2½. text"));
        assert_eq!(None, index_of_markdown_ordered_item_delimiter("2½"));
        assert_eq!(None, index_of_markdown_ordered_item_delimiter("1١. text"));
        // With only ASCII digits ahead of it, the delimiter offset counts them;
        // a multibyte character after the delimiter leaves the offset alone.
        assert_eq!(Some(2), index_of_markdown_ordered_item_delimiter("12. ½"));
    }
}
