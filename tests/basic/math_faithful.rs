use pretty_assertions::assert_eq;
use pulldown_cmark::Options as CommonMarkOptions;

use crate::common::{convert_faithful, convert_pure, render_markdown};

/// pulldown-cmark reads `$a$$b$` as two math expressions, so adjacent math
/// spans, unlike other runs of inline elements, are neither merged nor written
/// as HTML.
#[test]
fn adjacent_math_spans_stay_math() {
    for (html, markdown) in [
        (
            r#"<p><span class="math math-inline">a</span><span class="math math-inline">b</span></p>"#,
            "$a$$b$",
        ),
        (
            r#"<p><span class="math math-display">a</span><span class="math math-display">b</span></p>"#,
            "$$a$$$$b$$",
        ),
        (
            r#"<p><span class="math math-inline">a</span><span class="math math-display">b</span></p>"#,
            "$a$$$b$$",
        ),
        (
            r#"<p><span class="math math-display">a</span><span class="math math-inline">b</span></p>"#,
            "$$a$$$b$",
        ),
    ] {
        assert_eq!(markdown, convert_faithful(html).unwrap());
        assert_eq!(markdown, convert_pure(html).unwrap());
        assert_eq!(
            format!("{html}\n"),
            render_markdown(markdown, CommonMarkOptions::ENABLE_MATH)
        );
    }
}

#[test]
fn math() {
    assert_eq!(
        "$x^2$",
        convert_faithful(r#"<p><span class="math math-inline">x^2</span></p>"#).unwrap()
    );

    assert_eq!(
        "$$x^2$$",
        convert_faithful(r#"<p><span class="math math-display">x^2</span></p>"#).unwrap()
    );

    // Test escaping -- values inside math should not be escaped.
    assert_eq!(
        "$${a}_1, b_{2}, a*1, b*2, [a](b), 3 <a> b, a \\; b$$",
        convert_faithful(r#"<p><span class="math math-display">{a}_1, b_{2}, a*1, b*2, [a](b), 3 &lt;a&gt; b, a \; b</span></p>"#).unwrap()
    );
}

/// Faithful translations of math which `render_markdown` must read back as the
/// HTML it came from.
fn assert_math_round_trips(html: &str, markdown: &str) {
    assert_eq!(markdown, convert_faithful(html).unwrap());
    assert_eq!(
        format!("{html}\n"),
        render_markdown(
            markdown,
            CommonMarkOptions::ENABLE_MATH | CommonMarkOptions::ENABLE_TABLES
        )
    );
}

/// pulldown-cmark reads `$$$$` as display math holding nothing, but inline math
/// cannot be empty: it reads `$$` as display math's delimiter.
#[test]
fn empty_math_span() {
    assert_math_round_trips(r#"<p><span class="math math-display"></span></p>"#, "$$$$");
    assert_math_round_trips(
        r#"<p><span class="math math-inline"></span></p>"#,
        r#"<span class="math math-inline"></span>"#,
    );
    assert_eq!(
        "",
        convert_pure(r#"<p><span class="math math-inline"></span></p>"#).unwrap()
    );
}

/// Inline math cannot begin or end with whitespace, so it is trimmed; display
/// math keeps its whitespace.
#[test]
fn math_span_whitespace() {
    assert_eq!(
        "$x$",
        convert_faithful("<p><span class=\"math math-inline\"> \u{b}x\t</span></p>").unwrap()
    );
    assert_math_round_trips(
        r#"<p><span class="math math-display"> x </span></p>"#,
        "$$ x $$",
    );
    // Trimming a control space leaves a trailing `\`, which would escape the
    // closing `$`; display math keeps the space, so needs no HTML.
    assert_math_round_trips(
        r#"<p><span class="math math-inline">a\ </span></p>"#,
        r#"<span class="math math-inline">a\\ </span>"#,
    );
    assert_math_round_trips(
        r#"<p><span class="math math-display">a\ </span></p>"#,
        r"$$a\ $$",
    );
}

/// Math which pulldown-cmark would not read back between `$`s is written as
/// HTML. A backslash escapes the character after it, so an escaped `\` escapes
/// nothing else.
#[test]
fn unwritable_math_span() {
    for (math, markdown) in [
        // An unescaped `$` ends the expression early.
        ("a$b", r"a\$b"),
        (r"a\\$b", r"a\\\\\$b"),
        // A trailing `\` escapes the closing `$`.
        (r"a\", r"a\\"),
        // pulldown-cmark pairs delimiters only around balanced braces.
        ("}a{", "}a{"),
        ("a{", "a{"),
        (r"\\{a", r"\\\\{a"),
    ] {
        assert_math_round_trips(
            &format!(r#"<p><span class="math math-inline">{math}</span></p>"#),
            &format!(r#"<span class="math math-inline">{markdown}</span>"#),
        );
    }
    for math in [r"a\$b", r"a\\", "{a}", r"\{a", r"a\}"] {
        assert_math_round_trips(
            &format!(r#"<p><span class="math math-inline">{math}</span></p>"#),
            &format!("${math}$"),
        );
    }
}

/// Only a span whose sole attribute is its math class and which holds nothing
/// or a single text node is math; any other is translated like an ordinary
/// span.
#[test]
fn span_not_of_the_math_form() {
    let html = r#"<p><span class="math math-inline">a<em>b</em></span></p>"#;
    assert_math_round_trips(html, r#"<span class="math math-inline">a*b*</span>"#);
    assert_eq!("a*b*", convert_pure(html).unwrap());
    let html = r#"<p><span class="math math-inline">a<!--c--></span></p>"#;
    assert_math_round_trips(html, r#"<span class="math math-inline">a<!--c--></span>"#);
    assert_eq!("a", convert_pure(html).unwrap());
    let html = r#"<p><span class="math math-inline" id="x">a</span></p>"#;
    assert_math_round_trips(html, r#"<span class="math math-inline" id="x">a</span>"#);
    assert_eq!("a", convert_pure(html).unwrap());
}

/// pulldown-cmark reads `\|` in a table cell as `|` even inside math, so a `|`
/// there takes the escape it takes in text.
#[test]
fn math_span_in_a_table_cell() {
    assert_math_round_trips(
        concat!(
            "<table><thead><tr><th>h</th></tr></thead><tbody>\n",
            r#"<tr><td><span class="math math-inline">\|x\| = a|b</span></td></tr>"#,
            "\n</tbody></table>"
        ),
        concat!(
            "| h                |\n",
            "| ---------------- |\n",
            r"| $\\|x\\| = a\|b$ |"
        ),
    );
}

/// pulldown-cmark looks for math delimiters in all text, so a text `$` is
/// escaped; without the escape, this reads back as math holding `5,`.
#[test]
fn dollar_sign_in_text() {
    assert_math_round_trips("<p>$5,$6</p>", r"\$5,\$6");
    assert_eq!(r"\$5,\$6", convert_pure("<p>$5,$6</p>").unwrap());
}
