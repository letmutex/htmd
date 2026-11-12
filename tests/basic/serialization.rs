use pretty_assertions::assert_eq;
use pulldown_cmark::Options as CommonMarkOptions;

#[cfg(feature = "rcdom")]
use crate::common::tree;
use crate::common::{convert_pure, render_markdown};

/// Pure mode keeps the merge: it has no HTML to fall back on.
#[test]
fn pure_mode_merges_adjacent_inline_elements() {
    assert_eq!("*ab*", convert_pure("<p><em>a</em><em>b</em></p>").unwrap());
    assert_eq!(
        "ab",
        convert_pure("<p><sub>a</sub><sub>b</sub></p>").unwrap()
    );
}

/// No handler reads an element's prefix, and the serializer drops it, so a pair
/// of emphases differing only in prefix would go out as `*a**b*` too. Only a
/// tree built elsewhere can hold a prefixed HTML element.
#[cfg(feature = "rcdom")]
#[test]
fn elements_differing_only_in_prefix_merge() {
    let tree = tree::document(vec![tree::element(
        "p",
        vec![
            tree::prefixed_element("x", "em", vec![tree::text("a")]),
            tree::prefixed_element("y", "em", vec![tree::text("b")]),
        ],
    )]);

    assert_eq!("*ab*", convert_pure(&tree).unwrap());
}

// Faithful mode keeps an emphasis with nothing to emphasize as HTML; see rows 5
// and 6 of the "Emphasis and strong emphasis" table of
// `translation_approach.md`. Pure mode has no such spelling, so the element
// goes and only the whitespace it held survives, still separating the words
// around it.
#[test]
fn blank_emphasis_leaves_its_whitespace_behind() {
    assert_eq!("a b", convert_pure("<p>a<em> </em>b</p>").unwrap());
    assert_eq!("a b", convert_pure("<p>a<strong> </strong>b</p>").unwrap());
    assert_eq!(
        "a\u{a0}b",
        convert_pure("<p>a<em>&#160;</em>b</p>").unwrap()
    );
    assert_eq!("ab", convert_pure("<p>a<em></em>b</p>").unwrap());
}

/// Pure mode has no HTML to fall back on, so it keeps writing the caption as a
/// paragraph above the table.
#[test]
fn pure_mode_writes_a_caption_as_a_paragraph() {
    let html = concat!(
        "<table><caption>Caption</caption>",
        "<tr><th>Header</th></tr><tr><td>Cell</td></tr></table>"
    );

    assert_eq!(
        "Caption\n\n| Header |\n| ------ |\n| Cell   |",
        htmd::convert(html).unwrap()
    );
}

/// The blank line the caption is followed by is what keeps the table a block of
/// its own. A caption which translates to a list or a blockquote opens a block
/// which the rows below it would otherwise join as lazy continuation lines,
/// leaving the table dissolved into that block's text.
#[test]
fn pure_mode_separates_a_caption_from_the_table_it_sits_above() {
    for caption in ["<ul><li>a</li></ul>", "<blockquote>a</blockquote>"] {
        let html = format!(
            "<table><caption>{caption}</caption>\
             <tr><th>Header</th></tr><tr><td>Cell</td></tr></table>"
        );
        let markdown = htmd::convert(&html).unwrap();

        assert!(
            markdown.contains("\n\n| Header |"),
            "the caption ran into the table: {markdown:?}"
        );
        assert!(round_trip_pure_table(&html).contains("<table>"));
    }
}

/// [`round_trip`] in pure mode and with the
/// [GFM table extension](https://github.github.com/gfm/#tables-extension-),
/// which a Markdown table needs to be read back as a table at all.
fn round_trip_pure_table(html: &str) -> String {
    render_markdown(
        &htmd::convert(html).unwrap(),
        CommonMarkOptions::ENABLE_TABLES,
    )
}

#[test]
fn template_and_annotation_xml_conversion() {
    assert_eq!(
        "visible",
        convert_pure("<template><em>a</em><em>b</em></template><p>visible</p>").unwrap()
    );
    assert_eq!(
        "*ab*",
        convert_pure(
            "<math><annotation-xml encoding=\"text/html\"><p><em>a</em><em>b</em></p></annotation-xml></math>"
        )
        .unwrap()
    );
}
