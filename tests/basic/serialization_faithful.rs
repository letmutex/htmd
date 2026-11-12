use indoc::indoc;
use pretty_assertions::assert_eq;

#[cfg(feature = "rcdom")]
use crate::common::tree;
use crate::common::{convert_faithful, convert_pure, round_trip};

/// A pair of adjacent emphases is merged so that it does not go out as
/// `*a**b*`, which CommonMark reads as one emphasis holding `a**b`. Merging is
/// a pure-mode spelling, though: it writes two elements as one, which faithful
/// mode owes the reader as HTML instead.
#[test]
fn faithful_mode_keeps_adjacent_emphasis_apart() {
    assert_eq!(
        "<em>a</em><em>b</em>",
        convert_faithful("<p><em>a</em><em>b</em></p>").unwrap()
    );
    assert_eq!(
        "<i>a</i><em>b</em>",
        convert_faithful("<p><i>a</i><em>b</em></p>").unwrap()
    );
    assert_eq!(
        "<b>a</b><strong>b</strong>",
        convert_faithful("<p><b>a</b><strong>b</strong></p>").unwrap()
    );
}

/// A pair of emphases differing only in prefix would go out as `*a**b*` too, so
/// faithful mode writes it as HTML, which the serializer writes without the
/// prefix. Only a tree built elsewhere can hold a prefixed HTML element.
#[cfg(feature = "rcdom")]
#[test]
fn faithful_mode_keeps_emphases_differing_only_in_prefix_apart() {
    let tree = tree::document(vec![tree::element(
        "p",
        vec![
            tree::prefixed_element("x", "em", vec![tree::text("a")]),
            tree::prefixed_element("y", "em", vec![tree::text("b")]),
        ],
    )]);

    assert_eq!("<em>a</em><em>b</em>", convert_faithful(&tree).unwrap());
}

/// Writing the two code spans one after the other is no answer either: the
/// backtick strings of ``` `a``b` ``` pair outside in, making one span whose
/// content is ``` a``b ```. Faithful mode writes both as HTML.
#[test]
fn faithful_mode_keeps_adjacent_code_spans_apart() {
    assert_eq!(
        "<code>a</code><code>b</code>",
        convert_faithful("<p><code>a</code><code>b</code></p>").unwrap()
    );
}

/// An element CommonMark cannot spell is already written as a raw HTML inline,
/// so merging a pair of them buys nothing and loses an element.
#[test]
fn faithful_mode_keeps_adjacent_untranslated_elements_apart() {
    assert_eq!(
        "<sub>a</sub><sub>b</sub>",
        convert_faithful("<p><sub>a</sub><sub>b</sub></p>").unwrap()
    );
    assert_eq!(
        r#"<abbr title="t">a</abbr><abbr title="t">b</abbr>"#,
        convert_faithful(r#"<p><abbr title="t">a</abbr><abbr title="t">b</abbr></p>"#).unwrap()
    );
    assert_eq!(
        r#"<span class="x">a</span><span class="x">b</span>"#,
        convert_faithful(r#"<p><span class="x">a</span><span class="x">b</span></p>"#).unwrap()
    );
}

#[test]
fn faithful_mode_inline() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<p>
                <img src="one.png" alt="yyy" title="zzz" scale="50%">
                <em bar>Testing</em>
                <strong foo>Testing</strong>
                <a href="http://foo.com" bar>link</a>
                <code class="not-a-language">code</code>
                <br foo>
            </p>"#
        ))
        .unwrap(),
        indoc!(
            r#"<img src="one.png" alt="yyy" title="zzz" scale="50%"> <em bar="">Testing</em> <strong foo="">Testing</strong> <a href="http://foo.com" bar="">link</a> <code class="not-a-language">code</code> <br foo="">"#
        )
    );
}

#[test]
fn faithful_mode_hr() {
    assert_eq!(
        convert_faithful(indoc!(r#"<hr bar>"#)).unwrap(),
        indoc!(r#"<hr bar="">"#)
    );
}

#[test]
fn faithful_mode_blockquote() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<blockquote style="foo">
            <em>Testing</em>

            <blockquote>Nested</blockquote>
        </blockquote>"#
        ))
        .unwrap(),
        indoc!(
            r#"<blockquote style="foo">
                <em>Testing</em>
            &#10;    <blockquote>Nested</blockquote>
            </blockquote>"#
        )
    );
}

