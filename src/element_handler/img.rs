use crate::{
    Element,
    element_handler::element_util::{serialize_element_when_faithful, serialize_if_extra_attrs},
    element_handler::{HandlerResult, Handlers},
    util::{
        escape::{escape_image_description, escape_link_destination, escape_link_title},
        text::concat_strings,
    },
};

/// Translates an `<img>` as an inline image, whatever the `LinkStyle`; see the
/// images section of `translation_approach.md`.
pub(super) fn img_handler(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    let mut link: Option<&str> = None;
    let mut alt: Option<&str> = None;
    let mut title: Option<&str> = None;
    for attr in element.attrs.iter() {
        let name = &attr.name.local;
        if name == "src" {
            link = Some(&attr.value);
        } else if name == "alt" {
            alt = Some(&attr.value);
        } else if name == "title" {
            title = Some(&attr.value);
        } else {
            serialize_if_extra_attrs!(handlers, element, 0);
        }
    }

    // Markdown can't spell a missing `src` or `alt`, which read back as empty,
    // or an empty `title`, which reads back as missing, so faithful mode keeps
    // these as HTML.
    serialize_element_when_faithful!(
        handlers,
        element,
        link.is_none() || alt.is_none() || title == Some("")
    );
    let link = escape_link_destination(link?);
    let alt = alt.map(escape_image_description);
    let title = title.map(escape_link_title);

    let (title_prefix, title_text, title_suffix) = match title.as_deref() {
        Some(t) => (" \"", t, "\""),
        None => ("", "", ""),
    };
    let md = concat_strings!(
        "![",
        alt.as_deref().unwrap_or_default(),
        "](",
        link,
        title_prefix,
        title_text,
        title_suffix,
        ")"
    );
    Some(md.into())
}
