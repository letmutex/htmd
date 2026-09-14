use std::cmp::Ordering;

use crate::{
    Element,
    element_handler::{
        HandlerResult, Handlers,
        element_util::{serialize_element, serialize_if_extra_attrs},
        unicode_punctuation::PUNCTUATION_OR_SYMBOL,
    },
    options::TranslationMode,
    text_util::{StripWhitespace, concat_strings},
};

/// A [Unicode punctuation character](https://spec.commonmark.org/0.31.2/#unicode-punctuation-character):
/// a character in a general category of `P` (punctuation) or `S` (symbol).
fn is_unicode_punctuation(ch: char) -> bool {
    // ASCII is the overwhelmingly common case, and there the two categories are exactly what
    // `is_ascii_punctuation` reports.
    if ch.is_ascii() {
        return ch.is_ascii_punctuation();
    }
    PUNCTUATION_OR_SYMBOL
        .binary_search_by(|&(low, high)| {
            if high < ch {
                Ordering::Less
            } else if low > ch {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

pub(super) fn emphasis_handler(
    handlers: &dyn Handlers,
    element: Element,
    marker: &str,
) -> Option<HandlerResult> {
    serialize_if_extra_attrs!(handlers, element, 0);
    let content = handlers.walk_children_content(element.node, element.context);
    if content.is_empty() {
        return None;
    }
    // Note: this is whitespace, NOT document whitespace, per the
    // [Commonmark spec](https://spec.commonmark.org/0.31.2/#emphasis-and-strong-emphasis).
    let (content, leading_whitespace) = content.strip_leading_whitespace();
    let (content, trailing_whitespace) = content.strip_trailing_whitespace();
    if content.is_empty() {
        return None;
    }
    // See the [spec](unsupported_html.md) section on Inline elements.
    if handlers.options().translation_mode == TranslationMode::Faithful
        && (content.starts_with(is_unicode_punctuation)
            || content.ends_with(is_unicode_punctuation))
    {
        return Some(HandlerResult::html(serialize_element(handlers, &element)));
    }

    let content = concat_strings!(
        leading_whitespace.unwrap_or(""),
        marker,
        content,
        marker,
        trailing_whitespace.unwrap_or("")
    );
    Some(content.into())
}

#[cfg(test)]
mod tests {
    use super::{PUNCTUATION_OR_SYMBOL, is_unicode_punctuation};

    /// `binary_search_by` needs the ranges sorted, and the generator is supposed to merge
    /// neighbours, so every range must start at least two code points past the previous end.
    #[test]
    fn table_is_sorted_and_merged() {
        assert!(PUNCTUATION_OR_SYMBOL[0].0 > '\u{7F}');
        for pair in PUNCTUATION_OR_SYMBOL.windows(2) {
            let [(low, high), (next_low, next_high)] = pair else {
                unreachable!()
            };
            assert!(low <= high, "{low:?}..={high:?}");
            assert!(next_low <= next_high, "{next_low:?}..={next_high:?}");
            assert!(
                (*high as u32) + 1 < *next_low as u32,
                "{high:?} and {next_low:?} should be one range",
            );
        }
    }

    #[test]
    fn agrees_with_a_linear_scan() {
        for cp in 0..=char::MAX as u32 {
            let Some(ch) = char::from_u32(cp) else {
                continue;
            };
            let linear = if ch.is_ascii() {
                ch.is_ascii_punctuation()
            } else {
                PUNCTUATION_OR_SYMBOL
                    .iter()
                    .any(|&(low, high)| low <= ch && ch <= high)
            };
            assert_eq!(is_unicode_punctuation(ch), linear, "{ch:?}");
        }
    }

    #[test]
    fn classifies_punctuation_and_symbols() {
        // ASCII `P` and `S`, including the ends of each ASCII run.
        for ch in "!/:@[`{~\"#$%&'()*+,-.;<=>?^_|\\".chars() {
            assert!(is_unicode_punctuation(ch), "{ch:?}");
        }
        // Non-ASCII `P`: inverted mark (Po), guillemet (Pi/Pf), em dash (Pd), Arabic comma (Po),
        // CJK bracket (Ps), supplementary-plane Aegean word separator (Po).
        for ch in "¡«»—،〔𐄁".chars() {
            assert!(is_unicode_punctuation(ch), "{ch:?}");
        }
        // Non-ASCII `S`: currency (Sc), modifier (Sk), math (Sm), other (So), and an emoji from
        // the supplementary planes.
        for ch in "£¢´×÷©🙂".chars() {
            assert!(is_unicode_punctuation(ch), "{ch:?}");
        }
    }

    #[test]
    fn rejects_everything_else() {
        // Letters (`L`), digits (`N`), and whitespace (`Z`).
        for ch in "aZ0é漢Ⅷ ".chars() {
            assert!(!is_unicode_punctuation(ch), "{ch:?}");
        }
        // Categories with no ASCII members that a "not a letter, digit, or space" test would
        // wrongly report as punctuation: combining marks (`Mn`, `Mc`), a format character (`Cf`),
        // a private-use character (`Co`), and an unassigned code point (`Cn`).
        for ch in ['\u{0301}', '\u{0903}', '\u{200D}', '\u{E000}', '\u{0378}'] {
            assert!(!is_unicode_punctuation(ch), "{ch:?}");
        }
        // The code points bracketing each non-ASCII range must not leak in.
        assert!(!is_unicode_punctuation('\u{00A0}'));
        assert!(!is_unicode_punctuation('\u{00AA}'));
        assert!(!is_unicode_punctuation(char::MAX));
    }
}
