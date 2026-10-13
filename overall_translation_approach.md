Overall translation approach
============================

Escaping
--------

The type of escape needed varies based on the context of the text to escape.
Material covered in
[CommonMark translation of HTML blocks and raw HTML inlines](unsupported_html.md)
is not repeated here.

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
