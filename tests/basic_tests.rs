use std::{sync::Arc, thread::JoinHandle};

use indoc::indoc;
use pretty_assertions::assert_eq;

use htmd::{
    Element, HtmlToMarkdown,
    element_handler::Chain,
    options::{BrStyle, LinkStyle, Options, TranslationMode},
};
mod common;
use common::convert;

#[test]
fn links_with_spaces() {
    let html = r#"
        <a href="https://example.com/Some Page.html">Example</a>
        "#;
    assert_eq!(
        "[Example](<https://example.com/Some Page.html>)",
        convert(html).unwrap(),
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
        <img src="https://example.com" />
        <img src="https://example.com" alt="Image 1" />
        <img src="https://example.com" alt="Image 2" title="Hello" />
        "#;
    assert_eq!(
        "![](https://example.com) ![Image 1](https://example.com) \
            ![Image 2](https://example.com \"Hello\")",
        convert(html).unwrap(),
    )
}

#[test]
fn images_with_spaces_in_url() {
    let html = r#"
        <img src="https://example.com/Some Image.jpg" />
        "#;
    assert_eq!(
        "![](<https://example.com/Some Image.jpg>)",
        convert(html).unwrap(),
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
        convert(html).unwrap(),
    )
}

#[test]
fn code_blocks() {
    let html = r#"
        <pre><code>println!("Hello");</code></pre>
        "#;
    assert_eq!("```\nprintln!(\"Hello\");\n```", convert(html).unwrap());
}

#[test]
fn code_blocks_with_lang_class() {
    let html = r#"
        <pre><code class="language-rust">println!("Hello");</code></pre>
        "#;
    assert_eq!("```rust\nprintln!(\"Hello\");\n```", convert(html).unwrap());
}

// See https://github.com/letmutex/htmd/issues/14 for background on this test --
// the `class` attribute is deliberately misplaced to support Markdown renderers
// which don't follow the CommonMark spec.
#[test]
fn code_blocks_with_lang_class_on_pre_tag() {
    let html = r#"
        <pre class="language-rust"><code>println!("Hello");</code></pre>
        "#;
    assert_eq!(
        "```rust\nprintln!(\"Hello\");\n```",
        htmd::convert(html).unwrap()
    );
}

#[test]
fn paragraphs() {
    let html = r#"
        <p>The first.</p>
        <p>The <span>second.</span></p>
        "#;
    assert_eq!(
        "The first.\n\nThe <span>second.</span>",
        convert(html).unwrap()
    );
}

#[test]
fn quotes() {
    let html = r#"
        <blockquote>Once upon a time</blockquote>
        "#;
    assert_eq!("> Once upon a time", convert(html).unwrap());
}

#[test]
fn br() {
    let html = r#"
        Hi<br>there<br><br>!"#;
    assert_eq!("Hi  \nthere  \n  \n!", convert(html).unwrap());

    let md = HtmlToMarkdown::builder()
        .options(Options {
            br_style: BrStyle::Backslash,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build()
        .convert(html)
        .unwrap();
    assert_eq!("Hi\\\nthere\\\n\\\n!", &md);
}

#[test]
fn hr() {
    let html = r#"Hi <hr/> there"#;
    assert_eq!("Hi\n\n* * *\n\nthere", convert(html).unwrap());
}

#[test]
fn strong_italic() {
    let html = r#"<i>Italic</i><em>Also italic</em><strong>Strong</strong><b>Stronger</b>"#;
    assert_eq!(
        "*ItalicAlso italic***StrongStronger**",
        convert(html).unwrap()
    );
}

#[test]
fn italic_inside_word() {
    let html = r#"It<i>al</i>ic St<b>ro</b>ng"#;
    assert_eq!("It*al*ic St**ro**ng", convert(html).unwrap());
}

#[test]
fn inline_raw_html_escaping() {
    let html = r#"Test &lt;code&gt;tags&lt;/code&gt;, &lt;!-- comments --&gt;, &lt;?processing instructions?&gt;, &lt;!A declaration&gt;, and &lt;![CDATA[character data]]&gt;."#;
    assert_eq!(
        r#"Test \<code>tags\</code>, \<!-- comments -->, \<?processing instructions?>, \<!A declaration>, and <!\[CDATA\[character data\]\]>."#,
        convert(html).unwrap()
    );
}

#[test]
fn multiline_raw_html_escaping() {
    let html = indoc!(
        r#"
    Test &lt;code&gt;multi-line
    tags&lt;/code&gt;, &lt;!-- multi-line
    comments --&gt;, &lt;?multi-line
    processing instructions?&gt;, &lt;!A multi-line
    declaration&gt;, and &lt;![CDATA[multi-line
    character data]]&gt;.
    "#
    );
    assert_eq!(
        indoc!(
            r#"Test \<code>multi-line tags\</code>, \<!-- multi-line comments -->, \<?multi-line processing instructions?>, \<!A multi-line declaration>, and <!\[CDATA\[multi-line character data\]\]>."#
        ),
        convert(html).unwrap()
    );
}

#[test]
fn html_escaping() {
    let html = indoc!(
        r#"
        <p>&lt;pre</p>
        <p>&lt;script</p>
        <p>&lt;style</p>
        <p>&lt;textarea</p>
        <p>&lt;address</p>
        <p>&lt;ul</p>
        "#
    );
    assert_eq!(
        indoc!(
            r#"\<pre

            \<script

            \<style

            \<textarea

            \<address

            \<ul"#
        ),
        convert(html).unwrap()
    );
}

#[test]
fn faithful_mode_inline() {
    assert_eq!(
        convert(indoc!(
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
        convert(indoc!(r#"<hr bar>"#)).unwrap(),
        indoc!(r#"<hr bar="">"#)
    );
}

#[test]
fn faithful_mode_blockquote() {
    assert_eq!(
        convert(indoc!(
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
        convert(indoc!(r#"<h1 class="foo">Heading</h1>"#)).unwrap(),
        indoc!(r#"<h1 class="foo">Heading</h1>"#)
    );
}

#[test]
fn faithful_mode_p() {
    assert_eq!(
        convert(indoc!(r#"<p dir="ltr">Test 1</p>"#)).unwrap(),
        indoc!(r#"<p dir="ltr">Test 1</p>"#)
    );
}

#[test]
fn faithful_mode_pre() {
    assert_eq!(
        convert(indoc!(r#"<pre>Test</pre>"#)).unwrap(),
        indoc!(r#"<pre>Test</pre>"#)
    );
}

#[test]
fn faithful_mode_code_block1() {
    assert_eq!(
        convert(indoc!(r#"<pre><code accesskey="f">Test</code></pre>"#)).unwrap(),
        indoc!(r#"<pre><code accesskey="f">Test</code></pre>"#)
    );
}

#[test]
fn faithful_mode_code_block2() {
    assert_eq!(
        convert(indoc!(
            r#"<pre><code class="language-ruby"><i>Test</i></code></pre>"#
        ))
        .unwrap(),
        indoc!(r#"<pre><code class="language-ruby"><i>Test</i></code></pre>"#)
    );
}

#[test]
fn faithful_mode_ol1() {
    assert_eq!(
        convert(indoc!(
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
        convert(indoc!(
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
        convert(indoc!(r#"<!-- Test -->"#)).unwrap(),
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
    let md = convert(html).unwrap();
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
        convert(indoc!(
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

#[test]
fn faithful_mode_nested_inline_html() {
    assert_eq!(
        convert("<p>Nested <foo><bar><em>content</em></bar></foo></p>").unwrap(),
        "Nested <foo><bar>*content*</bar></foo>"
    );
}

#[test]
fn spaces_check() {
    let html = r#"<i>Italic</i> <em>Also italic</em>  <strong>Strong</strong> <b>Stronger </b>"#;
    assert_eq!(
        "*Italic* *Also italic* **Strong** **Stronger**",
        convert(html).unwrap()
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
        convert(html).unwrap()
    );
}

#[test]
fn raw_text() {
    let html = r#"Hello world!"#;
    assert_eq!("Hello world!", convert(html).unwrap());
}

#[test]
fn nested_divs() {
    let html = r#"
    <div>
        <div>
            <div>Hi</div>
        </div>
        <div></div>
        <div>there</div>
    </div>
    "#;
    assert_eq!("Hi\n\nthere", htmd::convert(html).unwrap());
}

#[test]
fn with_head() {
    let html = r#"
    <html>
        <head>
            <title>Demo</title>
            <script>console.log('Hello');</script>
            <style>body {}</style>
        </head>
        <body>
            Content
        </body>
    </html>
    "#;
    assert_eq!(
        "Demo\n\nconsole.log('Hello');\n\nbody {}\n\nContent",
        htmd::convert(html).unwrap()
    );
}

#[test]
fn with_custom_rules() {
    // Remove element
    let html = r#"<img src="https://example.com"/>"#;
    let md = HtmlToMarkdown::builder()
        .add_handler(vec!["img"], |_: &dyn Chain, _element: Element| (None, true))
        .build()
        .convert(html)
        .unwrap();
    assert_eq!("", &md);
}

#[test]
fn with_custom_rules_and_fallback() {
    let html = r#"<img src="https://example.com"/>"#;
    let converter = HtmlToMarkdown::builder()
        .add_handler(vec!["img"], |chain: &dyn Chain, element: Element| {
            if element
                .attrs
                .iter()
                .find(|attr| &attr.name.local == "id" && attr.value.as_ref() == "do_not_skip_me")
                .is_some()
            {
                chain.proceed(element)
            } else {
                (None, true)
            }
        })
        .options(Options {
            ..Default::default()
        })
        .build();
    assert_eq!("", &converter.convert(html).unwrap());

    let html = r#"<img src="https://example.com" id="do_not_skip_me"/>"#;
    assert_eq!(
        "![](https://example.com)",
        &converter.convert(html).unwrap()
    );
}

#[test]
fn upper_case_tags() {
    let html = r#"<H1>Hello</H1> <P>World</P>"#;
    assert_eq!("# Hello\n\nWorld", convert(html).unwrap());
}

#[test]
fn html_entities() {
    let html = r#"<p><a href="/my%20&amp;uri" title="my%20&amp;title">my%20&amp;link</a></p>"#;
    assert_eq!(
        r#"[my%20&link](/my%20&uri "my%20&title")"#,
        convert(html).unwrap()
    );

    let html_plain = r#"<p>This &amp; that, then &lt; &gt; now.</p>"#;
    assert_eq!(
        r#"This & that, then < > now."#,
        convert(html_plain).unwrap()
    );

    let html_pre = r#"<pre><code>let x = 5 &amp;&amp; y &lt; 10;</code></pre>"#;
    assert_eq!("```\nlet x = 5 && y < 10;\n```", convert(html_pre).unwrap());
}

#[test]
fn scripting_option() {
    let html = r#"<noscript><p>Hello</p></noscript>"#;
    let md = HtmlToMarkdown::builder()
        .scripting_enabled(true)
        .build()
        .convert(html)
        .unwrap();
    assert_eq!(r#"\<p>Hello\</p>"#, md);

    let md = HtmlToMarkdown::builder()
        .scripting_enabled(false)
        .build()
        .convert(html)
        .unwrap();
    assert_eq!("Hello", md);
}

#[test]
fn multithreading() {
    let html = r#"<a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    <a href="https://example.com">Example</a>
    "#;
    let expected = "[Example][1] [Example][2] [Example][3] [Example][4] [Example][5]\n\n\
    [1]: https://example.com\n[2]: https://example.com\n[3]: https://example.com\n\
    [4]: https://example.com\n[5]: https://example.com";
    let converter = HtmlToMarkdown::builder()
        .options(Options {
            // We use a global vec to store all referenced links of the doc in
            // the anchor element handler, this is unsafe for multithreading
            // usage if we do nothing
            link_style: LinkStyle::Referenced,
            translation_mode: TranslationMode::Faithful,
            ..Default::default()
        })
        .build();
    let converter = Arc::new(converter);
    let mut handlers: Vec<JoinHandle<()>> = vec![];
    for _ in 0..20 {
        let converter_clone = converter.clone();
        let handle = std::thread::spawn(move || {
            let md = converter_clone.convert(html).unwrap();
            assert_eq!(expected, md);
        });
        handlers.push(handle);
    }
    for handle in handlers {
        handle.join().unwrap();
    }
}

#[test]
fn unterminated_html() {
    // The `<i>` tag isn't terminated. Make sure the conversion still works.
    assert_eq!("# *A*", convert("<h1><i>A</h1>").unwrap());
}
