use std::cmp::Ordering;

use super::unicode_table::PUNCTUATION_OR_SYMBOL;

/// A [Unicode punctuation character](https://spec.commonmark.org/0.31.2/#unicode-punctuation-character):
/// a character in a general category of `P` (punctuation) or `S` (symbol).
pub(crate) fn is_unicode_punctuation(ch: char) -> bool {
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

    /// Every non-ASCII code point whose general category group is `P` or `S`, as merged inclusive
    /// ranges: what `unicode_punctuation.rs` is supposed to contain, classified independently of
    /// it by `unicode-properties` (a dev-dependency, so the crate itself carries no Unicode data
    /// beyond the table).
    fn punctuation_or_symbol_from_unicode() -> Vec<(char, char)> {
        use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};

        let mut ranges: Vec<(char, char)> = Vec::new();
        for ch in (0x80..=char::MAX as u32).filter_map(char::from_u32) {
            if !matches!(
                ch.general_category_group(),
                GeneralCategoryGroup::Punctuation | GeneralCategoryGroup::Symbol
            ) {
                continue;
            }
            match ranges.last_mut() {
                Some((_, high)) if *high as u32 + 1 == ch as u32 => *high = ch,
                _ => ranges.push((ch, ch)),
            }
        }
        ranges
    }

    /// The other tests compare the table against itself, so they pass just as happily on a table
    /// that dropped or gained a range. Check the data itself against Unicode instead: a
    /// regeneration that went wrong, or a hand edit of the "do not edit by hand" file, fails here.
    #[test]
    fn table_matches_unicode_general_categories() {
        let expected = punctuation_or_symbol_from_unicode();
        let missing: Vec<_> = expected
            .iter()
            .filter(|range| !PUNCTUATION_OR_SYMBOL.contains(range))
            .collect();
        let extra: Vec<_> = PUNCTUATION_OR_SYMBOL
            .iter()
            .filter(|range| !expected.contains(range))
            .collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "`PUNCTUATION_OR_SYMBOL` disagrees with the Unicode general categories: \
             missing {missing:?}, extra {extra:?}. If this is a new Unicode release, \
             regenerate the table with `cargo test print_punctuation_or_symbol_table \
             -- --ignored --nocapture`.",
        );
    }

    /// The generator for `unicode_punctuation.rs`: prints the table in source form. Run it with
    /// `cargo test print_punctuation_or_symbol_table -- --ignored --nocapture` and paste the
    /// output over the old table.
    #[test]
    #[ignore = "generates the table rather than checking it"]
    fn print_punctuation_or_symbol_table() {
        let ranges = punctuation_or_symbol_from_unicode();
        println!(
            "pub(super) static PUNCTUATION_OR_SYMBOL: [(char, char); {}] = [",
            ranges.len()
        );
        for (low, high) in ranges {
            println!(
                "    ('\\u{{{:04X}}}', '\\u{{{:04X}}}'),",
                low as u32, high as u32
            );
        }
        println!("];");
    }
}
