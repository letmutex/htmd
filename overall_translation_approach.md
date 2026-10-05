Overall translation approach
============================

Notation
--------

Notation inside a cell: `⏎` is a newline; `␣` is a space; `<br><br>...<br>`
stands for one or more `<br>` elements. Where one `<br>` and a run of them need
different Markdown or produce different translations, they get a row each.
`<br>` elements provide a concrete example which applies to any raw HTML inline.

Escaping
--------

The type of escape needed varies based on the context of the text to escape.
Material covered in
[CommonMark translation of HTML blocks and raw HTML inlines](unsupported_html.md)
is not repeated here.

Code
----

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
| `<br>`s in a code span       | `<p>a<code>x<br><br>...<br>y</code>b</p>`   | `a<code>x<br><br>...<br>y</code>b`          |
| `<br>`s alone in a code span | `<p>a<code><br><br>...<br></code>b</p>`     | `a<code><br><br>...<br></code>b`            |
| `<br>`s in a code block      | `<pre><code>a<br><br>...<br>b</code></pre>` | `<pre><code>a<br><br>...<br>b</code></pre>` |
| `<br>`s-only code block      | `<pre><code><br><br>...<br></code></pre>`   | `<pre><code><br><br>...<br></code></pre>`   |
| Adjacent code spans          | `<p><code>a</code><code>b</code></p>`       | `<code>a</code><code>b</code>`              |

Emphasis and strong emphasis
----------------------------

Notation: the characters `a` and `b` stand for characters which are neither
Unicode whitespace nor Unicode punctuation.

| Description                                     | HTML in                              | Faithful expected                    |
| ----------------------------------------------- | ------------------------------------ | ------------------------------------ |
| `<br>`s starting emphasis                       | `<p><em><br><br>...<br>a</em></p>`   | `<em><br><br>...<br>a</em>`          |
| `<br>`s ending emphasis                         | `<p><em>a<br><br>...<br></em></p>`   | `<em>a<br><br>...<br></em>`          |
| `<br>`s inside emphasis                         | `<p><em>a<br><br>...<br>b</em></p>`  | `*a<br><br>...<br>b*`                |
| `<br>`s alone in emphasis with surrounding text | `<p>a<em><br><br>...<br></em>b</p>`  | `a<em><br><br>...<br></em>b`         |
| Whitespace-only emphasis                        | `<p>a<em>␣</em>b</p>`                | `a<em>␣</em>b`                       |
| Empty emphasis                                  | `<p>a<em></em>b</p>`                 | `a<em></em>b`                        |
| Adjacent emphases                               | `<p><em>a</em><em>b</em></p>`        | `<em>a</em><em>b</em>`               |
| Emphasis inside emphasis                        | `<p><em><em>a</em></em></p>`         | `<p><em><em>a</em></em></p>`         |
| Strong emphasis inside emphasis                 | `<p><strong><em>a</em></strong></p>` | `<p><strong><em>a</em></strong></p>` |

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
CommonMark cannot encode an emphasis empty: `**` is a literal pair of asterisks,
not a strong emphasis holding nothing. Both are serialized.

Rows 7 is a pair of elements which cannot be encoded as CommonMark. `*a**b*`
pairs its delimiter runs outside in, giving a single emphasis holding `a**b`.
Pure mode writes the one element the pair reads as, combining the text of both,
which is an encoding and not a translation: it loses an element, and loses the
distinction between `<i>` and `<em>` or `<b>` and `<strong>` when the pair mixes
them. Faithful mode serializes each element of such a run instead. This applies
to a run of any inline element, so a pair which is already serialized —
`<sub>a</sub><sub>b</sub>` — stays the two elements it was rather than being
combined into one.

While strong emphasis nested inside strong emphasis (encoded as `****a****` and
strong emphasis nested inside emphasis (encoded as `***a***`) are translatable,
neither emphasis nested inside strong emphasis nor emphasis nested inside
emphasis can be encoded, as shown in the last two rows.

Approach:

1. If the emphasis string is empty once its leading and trailing whitespace is
   removed, serialize the entire element as HTML. This comes before step 3,
   which would otherwise write a bare pair of delimiters around nothing.
2. If the first or last character of the emphasis string, after removing leading
   and trailing whitespace, is a Unicode punctuation character, serialize the
   entire element as HTML. (This is conservative; if the emphasis was preceded
   or followed by Unicode whitespace, serialization isn't needed. However, the
   current implementation makes this difficult to determine.)
3. Move leading and trailing whitespace outside the emphasis string; surround
   the string with the appropriate delimiters.

The following is not implemented; it requires the ability to examine text
preceding and following emphasis. It is kept for possible future improvements.

> If the emphasis/strong emphasis has no content, it must be serialized.
> Otherwise, consider `1*233..34*5`, where:
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

Links
-----

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
    escaped. These inlines bind more tightly than a link.
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

  * All `"` and `\` characters must be escaped, since the `"` ends a title
    string, while `\` escapes the following character. This is a conservative
    selection that simplifies the translation but may insert unnecessary
    escapes. For example, the title `Say "hi"` could be encoded as `'Say "hi"'`
    instead of `"Say \"hi\""`.

Some of these escapes are already performed by generic text-node escaping,
covered in section TODO.

Images
------

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

Math
----

Like an inline code span, the contents of
[math expressions](https://pulldown-cmark.github.io/pulldown-cmark/specs/math.html)
are literal text and are not parsed by CommonMark. However, newlines in
single-line CommonMark blocks (such as a heading or a table cell) or blank lines
in other blocks end the math span and begin another block. Since LaTeX
mathematics ignores newlines and other forms of whitespace except in comments,
all newlines are replaced with spaces in math expressions to prevent these
mis-translations. Before this replacement, comments are stripped by removing all
characters from an unescaped `%` to the end of the line for each lines of
mathematics; the sequence `%` is not treated as a comment. In addition, leading
and trailing whitespace is trimmed, since inline math may not begin or end with
whitespace per the
[pulldown-cmark spec](https://pulldown-cmark.github.io/pulldown-cmark/specs/math.html).
