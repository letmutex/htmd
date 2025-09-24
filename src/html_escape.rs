use std::borrow::Cow;

/// Escape sequences that Markdown would treat as raw HTML so that they stay as literal text.
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

fn should_escape_html_like_sequence(fragment: &str) -> bool {
    let mut chars = fragment.chars();
    let Some('<') = chars.next() else {
        return false;
    };

    let Some(next) = chars.next() else {
        return false;
    };

    match next {
        '!' => {
            let rest = chars.as_str();
            !(rest.starts_with("[CDATA[") || rest.starts_with("\\[CDATA\\["))
        }
        '?' => true,
        '/' => chars.next().is_some_and(|c| c.is_ascii_alphabetic()),
        c if c.is_ascii_alphabetic() => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::escape_html;

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

    #[test]
    fn leaves_cdata_sections() {
        assert_eq!(
            escape_html("<![CDATA[character data]]>".into()),
            "<![CDATA[character data]]>"
        );
        assert_eq!(
            escape_html("<!\\[CDATA\\[already escaped]]>".into()),
            "<!\\[CDATA\\[already escaped]]>"
        );
    }
}
