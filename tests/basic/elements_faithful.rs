use indoc::indoc;
use pretty_assertions::assert_eq;

use htmd::{
    HtmlToMarkdown,
    options::{LinkStyle, Options, TranslationMode},
};

use crate::common::{convert_faithful, convert_faithful_setext, convert_pure};

#[test]
fn links_with_spaces() {
    let html = r#"
        <a href="https://example.com/Some Page.html">Example</a>
        "#;
    assert_eq!(
        "[Example](<https://example.com/Some Page.html>)",
        convert_faithful(html).unwrap(),
    )
}

#[test]
fn referenced_links_with_title() {
    let html = r#"
        <a href="https://example.com" title="Some title">Example</a>
        "#;
    let md = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build()
        .convert(html)
        .unwrap();
    assert_eq!(
        "[Example][1]\n\n[1]: https://example.com \"Some title\"",
        &md
    )
}

#[test]
fn consecutive_referenced_links_with_title() {
    let html = r#"
        <a href="https://example.com" title="Some title">Example</a><a href="https://example.com" title="Some title">Another example</a>
        "#;
    let md = HtmlToMarkdown::builder()
        .options(Options {
            link_style: LinkStyle::Referenced,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build()
        .convert(html)
        .unwrap();
    assert_eq!(
        indoc!(
            r#"
        [Example][1][Another example][2]

        [1]: https://example.com "Some title"
        [2]: https://example.com "Some title""#
        ),
        &md
    )
}

#[test]
fn images() {
    let html = r#"
        <img src="https://example.com" alt="" />
        <img src="https://example.com" alt="Image 1" />
        <img src="https://example.com" alt="Image 2" title="Hello" />
        "#;
    assert_eq!(
        "![](https://example.com) ![Image 1](https://example.com) \
            ![Image 2](https://example.com \"Hello\")",
        convert_faithful(html).unwrap(),
    )
}

#[test]
fn images_with_spaces_in_url() {
    let html = r#"
        <img src="https://example.com/Some Image.jpg" alt="" />
        "#;
    assert_eq!(
        "![](<https://example.com/Some Image.jpg>)",
        convert_faithful(html).unwrap(),
    )
}

#[test]
fn headings() {
    let html = r#"
        <h1>Heading 1</h1>
        <h2>Heading 2</h2>
        <h3>Heading 3</h3>
        <h4>Heading 4</h4>
        <h5>Heading 5</h5>
        <h6>Heading 6</h6>
        "#;
    assert_eq!(
        "# Heading 1\n\n## Heading 2\n\n### Heading 3\n\n\
             #### Heading 4\n\n##### Heading 5\n\n###### Heading 6",
        convert_faithful(html).unwrap(),
    )
}

/// Step 2 of the headings approach: only document whitespace is trimmed from
/// a heading's content, so a non-breaking space is content.
#[test]
fn heading_content_trims_only_document_whitespace() {
    assert_eq!(
        "# a",
        convert_faithful("<h1> \t\r\n a \t\r\n </h1>").unwrap()
    );
    assert_eq!("# \u{a0}", convert_faithful("<h1>&nbsp;</h1>").unwrap());
    assert_eq!(
        "\u{a0}\n=",
        convert_faithful_setext("<h1>&nbsp;</h1>").unwrap()
    );
}

/// Step 4 of the headings approach: a setext underline has one character per
/// Unicode scalar value of the content as written, escapes and tags included.
/// Levels 3-6 have no setext spelling.
#[test]
fn setext_underline() {
    assert_eq!("a\n-", convert_faithful_setext("<h2>a</h2>").unwrap());
    assert_eq!("é\n=", convert_faithful_setext("<h1>é</h1>").unwrap());
    assert_eq!(
        "a\\*b\n====",
        convert_faithful_setext("<h1>a*b</h1>").unwrap()
    );
    assert_eq!(
        "<br>*b*\n=======",
        convert_faithful_setext("<h1><br><em>b</em></h1>").unwrap()
    );
    assert_eq!(
        "*a*<br>\n=======",
        convert_faithful_setext("<h1><em>a</em><br></h1>").unwrap()
    );
    assert_eq!("### a", convert_faithful_setext("<h3>a</h3>").unwrap());
}

#[test]
fn paragraphs() {
    let html = r#"
        <p>The first.</p>
        <p>The <span>second.</span></p>
        "#;
    assert_eq!(
        "The first.\n\nThe <span>second.</span>",
        convert_faithful(html).unwrap()
    );
}

#[test]
fn quotes() {
    let html = r#"
        <blockquote>Once upon a time</blockquote>
        "#;
    assert_eq!("> Once upon a time", convert_faithful(html).unwrap());
}

#[test]
fn br() {
    let html = r#"
        <p>Hi<br>there<br><br>!</p>"#;
    assert_eq!("Hi<br>there<br><br>!", convert_faithful(html).unwrap());
}

#[test]
fn hr() {
    let html = r#"Hi <hr/> there"#;
    assert_eq!("Hi\n\n* * *\n\nthere", convert_faithful(html).unwrap());
}

#[test]
fn strong_italic() {
    let html = r#"<i>Italic</i><em>Also italic</em><strong>Strong</strong><b>Stronger</b>"#;
    assert_eq!(
        "<i>Italic</i><em>Also italic</em><strong>Strong</strong><b>Stronger</b>",
        convert_faithful(html).unwrap()
    );
    assert_eq!(
        "*ItalicAlso italic***StrongStronger**",
        convert_pure(html).unwrap()
    );
}

#[test]
fn italic_inside_word() {
    let html = r#"It<i>al</i>ic St<b>ro</b>ng"#;
    assert_eq!("It*al*ic St**ro**ng", convert_faithful(html).unwrap());
}

#[test]
fn italic_with_br() {
    let html = "a<i>b<br></i>c d<b><br>e</b>f";
    assert_eq!(
        "a<i>b<br></i>c d<b><br>e</b>f",
        convert_faithful(html).unwrap()
    );
}

#[test]
fn spaces_check() {
    let html = r#"<i>Italic</i> <em>Also italic</em>  <strong>Strong</strong> <b>Stronger </b>"#;
    assert_eq!(
        "*Italic* *Also italic* **Strong** **Stronger**",
        convert_faithful(html).unwrap()
    );
}

#[test]
fn consecutive_blocks() {
    let html = r#"<p>One</p><p>Two</p>"#;
    assert_eq!(
        indoc!(
            "
        One

        Two"
        ),
        convert_faithful(html).unwrap()
    );
}

#[test]
fn raw_text() {
    let html = r#"Hello world!"#;
    assert_eq!("Hello world!", convert_faithful(html).unwrap());
}

#[test]
fn unterminated_html() {
    // The `<i>` tag isn't terminated. Make sure the conversion still works.
    assert_eq!("# *A*", convert_faithful("<h1><i>A</h1>").unwrap());
}

// Document white space characters don't include non-breaking spaces; these should be preserved.
#[test]
fn document_whitespace() {
    assert_eq!(
        "bar\u{a0}\n\n*   foo\u{a0}",
        convert_faithful(indoc!(
            "
            <p>bar&nbsp;</p>
            <ul>
              <li>foo&nbsp;</li>
            </ul>
            "
        ))
        .unwrap()
    );
}