#[test]
fn faithful_mode_h1() {
    assert_eq!(
        convert_faithful(indoc!(r#"<h1 class="foo">Heading</h1>"#)).unwrap(),
        indoc!(r#"<h1 class="foo">Heading</h1>"#)
    );
}

/// Step 1 of the headings approach in `translation_approach.md`: CommonMark has
/// no syntax for a heading's attributes, so faithful mode serializes a heading
/// holding any, even one with an empty value. Pure mode drops them.
#[test]
fn heading_attributes() {
    assert_eq!(
        r#"<h1 class="">a</h1>"#,
        convert_faithful(r#"<h1 class="">a</h1>"#).unwrap()
    );
    assert_eq!(
        r#"<h2 id="">a</h2>"#,
        convert_faithful("<h2 id>a</h2>").unwrap()
    );
    assert_eq!("# a", convert_pure(r#"<h1 class="x">a</h1>"#).unwrap());
}

#[test]
fn faithful_mode_p() {
    assert_eq!(
        convert_faithful(indoc!(r#"<p dir="ltr">Test 1</p>"#)).unwrap(),
        indoc!(r#"<p dir="ltr">Test 1</p>"#)
    );
}

#[test]
fn faithful_mode_ol1() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<ol>
            <li>Test 1</li>
            <li foo>Test 2</li>
            <li>Test 3</li>
        </ol>"#
        ))
        .unwrap(),
        indoc!(
            r#"<ol>
                <li>Test 1</li>
                <li foo="">Test 2</li>
                <li>Test 3</li>
            </ol>"#
        )
    );
}

#[test]
fn faithful_mode_ol2() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<ol foo>
            <li>Test</li>
        </ol>"#
        ))
        .unwrap(),
        indoc!(
            r#"<ol foo="">
                <li>Test</li>
            </ol>"#
        )
    );
}

#[test]
fn faithful_mode_comment() {
    assert_eq!(
        convert_faithful(indoc!(r#"<!-- Test -->"#)).unwrap(),
        indoc!(r#"<!-- Test -->"#)
    );
}

#[test]
fn faithful_mode_html() {
    let html = indoc!(
        r#"<details>
            <summary>Test

                1</summary>
            Test 2
        </details>"#
    );
    let md = convert_faithful(html).unwrap();
    assert_eq!(
        indoc!(
            r#"<details>
                <summary>Test
            &#10;        1</summary>
                Test 2
            </details>"#
        ),
        md
    );
}

#[test]
fn faithful_mode_table() {
    assert_eq!(
        convert_faithful(indoc!(
            r#"<table>
            <tr>
                <th>Header 1</th>
                <th>Header 2</th>
            </tr>
            <tr>
                <td foo>Cell 1</td>
                <td>Cell 2</td>
            </tr>
            <tr>
                <td>Cell 3</td>
                <td>Cell 4</td>
            </tr>
        </table>
"#
        ))
        .unwrap(),
        indoc!(
            r#"<table>
            <tbody><tr>
                <th>Header 1</th>
                <th>Header 2</th>
            </tr>
            <tr>
                <td foo="">Cell 1</td>
                <td>Cell 2</td>
            </tr>
            <tr>
                <td>Cell 3</td>
                <td>Cell 4</td>
            </tr>
        </tbody></table>"#
        )
    );
}

/// CommonMark has no caption, so faithful mode always writes the element as
/// HTML — and a `<caption>` is only valid inside a `<table>`, so the whole table
/// is serialized around it.
#[test]
fn faithful_mode_serializes_a_table_with_a_caption() {
    let html = concat!(
        r#"<table><caption><span class="label">Caption</span></caption>"#,
        "<tr><th>Header</th></tr><tr><td>Cell</td></tr></table>"
    );
    // html5ever inserts the `<tbody>` around the rows.
    let expected = concat!(
        r#"<table><caption><span class="label">Caption</span></caption>"#,
        "<tbody><tr><th>Header</th></tr><tr><td>Cell</td></tr></tbody></table>"
    );

    assert_eq!(expected, convert_faithful(html).unwrap());
    // The point of serializing the whole table: the `<caption>` comes back as
    // an element rather than as the bare text an HTML block outside the table
    // would give. The block is the whole document here, so pulldown-cmark
    // writes it without the trailing line ending `assert_round_trips` expects.
    assert_eq!(expected, round_trip(expected));
}

#[test]
fn faithful_mode_nested_inline_html() {
    assert_eq!(
        convert_faithful("<p>Nested <foo><bar><em>content</em></bar></foo></p>").unwrap(),
        "Nested <foo><bar>*content*</bar></foo>"
    );
}
