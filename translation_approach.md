Translation approach
====================

This document is organized to mirror the
[CommonMark spec](https://spec.commonmark.org/0.31.2/). CommonMark extensions
are governed by the
[pulldown-cmark spec](https://pulldown-cmark.github.io/pulldown-cmark/specs.html),
which also references the
[GitHub Flavored Markdown Spec](https://github.github.com/gfm/). However, this
document begins with a discussion of the translation of HTML blocks and raw HTML
inlines, since these impact the entire translation process.

The Faithful expected column is what a correct design should produce. Notation
inside a cell: `⏎` is a newline; `␣` is a space; `<br><br>...<br>` stands for
one or more `<br>` elements. Where one `<br>` and a run of them need different
Markdown or produce different translations, they get a row each.

CommonMark translation of HTML blocks and raw HTML inlines
----------------------------------------------------------

The majority of the document uses `TranslationMode::Faithful`, which outputs all
`<br>` elements not as a CommonMark break, but as the HTML `<br>`. In contrast,
`TranslationMode::Pure` outputs traditional CommonMark hard line breaks,
sacrificing correctness for simpler output. This tradeoff is driven by
[CommonMark](https://spec.commonmark.org/0.31.2/) rules:

1. A [hard line break](https://spec.commonmark.org/0.31.2/#hard-line-breaks)
   must be followed by content **in the same block**. Neither `␣␣⏎` nor `\⏎` can
   encode a `<br>` that ends a block: a line holding only two spaces is
   [blank](https://spec.commonmark.org/0.31.2/#blank-lines), and a trailing `\`
   is a literal backslash. This is what `TranslationMode::Faithful` sidesteps by
   writing the tag where the break is rather than at a line end.

2. Some HTML sequences cannot be represented in CommonMark; only
   `TranslationMode::Faithful` produces the
   [HTML blocks](https://spec.commonmark.org/0.31.2/#html-blocks) and
   [raw HTML inlines](https://spec.commonmark.org/0.31.2/#raw-html) necessary in
   these cases.

### Analysis

The CommonMark rules for HTML blocks and raw HTML inlines drive nearly every
table row below. While this section focuses on `<br>` elements, the same
principles apply to any HTML block (in some cases, over-conservatively) or raw
HTML inline. An implementation ensuring correct `<br>` operation should not
employ special cases for a `<br>` element, but rather generalize to any HTML
block or raw HTML inline.

`br` is not a block-level tag name, so the only HTML block a raw `<br>` can open
is [type 7](https://spec.commonmark.org/0.31.2/#html-blocks): a line that begins
with a complete open or closing tag and holds nothing but whitespace after it
(see the spec for additional conditions). Therefore, `<br><br>` is an ordinary
paragraph where `<br>` alone is not.

Type 7 is also the one HTML block that cannot
[interrupt a paragraph](https://spec.commonmark.org/0.31.2/#paragraphs), and the
block it does open runs to the next blank line, swallowing every following line
in the same container. A lone `<br>` therefore needs a blank line on *both*
sides: without the one before, it joins the paragraph above as an ordinary
inline tag; without the one after, it eats the block below.

For simplicity, a serialized HTML block is given a blank line before and after,
though this is only required for some HTML block types. The container supplies
two exceptions: the marker line of a list item stands in for the blank line
before, and a following sibling item closes the block without one. Inside a
[blockquote](https://spec.commonmark.org/0.31.2/#block-quotes) the separator is
a blank `>` line, since a truly blank line would end the blockquote instead.

### Translating HTML nodes

CommonMark blocks may only process their content into CommonMark in a block
context; in an inline context, they should emit raw HTML inlines. For example:

| Description                     | HTML in             | Faithful expected |
| ------------------------------- | ------------------- | ----------------- |
| Heading with embedded paragraph | `<h1><p>a</p></h1>` | `#␣<p>a</p>`      |

The following rules determine the context of an HTML node's contents based on
the current context. This context passed down by the DOM walker then enables an
HTML node to examine the current context to be correctly translated.

* The initial (root) context is a block context.
* Container blocks, which are a blockquote (`<blockquote>`) and a list/list item
  (`<ol>`/`<ul>` then `<li>`), can only be translated as CommonMark in a block
  context; they begin a (new) block context.
* Thematic breaks (`<hr>`) must occur in a block context to be translated to
  CommonMark. They have no content.
* Leaf blocks, which are headings (`<h1>`-`<h6>`), paragraphs (`<p>`), and
  tables (`<table>`, `<thead>`, `<tbody>`, `<tr>`), can only be translated as
  CommonMark/GFM when they appear in a block context. Headings, paragraphs, and
  table cells (`<td>`/`<th>`) begin an inline context.
* Inlines, consisting of emphasis (`<em>`) and strong emphasis (`<strong>`),
  links (`<a>`), autolinks (`<a>`), and raw HTML, begin an inline context.
* Inlines with no content (`<img>`, `<br>`) aren't affected by context.
* Both code blocks and code spans (`<pre>`, `<code>`) create a literal context.
  Code blocks must occur in a block context to be translated to CommonMark. Any
  HTML element discovered inside a literal context forces serialization of the
  `<pre>`/`<code>` elements.
* Math (a `<span>` of the form the math section describes) creates a literal
  context when translated as `$...$` or `$$...$$`. A span holding an element or
  a comment is not of that form, so it is translated like any other `<span>`.
* Given an HTML node which cannot be translated to CommonMark:
  * HTML type 1-6 in a block context is emitted as an HTML block; the
    inline/block classification does not apply to these contents. In contrast,
    HTML type 1-6 in an inline context is emitted as raw HTML inlines; the
    resulting context is inline.
  * HTML type 7 is always emitted as raw HTML inlines; the resulting context is
    inline. This choice is a tradeoff: it provides more natural behavior inside
    tight lists and when encountering text that's a direct child of `<body>`;
    for example, it translates `<body>This is <em>really</em> important.</body>`
    as `This is *really* important.` instead of (as HTML blocks) `This
    is⏎⏎<em>⏎⏎really⏎⏎</em>⏎⏎important.` This approach, however, fails to
    preserve a possible faithful encoding of lone inline tags: `<br><br>` stays
    `<br><br>` in CommonMark, which re-translates to `<p><br><br></p>`.
    Translating to `<br>⏎⏎<br>` would preserve the original HTML.

Newline encoding works exactly when the parser reading that position decodes
character references there: the CommonMark parser in the text between raw HTML
inlines, the HTML parser in the data and attribute values of an HTML block or
raw HTML inlines. It works nowhere else — inside a comment, a CDATA section, a
declaration, a processing instruction (although the html5ever crate turns these
into comments), or a raw text element the reference stays as the literal
characters composing it. The rest of this section follows from this.

Type 6 HTML blocks cannot contain blank lines; these must be encoded by
replacing CR/LF characters in blank lines with `&#13;`/`&#10;`, where blank
lines are defined by
[section 2.1 of the CommonMark spec](https://spec.commonmark.org/0.31.2/#characters-and-lines).
All raw HTML inlines will have whitespace collapsed in their contents (including
newlines, which would otherwise be problematic), a standard part of current htmd
processing.

Type 1 content placed inline is interpreted correctly for `<pre>` and
`<textarea>`: always-encoded characters `"`, `&`, `<`, and `>` are interpreted
by the HTML parser; however, newlines are treated as whitespace and emitted as
spaces. For `<script>` and `<style>`, the always-encoded characters are not
interpreted by the HTML parser inside these tags. This requires a tradeoff: this
type 1 content is walked like any other raw HTML inline, so the always-encoded
characters in `<script>`/`<style>` tags are escaped and come back as character
references, causing them to be mis-translated and newlines in
`<pre>`/`<textarea>` tags are lost. The alternative of emitting this content as
an HTML block would mis-translate this content placed in CommonMark blocks that
cannot contain a newline (headings and tables). Walking ensures that CommonMark
in the content is escaped, so `<h1>a<script>b*c*d</script>e</h1>` keeps its
literal `*c*`.

Type 2-5 content placed inline faces other difficulties. Newlines cannot be
encoded, since the CommonMark parser doesn't interpret character references
inside this type of raw HTML inline. For simplicity, whitespace including
newlines is collapsed, a lossy translation.

A more complex, higher-fidelity result for type 1-5 content placed inline comes
from translating the containing CommonMark block to HTML while also translating
the type 1-5 content as an HTML block, then appending the remaining HTML
contents to it without an intervening newline. The result is slightly lossy:
`<h1>a<script>"&<⏎⏎>"</script></h1>` becomes
`<h1>a⏎⏎<script>"&<⏎⏎>"</script></h1>`, and `<h1>a<!--b⏎⏎c-->d</h1>` becomes
`<h1>a⏎⏎<!--b⏎⏎c-->d</h1>`. For simplicity, this is not implemented.

**Special case**: If a type 1 `<script>`, `<style>` or a type 2-5 block
containing blank lines is nested inside a type 6 block, the naive translated
result is incorrect. The correct result would be placing a blank line before the
next nested type 1-5 block to start another block. Likewise, nesting a type 1-5
block inside another type 1 block fails when the inner block contains any type 1
termination condition followed by newlines: `<pre><style></style>⏎a</pre>`. Due
to this complexity, the current translation is unspecified.

**Special case**: Content in `<iframe>`, `<xmp>`, `<noscript>` (scripting
enabled), `<noembed>`, `<noframes>`, and `<plaintext>` tags comes back from the
html5ever tokenizer as literal characters, meaning encoding cannot be used to
handle newlines. The translation for these unusual cases is unspecified for
simplicity.

**Special case**: A `<!DOCTYPE html>` declaration (which html5ever attaches to
the document node) is dropped when translating.

These rules bind every element handler, the custom handlers registered through
`add_handler` included. In particular, no handler may write a bare newline in an
inline context: every one of them is encoded, replaced or removed by the rules
above, so an inline context holds a single line. A handler which writes one
anyway produces a translation this design does not describe, and the result is
unspecified.

### At the document root

| Description           | HTML in                      | Faithful expected            |
| --------------------- | ---------------------------- | ---------------------------- |
| Lone `<br>`s          | `<br><br>...<br>`            | `<br><br>...<br>`            |
| `<br>` before a block | `<br><p>a</p>`               | `<br>⏎⏎a`                    |
| `<br>` after a block  | `<p>a</p><br>`               | `a⏎⏎<br>`                    |
| `<br>`s-only `<div>`  | `<div><br><br>...<br></div>` | `<div><br><br>...<br></div>` |

Note that the row 1 translation is lossy: a `<p>` will be inserted when the
CommonMark is translated back to HTML when there are two or more `<br>`s. The
`<div>` row needs no special case at all: `div` *is* a block-level tag name, so
the whole element is an HTML block of
[type 6](https://spec.commonmark.org/0.31.2/#html-blocks) and round-trips
verbatim, however many `<br>`s it holds.

Leaf blocks
-----------

### Headings

The rows use `<h1>`. In the setext rows, an `<h2>` underlines with `---` instead
of `===`, or writes `##` in a row which falls back to ATX; levels 3-6 have no
setext form. In ATX rows, each level writes one more `#`.

The criteria of the paragraphs section below also apply to
[Setext headings](https://spec.commonmark.org/0.31.2/#setext-headings); if a
level 1 or level 2 heading (the only headings expressible using setext) meets
these criteria, it must instead be encoded as an ATX heading, as the `<br>`-only
heading row below shows. A heading holding nothing falls back to ATX for a
different reason: its underline is empty as well, so the setext spelling is two
blank lines and the heading is lost.

| Description                        | HTML in                    | Faithful expected                 |
| ---------------------------------- | -------------------------- | --------------------------------- |
| Empty heading                      | `<h1></h1>`                | `#`                               |
| `<br>`-only heading (one `<br>`)   | `<h1><br></h1>`            | `#␣<br>`                          |
| `<br>`s-only heading (two or more) | `<h1><br><br>...<br></h1>` | `<br><br>...<br>⏎===============` |
| `<br>` starting a heading          | `<h1><br><em>b</em></h1>`  | `<br>*b*⏎=======`                 |
| `<br>` ending a heading            | `<h1><em>a</em><br></h1>`  | `*a*<br>⏎=======`                 |

An [ATX heading](https://spec.commonmark.org/0.31.2/#atx-headings) is a single
line. The `#` has already opened the line, so a raw `<br>` is safe anywhere in
the heading (see the analysis section). A heading with no content is the `#`s
alone: a space with nothing after it would leave trailing whitespace on the
line.

| Description               | HTML in                    | Faithful expected   |
| ------------------------- | -------------------------- | ------------------- |
| Empty heading             | `<h1></h1>`                | `#`                 |
| `<br>`s-only heading      | `<h1><br><br>...<br></h1>` | `#␣<br><br>...<br>` |
| `<br>` starting a heading | `<h1><br><em>b</em></h1>`  | `#␣<br>*b*`         |
| `<br>` ending a heading   | `<h1><em>a</em><br></h1>`  | `#␣*a*<br>`         |

### Code blocks and code spans

The content of a [code span](https://spec.commonmark.org/0.31.2/#code-spans) or
an
[indented](https://spec.commonmark.org/0.31.2/#indented-code-blocks)/[fenced code block](https://spec.commonmark.org/0.31.2/#fenced-code-blocks)
is literal text: any raw HTML, such as a `<br>` written inside one, is four
characters, not a break. No encoding of raw HTML survives there, which is why
all rows of the table use HTML blocks or raw HTML inlines instead of inline code
spans or code blocks. Adjacent code spans cannot be encoded; for example,
``` `a``b` ``` produces a single code span holding `a``b`. The last row shows
the required serialization in this case.

| Description                  | HTML in                                     | Faithful expected                           |
| ---------------------------- | ------------------------------------------- | ------------------------------------------- |
| `<br>`s in a code span       | `a<code>x<br><br>...<br>y</code>b`          | `a<code>x<br><br>...<br>y</code>b`          |
| `<br>`s alone in a code span | `a<code><br><br>...<br></code>b`            | `a<code><br><br>...<br></code>b`            |
| `<br>`s in a code block      | `<pre><code>a<br><br>...<br>b</code></pre>` | `<pre><code>a<br><br>...<br>b</code></pre>` |
| `<br>`s-only code block      | `<pre><code><br><br>...<br></code></pre>`   | `<pre><code><br><br>...<br></code></pre>`   |
| Adjacent code spans          | `<code>a</code><code>b</code>`              | `<code>a</code><code>b</code>`              |

### Paragraphs

Classification alone is not enough: a paragraph containing only a lone type 7
raw HTML inline or beginning with a type 1-6 raw HTML inline dissolves the
paragraph containing it. Of the CommonMark leaf blocks, only a paragraph and a
setext heading are exposed. An ATX heading's `#` is leaf-block syntax the block
scan matches before any HTML block start condition; a table cell's contents are
parsed as inline content, so no block scan runs inside one at all; and a
blockquote's `>` and a list item's marker are stripped before that scan and so
protect nothing. Therefore, where a paragraph's content would be replaced by an
HTML block, serialize the entire paragraph instead of its content; a setext
heading falls back to ATX instead (see the headings section). Rows 1-2 of the
following table and row 2 of the blockquotes table show this rule.

| Description                          | HTML in                             | Faithful expected                   |
| ------------------------------------ | ----------------------------------- | ----------------------------------- |
| Paragraph starting with a `<iframe>` | `<p><iframe src="u">a</iframe></p>` | `<p><iframe src="u">a</iframe></p>` |
| `<br>`-only paragraph (one `<br>`)   | `<p><br></p>`                       | `<p><br></p>`                       |
| `<br>`s-only paragraph (two or more) | `<p><br><br>...<br></p>`            | `<br><br>...<br>`                   |
| `<br>` starting a paragraph          | `<p><br><em>b</em></p>`             | `<br>*b*`                           |
| `<br>` ending a paragraph            | `<p><em>a</em><br></p>`             | `*a*<br>`                           |
| `<br>` before an image               | `<p><br><img src="i" alt=""></p>`   | `<br>![](i)`                        |
| `<br>` after an image                | `<p><img src="i" alt=""><br></p>`   | `![](i)<br>`                        |

The second row illustrates this section's rule that a paragraph writing out as a
single complete tag must be serialized: a bare `<br>` re-opens as an HTML block,
and the `<p>` around it would be lost. Two or more `<br>`s need nothing of the
sort. The second tag keeps the line from being type 7, so the plain encoding is
already an ordinary paragraph and round-trips as it stands.

### Tables

The body-cell rows below are a one-column, one-row table —
`<table><thead><tr><th>h</th></tr></thead><tbody><tr><td>…</td></tr></tbody></table>`.
The columns show only the cell that holds the `<br>`, not the rest of the table.
Column padding is normalized here; the real output pads every cell to the column
width. Table heading behavior is identical to body-cell behavior. In the
*Faithful expected* column below, `\|` is
[GFM](https://github.github.com/gfm/#tables-extension-)'s escape for the literal
`|` which appears in the rendered output and is the faithful expected output:
the faithful expected output for row 1 is `|␣<br>*b*␣|`.

A cell's contents are parsed as inline content, so no HTML block can open inside
one; also, any instance of a `|` must be escaped as `\|`. That escape is read
only in CommonMark text, so a `|` sitting elsewhere in the cell is handled by
where it sits. In the open tag of a raw HTML inline it is encoded as `&#124;`,
which the HTML parser reading the result decodes back — the same trick the
"Translating HTML nodes" section uses for a line ending there. Inside a comment
neither the escape nor the encoding is read, so a comment holding a `|` leaves
the table no CommonMark spelling at all. The entire table must be serialized as
HTML if the table contains a child which could only be written as HTML (a cell
with an attribute, a `<caption>`, a comment between cells, a comment inside one
holding a `|`, etc.).

The table is also serialized as HTML where its shape is not one a
[GFM table](https://github.github.com/gfm/#tables-extension-) has. A GFM table
is a header row of header cells, then one body; every row holds the columns the
header declares. So the table goes out as HTML when it has no header row or more
than one, when a row of `<th>` follows a body row, when a `<thead>` follows a
`<tbody>` (one holding no row included), when its header row is made of `<td>`,
when it holds a `<tfoot>` or a second `<tbody>`, or when a row is wider or
narrower than the header.

| Description                       | HTML in                    | Faithful expected       |
| --------------------------------- | -------------------------- | ----------------------- |
| `<br>` before text in a body cell | `<td><br><em>b</em></td>`  | `\|␣<br>*b*␣\|`         |
| `<br>` after text in a body cell  | `<td><em>a</em><br></td>`  | `\|␣*a*<br>␣\|`         |
| `<br>`s-only body cell            | `<td><br><br>...<br></td>` | `\|␣<br><br>...<br>␣\|` |

Container blocks
----------------

### Blockquotes

| Description                      | HTML in                                          | Faithful expected   |
| -------------------------------- | ------------------------------------------------ | ------------------- |
| `<br>`s-only blockquote          | `<blockquote><br><br>...<br></blockquote>`       | `>␣<br><br>...<br>` |
| `<br>`-only blockquote paragraph | `<blockquote><p><br></p></blockquote>`           | `>␣<p><br></p>`     |
| `<br>` starting a blockquote     | `<blockquote><p><br><em>b</em></p></blockquote>` | `>␣<br>*b*`         |
| `<br>` ending a blockquote       | `<blockquote><p><em>a</em><br></p></blockquote>` | `>␣*a*<br>`         |

Note that the row 1 translation is lossy: a `<p>` will be inserted when the
CommonMark is translated back to HTML when there are two or more `<br>`s.

As stated earlier, inside a
[blockquote](https://spec.commonmark.org/0.31.2/#block-quotes) the separator is
a blank `>` line, since a truly blank line would end the blockquote instead.

### Lists

**Important**: This section is under development. Ignore it.

| Description                          | HTML in                                    | Faithful expected     |
| ------------------------------------ | ------------------------------------------ | --------------------- |
| Lone `<br>`s                         | `<ul><li><br><br>...<br></li></ul>`        | `*␣␣␣<br><br>...<br>` |
| Lone `<br>` before a block           | `<ul><li><br><p>a</p></li></ul>`           | `*␣␣␣<br>a`           |
| Lone `<br>` after a block            | `<ul><li><p>a</p><br></li></ul>`           | `*␣␣␣a<br>`           |
| `<br>`-only paragraph (one `<br>`)   | `<ul><li><p><br></p></li></ul>`            | `*␣␣␣<p><br></p>`     |
| `<br>`s-only paragraph (two or more) | `<ul><li><p><br><br>...<br></p></li></ul>` | `*␣␣␣<br><br>...<br>` |

Row 4 results from the rule in the paragraphs section above: like a blockquote,
a list item's marker is stripped like a `>`, so a bare tag re-opens as an HTML
block.

Note that the last row of this table produces the desired HTML only in a loose
list; in a tight list, it produces the HTML for row 1. The algorithm below uses
this row for a loose list, using an HTML block for a tight list.

TODO: the table above only tests HTML blocks and raw HTML inlines in a list. The
following algorithm needs a much larger set of tests to cover all its cases.

Lists are difficult to get right. Some lists can only be represented as HTML;
some are more compactly represented as a loose list; others are more compactly
represented as a tight list. The goal of this algorithm is to categorize this
list, then produce the resulting CommonMark. However, it requires knowledge of
the CommonMark type of each list item's direct children (meaning a likely
re-parse of the current CommonMark, or an approach that stores the CommonMark as
a CommonMark AST) and the ability to access the DOM backing each of these
CommonMark list item direct children. Therefore, the following approach is
impractical to implement.

Notes:

* A CommonMark list item that directly contains two block-level elements with a
  blank line between them forces the list to be
  [loose](https://spec.commonmark.org/0.31.2/#loose). To avoid this forcing, if
  the list item contains only a paragraph followed by a block that can
  [interrupt a paragraph](https://spec.commonmark.org/0.31.2/#paragraphs) — a
  [bulleted or ordered list](https://spec.commonmark.org/0.31.2/#list-items)
  starting at 1 with a non-empty first element, a
  [blockquote](https://spec.commonmark.org/0.31.2/#block-quotes), an
  [ATX heading](https://spec.commonmark.org/0.31.2/#atx-headings), a
  [fenced code block](https://spec.commonmark.org/0.31.2/#fenced-code-blocks), a
  [thematic break](https://spec.commonmark.org/0.31.2/#thematic-breaks), or an
  [HTML block](https://spec.commonmark.org/0.31.2/#html-blocks) of types 1-6 —
  then omit the blank line which separates them except in approach 1.2.1 below.
* A CommonMark list of one item holding one block cannot contain a blank line,
  forcing the list to be [tight](https://spec.commonmark.org/0.31.2/#tight).

Approach:

1. If the list consists of one list item:
   1. If that list item consists of one paragraph, set the list type to tight
      (per the notes). Skip to step 3.
   2. If that list item consists of a paragraph followed by another block which
      can interrupt a paragraph:
      1. Determine if the DOM for the paragraph starts with a `<p>`. If so, this
         is more easily represented by a loose list; place a blank line between
         the paragraph and the next block to make this list loose. Set the list
         type to loose, then skip to step 3.
      2. Otherwise, this is more easily represented as a tight list. Write the
         paragraph followed by the block with no intervening blank line; since
         the block interrupts the paragraph, this forms a tight list. Set the
         list type to tight then skip to step 3.
2. Walk each CommonMark list item to determine how to more compactly represent
   the list. Default to an unknown (not loose or tight) list, with 0 votes for a
   loose list.
   1. If this list item directly contains two block-level elements with a blank
      line between them, this forces the list to be loose. If the list type is
      already tight, this list cannot be represented in CommonMark. End the
      walk, serializing the entire list as an HTML block. Otherwise, set the
      list representation to a loose list.
   2. Walk the children of this list item which are paragraphs:
      1. If the DOM for that paragraph begins with a `<p>`, then this paragraph
         is more compactly represented as a loose list, but can also be
         represented as the equivalent HTML block. Add 1 to the loose list vote.
      2. If the DOM for that paragraph begins with an element whose tag name is
         in the type 1 or 6 list, this paragraph is more compactly represented
         as a tight list, but can also be represented as the equivalent HTML
         block. Subtract 1 from the loose list vote.
      3. If the DOM for that paragraph begins with a text node or an element
         whose tag name is not in the type 1 or 6 list, the paragraph can only
         be represented as a tight list; there's no way to remove the beginning
         `<p>` element from a loose list. If the list type is loose, this list
         cannot be represented in CommonMark. End the walk, serializing the
         entire list as an HTML block. Otherwise, set the list representation to
         a tight list.
3. Examine the resulting computed list representation:
   1. If it is loose or unknown with a positive vote, then re-walk the direct
      children of each list item. If the child is a paragraph whose DOM does not
      begin with `<p>`, serialize it as HTML. Otherwise, leave it as CommonMark.
      Combine each list item with an intervening blank line.
   2. If it is tight or unknown with a non-positive vote, then re-walk the
      direct children of each list item. If the child is a paragraph whose DOM
      begins with `<p>`, serialize it as HTML. Otherwise, leave it as
      CommonMark. Combine each list item with an intervening newline.

Inlines
-------

### Emphasis and strong emphasis

Notation: the characters `a` and `b` stand for characters which are neither
Unicode whitespace nor Unicode punctuation.

| Description                                     | HTML in                       | Faithful expected            |
| ----------------------------------------------- | ----------------------------- | ---------------------------- |
| `<br>`s starting emphasis                       | `<em><br><br>...<br>a</em>`   | `<em><br><br>...<br>a</em>`  |
| `<br>`s ending emphasis                         | `<em>a<br><br>...<br></em>`   | `<em>a<br><br>...<br></em>`  |
| `<br>`s inside emphasis                         | `<em>a<br><br>...<br>b</em>`  | `*a<br><br>...<br>b*`        |
| `<br>`s alone in emphasis with surrounding text | `a<em><br><br>...<br></em>b`  | `a<em><br><br>...<br></em>b` |
| Whitespace-only emphasis                        | `a<em>␣</em>b`                | `a<em>␣</em>b`               |
| Empty emphasis                                  | `a<em></em>b`                 | `a<em></em>b`                |
| Adjacent emphases                               | `<em>a</em><em>b</em>`        | `<em>a</em><em>b</em>`       |
| Emphasis inside emphasis                        | `<em><em>a</em></em>`         | `<em>*a*</em>`               |
| Emphasis inside strong emphasis                 | `<strong><em>a</em></strong>` | `<strong>*a*</strong>`       |

An emphasis [delimiter run](https://spec.commonmark.org/0.31.2/#delimiter-run)
opens emphasis only if it is
[left-flanking](https://spec.commonmark.org/0.31.2/#left-flanking-delimiter-run)
and closes emphasis only if it is
[right-flanking](https://spec.commonmark.org/0.31.2/#right-flanking-delimiter-run).
`<` and `>` are
[Unicode punctuation characters](https://spec.commonmark.org/0.31.2/#unicode-punctuation-character),
so an emphasis delimiter placed against a raw `<br>` dies whenever the character
on its far side is neither whitespace nor Unicode punctuation: `a*<br>*b` is the
literal text `a*`, a break, and `*b`. Rows 1-2 show the conservative algorithm
below at work; while these do have a CommonMark encoding, it is not used for
simplicity.

Rows 5-6 have no emphasis string at all once the whitespace is removed, and
CommonMark cannot encode an empty emphasis: `**` is a literal pair of asterisks,
not a strong emphasis holding nothing. Both are serialized.

Row 7 is a pair of elements which cannot be simply encoded as CommonMark.
`*a**b*` pairs its delimiter runs outside in, giving a single emphasis holding
`a**b`. While `*a*_b_` is valid, this requires additional complexity and special
cases; this implementation chooses the simplicity shown in the table. Pure mode
writes the one element the pair reads as, combining the text of both, which is a
lossy encoding: it loses an element, and loses the distinction between `<i>` and
`<em>` or `<b>` and `<strong>` when the pair mixes them. Faithful mode
serializes each element of such a run instead. This applies to a run of any
inline element except math (see the math section), so a pair which is already
serialized — `<sub>a</sub><sub>b</sub>` — stays the two elements it was rather
than being combined into one.

While strong emphasis nested inside strong emphasis (encoded as `****a****`) and
strong emphasis nested inside emphasis (encoded as `***a***`) are simply
translatable, emphasis nested inside strong emphasis (which could be encoded as
`**_a_**`) and emphasis nested inside emphasis (perhaps `*_a_*`) are instead
encoded using a mix of raw HTML inlines and CommonMark, as shown in the last two
rows of the table. This avoids the additional complexity and special cases these
encodings require.

Approach:

1. If the emphasis string is empty once its leading and trailing whitespace is
   removed, serialize the entire element as HTML. This comes before step 3,
   which would otherwise write a bare pair of delimiters around nothing.
2. If the first or last character of the emphasis string, after removing leading
   and trailing whitespace, is a Unicode punctuation character, serialize the
   entire element as HTML. (This is conservative; if the emphasis was preceded
   or followed by Unicode whitespace or punctuation, serialization isn't needed.
   However, the current implementation makes this difficult to determine.)
3. Move leading and trailing whitespace outside the emphasis string; surround
   the string with the appropriate delimiters.

The following is not implemented; it requires the ability to examine text
preceding and following emphasis. It is kept for possible future improvements.

> If the emphasis/strong emphasis has no content, it must be serialized.
> Otherwise, consider `1*233..34*5`, where:
>
> * `1` is the character preceding em/strong and `5` is the character following
>   it. If either character is missing due to the beginning or end of the line,
>   treat the character as whitespace.
> * `2` is the first character of the em/strong content and `4` is the last
>   character. If the content is one character, note that `2` and `4` refer to
>   the same character.
> * `3` is zero or more following characters of content.
>
> To encode:
>
> 1. If `2` is whitespace, encode it as a character reference; em/strong cannot
>    begin or end with whitespace.
> 2. If `2` is punctuation (note that the previous step changes whitespace to
>    punctuation for this step) and `1` is neither whitespace nor punctuation,
>    encode `1` as a character reference.
> 3. Apply rule 1 to `4` and rule 2 to `5`.

### Links

These are mainly implemented in [anchor.rs](src/element_handler/anchor.rs). The
following rules primarily come from the
[spec](https://spec.commonmark.org/0.31.2/#links).

Links may be translated as inline, inline but an autolink preferred, or
referenced. Referenced link labels consist of an incrementing whole number
assigned by the translator. (While the library currently supports collapsed and
shortcut links, these will be
[deprecated or removed](https://github.com/letmutex/htmd/issues/126).)

Emit an autolink when:

* `LinkStyle::InlinedPreferAutolinks` is set,
* the `<a>` element does not contain a `title` attribute,
* its content is a single text node equal to the `href`,
* the content begins with a [scheme](https://spec.commonmark.org/0.31.2/#scheme)
  followed by a colon, and
* the content does not contain
  [ASCII control characters](https://spec.commonmark.org/0.31.2/#ascii-control-character),
  [spaces](https://spec.commonmark.org/0.31.2/#space), `&`, `<`, or `>`
  characters. (The `&` character in an autolink becomes `&amp;`, hence its
  inclusion in characters to avoid.)

Escaping rules:

* If a `!` immediately precedes a link, it must be escaped, to prevent links
  being interpreted as an image.

* For link text, destinations, and titles:

  * All `\` characters must be escaped, since they would otherwise escape the
    following character.
  * Newlines must be encoded as `&#10;`/`&#13;` since links are an inline
    element; newlines in ATX headings, table cells, etc. would prematurely
    terminate these items. In addition, newlines are not allowed in link
    destinations. This is a conservative rule that simplifies the translation
    but may insert unnecessary escapes. For example, newlines (but not blank
    lines) are allowed in a link placed in a paragraph.
  * All `&` characters must be escaped; for example, the HTML fragment
    `&amp;copy;` which arrives as `&copy;` from the HTML parser becomes
    `\&copy;` not the actual `&copy;` character entity.
* Link text:

  * All `[` and `]` characters must be escaped, per the spec. (The spec allows
    matched pairs of brackets; this is omitted for simplicity.)
  * All `<` characters which begin an autolink or a raw HTML inline must be
    escaped; all `$` characters which begin a math expression must be escaped.
    These inlines bind more tightly than a link.
  * All `` ` `` must be escaped; per the spec, code spans bind more tightly than
    a link.
  * Hard line breaks (`<br>`) are translated correctly only in Faithful mode.

  Because link text may contain HTML, it must be translated from HTML to
  CommonMark.

* Link destinations:

  * All `(` and `)` characters must be escaped; these mark the beginning and end
    of an inline link. (The spec allows matched pairs of parentheses; this is
    omitted for simplicity.)
  * If the escaped destination is empty, starts with `<`, or includes
    [ASCII control characters](https://spec.commonmark.org/0.31.2/#ascii-control-character)
    or [space](https://spec.commonmark.org/0.31.2/#space) characters:
    * All `<` and `>` characters must be escaped per the spec.
    * Prepend `<` and append `>` to the destination.
* Link titles:

  * All `"` characters must be escaped, since the `"` ends a title string. This
    is a conservative selection that simplifies the translation but may insert
    unnecessary escapes. For example, the title `Say "hi"` could be encoded as
    `'Say "hi"'` instead of `"Say \"hi\""`.

Some of these escapes are already performed by generic text-node escaping,
covered in section TODO.

### Images

Per the [spec](https://spec.commonmark.org/0.31.2/#images), images follow the
same rules as links. For simplicity, images are translated like inline links,
even with `LinkStyle::Referenced`. In addition:

* The image description follows link text rules. However, while link text can
  contain HTML markup, the `<img>` tag's `alt` attribute is not interpreted as
  HTML; its value should not be translated to CommonMark, but instead escaped as
  plain text.
* The image destination and title follow the same rules as the link destination
  and title.
* In Faithful mode, a missing `alt` attribute requires serialization of the tag
  as HTML: `![](a)` translates to `<img src="a" alt="">`. Likewise, a `title=""`
  attribute also requires serialization: `![](a "")` becomes `<img src="a"
  alt="">`.

### Math

pulldown-cmark writes a
[math expression](https://pulldown-cmark.github.io/pulldown-cmark/specs/math.html)
as a `<span>` whose only attribute is `class="math math-inline"` or `class="math
math-display"`. A span of this form that is empty or whose only child is a text
node is written as inline (`$...$`) or display (`$$...$$`) math unless one of
the conditions below holds; any other span is translated like an ordinary
`<span>`. The translation assumes its reader enables math, and no option
disables it; without math, `$...$` reads back as CommonMark text.

Like an inline code span, the contents of a math expression are literal text
which CommonMark does not parse. Unlike a code span, which
[yields to](https://spec.commonmark.org/0.31.2/#code-spans) raw HTML and
autolinks, math binds more tightly than both: pulldown-cmark reads `$a<span>$
x</span>` as math holding `a<span>`, not as a raw HTML inline. Therefore,
nothing inside math is escaped, except in a table cell: there pulldown-cmark
reads `\|` as `|`, so the escaping of `|` described in the tables section
applies inside math as well.

A character is escaped when an odd number of backslashes precedes it: the `$` of
`\$` is escaped, while the `$` of `\\$` is not. This is how pulldown-cmark reads
[backslash escapes](https://spec.commonmark.org/0.31.2/#backslash-escapes).
LaTeX agrees on which characters are escaped, though not on why: a backslash and
the non-letter after it form one control symbol, so `\\` is a line break rather
than a literal backslash
([see section 1.3.3 of The Not So Short Introduction to LaTeX 2ε](https://latex.us/info/lshort/english/lshort-letter.pdf#page=20)).

The contents of a math expression are processed by:

1. Stripping comments by replacing an unescaped comment character `%`, "the rest
   of the present line, the line break, and all whitespace at the beginning of
   the next line," (see
   [section 1.3.4](https://latex.us/info/lshort/english/lshort-letter.pdf#page=21))
   with a space. In this context, whitespace is defined by LaTeX: spaces and
   tabs
   ([see section 1.3.1](https://latex.us/info/lshort/english/lshort-letter.pdf#page=19)).
   This approach correctly translates input such as `\alpha%c⏎x` to `\alpha x`,
   since LaTeX control words are terminated by a comment. However, this adds a
   space in text mode: `\text{fo%c⏎o}` becomes `\text{fo o}`, not `\text{foo}`.
2. Replacing all newlines with spaces: "Most spaces and line breaks do not have
   any significance"
   ([section 3.2.1](https://latex.us/info/lshort/english/lshort-letter.pdf#page=64));
   also, when in text mode (`\text{...}`), LaTeX treats all whitespace
   characters as a space
   ([see section 1.3.1](https://latex.us/info/lshort/english/lshort-letter.pdf#page=19)).
   A newline preceded by `\` is replaced too, since both `\⏎` and `\␣` are
   LaTeX's control space. A blank line, which math mode does not allow
   ([section 3.2.1](https://latex.us/info/lshort/english/lshort-letter.pdf#page=64)),
   becomes spaces as well, which repairs the expression. This follows CommonMark
   requirements: a newline may end the math expression and begin another block,
   as in `x $a⏎# b$`; in addition, per the section "Translating HTML nodes", "an
   inline context holds a single line."
3. For inline math, trimming leading and trailing whitespace. Per the
   [pulldown-cmark spec](https://pulldown-cmark.github.io/pulldown-cmark/specs/math.html)
   inline math must not begin with whitespace or end with whitespace. Whitespace
   here is defined by pulldown-cmark: a space, tab, line feed, vertical tab,
   form feed, or carriage return (see
   [`is_ascii_whitespace`](https://github.com/pulldown-cmark/pulldown-cmark/blob/v0.13.4/pulldown-cmark/src/scanners.rs#L425-L427),
   which, unlike Rust's
   [`char::is_ascii_whitespace`](https://doc.rust-lang.org/std/primitive.char.html#method.is_ascii_whitespace),
   includes a vertical tab).

If the resulting string meets any of these conditions, serialize the span as
HTML instead:

1. it is empty and the span is inline math (see the
   [pulldown-cmark spec](https://pulldown-cmark.github.io/pulldown-cmark/specs/math.html));
2. it contains an unescaped `$`. This is a conservative, simpler rule, since
   pulldown-cmark allows a `$` inside matching unescaped curly braces, such as
   `\text{$a$}`;
3. it ends in an unescaped `\`, which would escape the closing `$`. Trimming a
   trailing control space `\␣` produces this case;
4. its unescaped curly braces are unbalanced. pulldown-cmark pairs two
   delimiters only when the braces between them balance, so it reads both
   `$}a{$` and `$a{$` as text.

The serialized span holds the result of steps 1-2, not its original text:
written as a raw HTML inline, the span's newlines collapse to spaces, which
would run each comment on to the end of the span. Skipping step 3 keeps any
leading and trailing whitespace, so `<span class="math math-inline">%a</span>`
is serialized as `<span class="math math-inline">␣</span>`. In
`TranslationMode::Pure`, which writes no HTML, the span is dropped and the
result of steps 1-2 is written as text: `\text{$a$}` reads back as that literal
text, not as math, and an empty inline span vanishes.

Adjacent math expressions are allowed; the combining of adjacent inline elements
described in the emphasis section therefore does not apply to math.

pulldown-cmark looks for math delimiters in all text, so a `$` in a text node
must be escaped as `\$`; otherwise, `<p>$5,$6</p>` reads back as math holding
`5,`. Since `\$` is an ordinary CommonMark escape, it renders the same when math
is not enabled, at the cost of a backslash before every `$`.

This translation loses source text, since it trims whitespace and replaces
comments and newlines with spaces, but it rarely changes the rendered math.
Trimming never does: a written expression has balanced braces (condition 4), so
its ends lie in math mode, where whitespace has no significance, and condition 3
catches the one significant trailing whitespace, a control space. LaTeX reads a
single newline as a space, and math mode ignores the space which replaces a
comment, so only a comment in text mode renders differently (see step 1).
