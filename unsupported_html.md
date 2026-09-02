CommonMark translation of HTML blocks and raw HTML inlines
==========================================================

This design requires `TranslationMode::Faithful`, which outputs all `<br>`
elements not as a CommonMark break, but as the HTML `<br>`. This choice is
driven by [CommonMark](https://spec.commonmark.org/0.31.2/) rules:

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

The Faithful expected column is what a correct design should produce. Notation
inside a cell: `⏎` is a newline; `␣` is a space; `<br><br>...<br>` stands for
one or more `<br>` elements. Where one `<br>` and a run of them need different
Markdown, they get a row each.

Analysis
--------

The CommonMark rules for HTML blocks and raw HTML inlines drive nearly every
table row below. While this file focuses on `<br>` elements, the same principles
apply to any HTML block (in some cases, over-conservatively) or raw HTML inline.
An implementation ensuring correct `<br>` operation should not employ special
cases for a `<br>` element, but rather generalize to any HTML block or raw HTML
inline.

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

Translating HTML nodes
----------------------

Given an HTML node that can't be encoded as CommonMark, it must be classified
either as an HTML block or a raw HTML inline. To distinguish these, the DOM
walker must record its context: the initial (root) context is a block context.
Leaf blocks, which are headings (`<h1>`-`<h6>`), paragraphs (`<p>`), and tables
(`<td>`/`<th>`/`<caption>`), begin an inline context, while container blocks,
which are a blockquote (`<blockquote>`) and a list item (`<li>`), begin a block
context. No other HTML element can affect the context. This context passed down
by the DOM walker then enables an HTML node to examine the current context to be
correctly translated. Note that the contents of code blocks are treated as
literal text, making them neither an inline nor a block context. Since these
cannot contain HTML nodes, they are exempt from the following logic.

| HTML node                     | Context | Newline encoding | CommonMark translation |
| ----------------------------- | ------- | ---------------- | ---------------------- |
| Type 1-5                      | Block   | None             | HTML block             |
| Type 1                        | Inline  | Newlines         | Raw HTML inline        |
| Type 2-5, with blank lines    | Inline  | None             | Special case           |
| Type 2-5, without blank lines | Inline  | None             | Raw HTML inline        |
| Type 6                        | Block   | Blank lines      | HTML block             |
| Type 6                        | Inline  | Newlines         | Raw HTML inline        |
| Type 7                        | Any     | Newlines         | Raw HTML inline        |

Newline encoding consists of replacing CR/LF characters with `&#13;`/`&#10;`.
Blank lines are defined by
[section 2.1 of the CommonMark spec](https://spec.commonmark.org/0.31.2/#characters-and-lines).
Encoding works exactly when the HTML parser decodes character references at that
position; the following special cases flow from this understanding.

**Special case**: the containing CommonMark block must be translated as an HTML
block while also translating the special case type 2-5 content as an HTML block
then appending the remaining HTML contents to it without an intervening newline;
`<h1>a<!--b⏎⏎c-->d</h1>` becomes `<h1>a⏎⏎<!--b⏎⏎c-->d</h1>` (which is a slightly
lossy translation). Due to this complexity, the current implementation is
unspecified for simplicity.

**Special case**: If (`<script>`, `<style>` or type 2-5 blocks) containing blank
lines are nested inside a type 6 block, the naive translated result is
incorrect. The correct result would be placing a blank line after the type 6
block to start another block, following the approach of previous special case.
Due to this complexity, the current implementation is unspecified for
simplicity.

**Special case**: Content in `<iframe>`, `<xmp>`, `<noscript>` (scripting
enabled), `<noembed>`, `<noframes>`, and `<plaintext>` tags all come back from
the HTML5ever tokenizer as literal characters, meaning encoding cannot be used
to handle newlines. The implementation for these unusual cases is unspecified
for simplicity.

The type 7 choice of always translating to a raw HTML inline is a tradeoff: it
provides more natural behavior inside tight lists and when encountering text
that's a direct child of `<body>`; for example, it translates `This is
<em>really</em> important.` as `This is *really* important.` instead of (as HTML
blocks) `This is⏎⏎<em>⏎⏎really⏎⏎</em>⏎⏎important.` This approach, however, fails
to preserve a possible faithful encoding of lone inline tags: `<br><br>` stays
`<br><br>` in CommonMark, which re-translates to `<p><br><br></p>`. Translating
to `<br>⏎⏎<br>` would preserve the original HTML.

CommonMark blocks may only process their content into CommonMark in a block
context; in an inline context, they should emit raw HTML inlines. For example:

| Description                     | HTML in             | Faithful expected |
| ------------------------------- | ------------------- | ----------------- |
| Heading with embedded paragraph | `<h1><p>a</p></h1>` | `#␣<p>a</p>`      |

Special case for paragraphs
---------------------------

Classification alone is not enough: a paragraph containing only a raw HTML
inline or only an HTML block dissolves the paragraph containing it. Of the
containers above, only a paragraph and a setext heading are exposed. An ATX
heading's `#` is leaf-block syntax the block scan matches before any HTML block
start condition; a table cell's contents are parsed as inline content, so no
block scan runs inside one at all; and a blockquote's `>` and a list item's
marker are stripped before that scan and so protect nothing. Therefore, where a
paragraph's content would be replaced by an HTML block, serialize the entire
paragraph instead of its content; a setext heading falls back to ATX instead
(see the headings section). See row 1 of the paragraphs section and row 2 of the
blockquotes section in addition to the following table.

| Description                          | HTML in                             | Faithful expected                   |
| ------------------------------------ | ----------------------------------- | ----------------------------------- |
| Paragraph starting with a `<iframe>` | `<p><iframe src="u">a</iframe></p>` | `<p><iframe src="u">a</iframe></p>` |

Code
----

| Description                  | HTML in                                     | Faithful expected                           |
| ---------------------------- | ------------------------------------------- | ------------------------------------------- |
| `<br>`s in a code span       | `<p>a<code>x<br><br>...<br>y</code>b</p>`   | `a<code>x<br><br>...<br>y</code>b`          |
| `<br>`s alone in a code span | `<p>a<code><br><br>...<br></code>b</p>`     | `a<code><br><br>...<br></code>b`            |
| `<br>`s in a code block      | `<pre><code>a<br><br>...<br>b</code></pre>` | `<pre><code>a<br><br>...<br>b</code></pre>` |
| `<br>`s-only code block      | `<pre><code><br><br>...<br></code></pre>`   | `<pre><code><br><br>...<br></code></pre>`   |

**Special case:** The content of a
[code span](https://spec.commonmark.org/0.31.2/#code-spans) or a
[fenced code block](https://spec.commonmark.org/0.31.2/#fenced-code-blocks) is
literal text: a `<br>` written inside one is four characters, not a break. No
encoding of a break survives there, which is why all rows of the table are HTML.

Inline elements
---------------

The principles derived in this section apply to all the following sections,
since they are containers for inline elements. Notation: *text*, represented
below as the characters `a`-`d`, stands for a character that is neither Unicode
whitespace nor Unicode punctuation.

| Description                                     | HTML in                                              | Faithful expected                |
| ----------------------------------------------- | ---------------------------------------------------- | -------------------------------- |
| `<br>`s alone in emphasis with surrounding text | `<p>a<em><br><br>...<br></em>b</p>`                  | `a<em><br><br>...<br></em>b`     |
| `<br>`s alone in a whole paragraph of emphasis  | `<p><em><br><br>...<br></em></p>`                    | `*<br><br>...<br>*`              |
| `<br>`s starting an emphasis                    | `<p><em><br><br>...<br>a</em></p>`                   | `*<br><br>...<br>a*`             |
| `<br>`s ending an emphasis                      | `<p><em>a<br><br>...<br></em></p>`                   | `*a<br><br>...<br>*`             |
| `<br>`s starting an emphasis, text before       | `<p>a<em><br><br>...<br>b</em>c</p>`                 | `a<em><br><br>...<br>b</em>c`    |
| `<br>`s ending an emphasis, text after          | `<p>a<em>b<br><br>...<br></em>c</p>`                 | `a<em>b<br><br>...<br></em>c`    |
| `<br>`s inside an emphasis                      | `<p>a<em>b<br><br>...<br>c</em>d</p>`                | `a*b<br><br>...<br>c*d`          |
| `<br>`s alone in nested emphasis                | `<p>a<em><strong><br><br>...<br></strong></em>b</p>` | `a<em>**<br><br>...<br>**</em>b` |
| `<br>`s alone in an untranslated element        | `<p>a<del><br><br>...<br></del>b</p>`                | `a<del><br><br>...<br></del>b`   |
| `<br>`s alone in a `<span>`                     | `<p>a<span><br><br>...<br></span>b</p>`              | `a<span><br><br>...<br></span>b` |

An emphasis [delimiter run](https://spec.commonmark.org/0.31.2/#delimiter-run)
opens emphasis only if it is
[left-flanking](https://spec.commonmark.org/0.31.2/#left-flanking-delimiter-run)
and closes emphasis only if it is
[right-flanking](https://spec.commonmark.org/0.31.2/#right-flanking-delimiter-run).
`<` and `>` are Unicode punctuation characters, so an emphasis delimiter placed
against a raw `<br>` dies whenever the character on its far side is neither
whitespace nor Unicode punctuation: `a*<br>*b` is the literal text `a*`, a
break, and `*b`. Because there is no Markdown encoding for that shape, the
faithful expected cell is HTML.

Approach:

* If the first character of the emphasis string is Unicode whitespace, serialize
  it.
* If the first character of the emphasis string is a Unicode punctuation
  character and the emphasis string is preceded by content and the last
  character of this content before the emphasis string is neither Unicode
  whitespace nor Unicode punctuation, serialize it.
* If the last character of the emphasis string is Unicode whitespace, serialize
  it.
* If the last character of the emphasis string is a Unicode punctuation
  character, serialize it. This is overly conservative; the implementation has
  no easy way to determine the character following the emphasis string.

Links
-----

| Description                   | HTML in                                     | Faithful expected         |
| ----------------------------- | ------------------------------------------- | ------------------------- |
| `<br>`s alone in a link       | `<p>a<a href="u"><br><br>...<br></a>b</p>`  | `a[<br><br>...<br>](u)b`  |
| `<br>`s starting a link label | `<p>a<a href="u"><br><br>...<br>c</a>b</p>` | `a[<br><br>...<br>c](u)b` |
| `<br>`s ending a link label   | `<p>a<a href="u">c<br><br>...<br></a>b</p>` | `a[c<br><br>...<br>](u)b` |

Unlike emphasis (see the inline elements section), a link label has no flanking
rule: `[` and `]` delimit the label whatever characters sit next to them, which
makes this encoding straightforward.

At the document root
--------------------

| Description                | HTML in                      | Faithful expected            |
| -------------------------- | ---------------------------- | ---------------------------- |
| Lone `<br>`                | `<br>`                       | `<br>`                       |
| Lone `<br>`s (two or more) | `<br><br>...<br>`            | `<br><br>...<br>`            |
| `<br>` before a block      | `<br><p>a</p>`               | `<br>⏎⏎a`                    |
| `<br>` after a block       | `<p>a</p><br>`               | `a⏎⏎<br>`                    |
| `<br>`s-only `<div>`       | `<div><br><br>...<br></div>` | `<div><br><br>...<br></div>` |

Note that rows 1 and 5 round-trip exactly, while the remaining rows wrap the
result in a paragraph per the discussion in the translating HTML nodes section.
The `<div>` row needs no special case at all: `div` *is* a block-level tag name,
so the whole element is an HTML block of
[type 6](https://spec.commonmark.org/0.31.2/#html-blocks) and round-trips
verbatim, however many `<br>`s it holds.

Headings
--------

The rows use `<h1>`. In the setext table, an `<h2>` underlines with `---`
instead of `===`; levels 3-6 have no setext form. In the ATX table, each level
writes one more `#`.

[Setext headings](https://spec.commonmark.org/0.31.2/#setext-headings) cannot
contain blank lines. The special case for paragraphs section's criteria for
paragraphs apply here as well; if a level 1 or level 2 heading (the only
headings expressible using setext) meets these criteria, it must instead be
encoded as an ATX heading as shown in the first row below.

| Description                        | HTML in                    | Faithful expected         |
| ---------------------------------- | -------------------------- | ------------------------- |
| `<br>`-only heading (one `<br>`)   | `<h1><br></h1>`            | `#␣<br>`                  |
| `<br>`s-only heading (two or more) | `<h1><br><br>...<br></h1>` | `<br><br>...<br>⏎=======` |
| `<br>` starting a heading          | `<h1><br><em>b</em></h1>`  | `<br>*b*⏎=======`         |
| `<br>` ending a heading            | `<h1><em>a</em><br></h1>`  | `*a*<br>⏎=======`         |

An [ATX heading](https://spec.commonmark.org/0.31.2/#atx-headings) is a single
line. The `#` has already opened the line, so a raw `<br>` is safe anywhere in
the heading (see the analysis section).

| Description               | HTML in                    | Faithful expected   |
| ------------------------- | -------------------------- | ------------------- |
| `<br>`s-only heading      | `<h1><br><br>...<br></h1>` | `#␣<br><br>...<br>` |
| `<br>` starting a heading | `<h1><br><em>b</em></h1>`  | `#␣<br>*b*`         |
| `<br>` ending a heading   | `<h1><em>a</em><br></h1>`  | `#␣*a*<br>`         |

Paragraphs
----------

| Description                          | HTML in                    | Faithful expected |
| ------------------------------------ | -------------------------- | ----------------- |
| `<br>`-only paragraph (one `<br>`)   | `<p><br></p>`              | `<p><br></p>`     |
| `<br>`s-only paragraph (two or more) | `<p><br><br>...<br></p>`   | `<br><br>...<br>` |
| `<br>` starting a paragraph          | `<p><br><em>b</em></p>`    | `<br>*b*`         |
| `<br>` ending a paragraph            | `<p><em>a</em><br></p>`    | `*a*<br>`         |
| `<br>` before an image               | `<p><br><img src="i"></p>` | `<br>![](i)`      |
| `<br>` after an image                | `<p><img src="i"><br></p>` | `![](i)<br>`      |

The first row is the analysis section's rule that a paragraph writing out as a
single complete tag must be serialized: a bare `<br>` re-opens as an HTML block,
and the `<p>` around it would be lost.

Two or more `<br>`s need nothing of the sort. The second tag keeps the line from
being type 7, so the plain encoding is already an ordinary paragraph and
round-trips as it stands.

Blockquotes
-----------

| Description                      | HTML in                                          | Faithful expected   |
| -------------------------------- | ------------------------------------------------ | ------------------- |
| `<br>`s-only blockquote          | `<blockquote><br><br>...<br></blockquote>`       | `>␣<br><br>...<br>` |
| `<br>`-only blockquote paragraph | `<blockquote><p><br></p></blockquote>`           | `>␣<p><br></p>`     |
| `<br>` starting a blockquote     | `<blockquote><p><br><em>b</em></p></blockquote>` | `>␣<br>*b*`         |
| `<br>` ending a blockquote       | `<blockquote><p><em>a</em><br></p></blockquote>` | `>␣*a*<br>`         |

**Special case**: the blank line rule from the analysis section that puts a
blank line on either side of an HTML block, such as a lone `<br>`, is written as
a blank `>` line here. The first row is the root-level row one container down,
and holds for the same reason. The second row is the paragraph row one container
down: the `>` is stripped before the line is scanned, so the bare tag would
re-open as an HTML block exactly as it does at the root.

Table cells
-----------

The body-cell rows below are a one-column, one-row table —
`<table><thead><tr><th>h</th></tr></thead><tbody><tr><td>…</td></tr></tbody></table>`.
The columns show only the cell that holds the `<br>`, not the rest of the table.
Column padding is normalized here; the real output pads every cell to the column
width. Table heading behavior is identical to body-cell behavior. In the
*Faithful expected* column below, `\|` is
[GFM](https://github.github.com/gfm/#tables-extension-)'s escape for the literal
`|` which appears in the rendered output.

A cell's contents are parsed as inline content, so no HTML block can open inside
one and the rule in the analysis section never applies. Any cell contents which
require a newline (such as an HTML comment with blank lines) forces the entire
table to be serialized as HTML.

| Description                       | HTML in                    | Faithful expected               |
| --------------------------------- | -------------------------- | ------------------------------- |
| `<br>` before text in a body cell | `<td><br><em>b</em></td>`  | `\|␣<br>*b*␣\|`         |
| `<br>` after text in a body cell  | `<td><em>a</em><br></td>`  | `\|␣*a*<br>␣\|`         |
| `<br>`s-only body cell            | `<td><br><br>...<br></td>` | `\|␣<br><br>...<br>␣\|` |

Lists
-----

| Description                          | HTML in                                    | Faithful expected     |
| ------------------------------------ | ------------------------------------------ | --------------------- |
| Lone `<br>`s                         | `<ul><li><br><br>...<br></li></ul>`        | `*␣␣␣<br><br>...<br>` |
| Lone `<br>` before a block           | `<ul><li><br><p>a</p></li></ul>`           | `*␣␣␣<br>a`           |
| Lone `<br>` after a block            | `<ul><li><p>a</p><br></li></ul>`           | `*␣␣␣a<br>`           |
| `<br>`-only paragraph (one `<br>`)   | `<ul><li><p><br></p></li></ul>`            | `*␣␣␣<p><br></p>`     |
| `<br>`s-only paragraph (two or more) | `<ul><li><p><br><br>...<br></p></li></ul>` | `*␣␣␣<br><br>...<br>` |

Row 4 results from the special case section above: like a blockquote, a list
item's marker is stripped like a `>`, so a bare tag re-opens as an HTML block.

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
  [thematic break](https://spec.commonmark.org/0.31.2/#thematic-breaks), a
  [GFM table](https://github.github.com/gfm/#tables-extension-), or an
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
