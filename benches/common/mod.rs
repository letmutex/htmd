use std::fmt::Write as _;

const HEAD: &str =
    "<!DOCTYPE html><html><head><title>Synthetic Benchmark</title></head><body><main>";
const TAIL: &str = "</main></body></html>";

/// Builds a realistic, deterministic HTML document of at least `target_bytes`.
pub fn generate_synthetic_html(target_bytes: usize) -> String {
    let mut html = String::with_capacity(target_bytes + 1024);
    html.push_str(HEAD);

    let mut section = 0usize;
    while html.len() + TAIL.len() < target_bytes {
        let _ = write!(
            html,
            "<section id=\"sec-{section}\">\
             <h2>Section {section}: Overview &amp; Details</h2>\
             <p>This paragraph tests <strong>strong emphasis</strong>, \
             <em>adjacent</em><em>emphasis</em>, <code>inline_code_{section}()</code>, \
             <span class=\"badge\">tag-{section}</span>, and a \
             <a href=\"https://example.com/docs/{section}\" title=\"Docs {section}\">reference link</a>.</p>\
             <ul>\
             <li>First bullet with <em>italicized</em> detail #{section}</li>\
             <li>Second bullet with <a href=\"https://example.com/item/{section}\">nested link</a>\
             <ol><li>Sub-item A</li><li>Sub-item B</li></ol></li>\
             </ul>\
             <blockquote><p>Quoted note for section {section} covering &lt;escaped&gt; tokens.</p></blockquote>\
             <pre><code class=\"language-rust\">fn bench_{section}() {{ let x = {section}; }}</code></pre>\
             <table><thead><tr><th>Key</th><th>Value</th><th>Status</th></tr></thead>\
             <tbody><tr><td>k{section}</td><td>v{section}</td><td>ok</td></tr>\
             <tr><td>n{section}</td><td>m{section}</td><td>done</td></tr></tbody></table>\
             </section>",
        );
        section += 1;
    }

    html.push_str(TAIL);
    html
}

/// Builds a deterministic HTML document with `depth` nested block elements and rich content per level.
pub fn generate_deep_nested_html(depth: usize) -> String {
    const TAGS: [&str; 3] = ["div", "section", "article"];
    let mut html = String::with_capacity(HEAD.len() + TAIL.len() + depth * 1100 + 512);
    html.push_str(HEAD);

    for level in 0..depth {
        let tag = TAGS[level % TAGS.len()];
        let _ = write!(
            html,
            "<{tag} class=\"d{level}\" data-depth=\"{level}\">\
             <h3>Layer {level}</h3>\
             <p>Level {level} prose containing <strong>strong text</strong>, \
             <em>adjacent</em><em>emphasis</em>, <code>node_fn_{level}()</code>, \
             <span class=\"badge\">layer-{level}</span>, and \
             <a href=\"https://example.com/layer/{level}\" title=\"Layer {level}\">link #{level}</a> \
             with extra filler words to bloat node payload size consistently across deep trees.</p>\
             <ul>\
             <li>Item A at depth {level} with <em>italic</em> marker</li>\
             <li>Item B at depth {level} with <a href=\"https://example.com/b/{level}\">nested ref</a>\
             <ol><li>Sub-step 1</li><li>Sub-step 2</li></ol></li>\
             </ul>\
             <blockquote><p>Sibling quote at depth {level} with &lt;escaped&gt; &amp; entities.</p></blockquote>"
        );
    }

    html.push_str(
        "<p>Deepest leaf content with <strong>bold</strong>, <code>deep_leaf()</code>, \
         and <a href=\"https://example.com/leaf\">leaf link</a>.</p>\
         <pre><code class=\"language-rust\">fn deepest_leaf() -> usize { 42 }</code></pre>",
    );

    for level in (0..depth).rev() {
        let tag = TAGS[level % TAGS.len()];
        let _ = write!(html, "</{tag}>");
    }

    html.push_str(TAIL);
    html
}
