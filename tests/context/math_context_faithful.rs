use pretty_assertions::assert_eq;

use crate::common::{convert_faithful, convert_pure};

/// The contents of a math span are literal text, but a line ending there would
/// end the span and begin another block. LaTeX ignores whitespace, so each line
/// ending becomes a space.
#[test]
fn a_math_span_replaces_its_line_endings_with_spaces() {
    assert_eq!(
        "x$a b$y",
        convert_faithful("<p>x<span class=\"math math-inline\">a\nb</span>y</p>").unwrap()
    );
    assert_eq!(
        "$$a b$$",
        convert_faithful("<p><span class=\"math math-display\">a\nb</span></p>").unwrap()
    );
    // A carriage return reaches the span only as a character reference, the
    // parser having folded any literal CRLF into a line feed. A CRLF pair is
    // one line ending, so it becomes one space...
    assert_eq!(
        "$a b$",
        convert_faithful("<p><span class=\"math math-inline\">a&#13;&#10;b</span></p>").unwrap()
    );
    // ...while a lone carriage return is a line ending of its own.
    assert_eq!(
        "$a b$",
        convert_faithful("<p><span class=\"math math-inline\">a&#13;b</span></p>").unwrap()
    );
    // An escaped line ending is LaTeX's control space, which an escaped space
    // spells as well.
    assert_eq!(
        r"$a\ b$",
        convert_faithful("<p><span class=\"math math-inline\">a\\\nb</span></p>").unwrap()
    );
}

/// Joining a math span's lines would extend a LaTeX comment over every line
/// after it, so each comment is replaced first. As in LaTeX, a comment runs
/// through its line ending and the whitespace beginning the next line; a space
/// replaces it, since that ends a control word as the comment did.
#[test]
fn a_math_span_strips_its_comments() {
    assert_eq!(
        r"$\alpha x$",
        convert_faithful("<p><span class=\"math math-inline\">\\alpha%c\n  \tx</span></p>")
            .unwrap()
    );
    // The space is wrong in text mode, where the comment joined two letters.
    assert_eq!(
        r"$\text{fo o}$",
        convert_faithful("<p><span class=\"math math-inline\">\\text{fo%c\no}</span></p>").unwrap()
    );
    // Every line ending bounds a comment: a CRLF pair and a lone carriage
    // return included.
    assert_eq!(
        "$$a b c d$$",
        convert_faithful(
            "<p><span class=\"math math-display\">a%x&#13;&#10;b&#13;c%y&#13;d</span></p>"
        )
        .unwrap()
    );
    // A comment on the last line still leaves its space, which only inline math
    // trims.
    assert_eq!(
        "$$a $$",
        convert_faithful("<p><span class=\"math math-display\">a%b</span></p>").unwrap()
    );
    // `\%` is a literal percent sign...
    assert_eq!(
        r"$a\%b$",
        convert_faithful(r#"<p><span class="math math-inline">a\%b</span></p>"#).unwrap()
    );
    // ...but `\\` is a control symbol, leaving the `%` after it unescaped.
    assert_eq!(
        r"$a\\$",
        convert_faithful(r#"<p><span class="math math-inline">a\\%b</span></p>"#).unwrap()
    );
    // A span which is all comment holds no math. Inline math cannot be empty,
    // so the span goes out as HTML, holding the space which replaced the
    // comment.
    assert_eq!(
        r#"<span class="math math-inline"> </span>"#,
        convert_faithful(r#"<p><span class="math math-inline">%a</span></p>"#).unwrap()
    );
}

/// A math span written as HTML holds its comment-stripped, single-line source:
/// serializing collapses the line ending of the original, which would leave
/// `d` inside the comment.
#[test]
fn an_unwritable_math_span_strips_its_comments() {
    let html = "<p><span class=\"math math-inline\">a$b%c\nd</span></p>";
    assert_eq!(
        r#"<span class="math math-inline">a\$b d</span>"#,
        convert_faithful(html).unwrap()
    );
    assert_eq!(r"a\$b d", convert_pure(html).unwrap());
}
