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
    let mut link: Option<String> = None;
    let mut alt: Option<String> = None;
    let mut title: Option<String> = None;
    for attr in element.attrs.iter() {
        let name = &attr.name.local;
        if name == "src" {
            link = Some(attr.value.to_string());
        } else if name == "alt" {
            alt = Some(attr.value.to_string());
        } else if name == "title" {
            title = Some(attr.value.to_string());
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
        link.is_none() || alt.is_none() || title.as_deref() == Some("")
    );
    let link = escape_link_destination(&link?);
    let alt = alt.as_deref().map(escape_image_description);
    let title = title.as_deref().map(escape_link_title);

    let md = concat_strings!(
        "![",
        alt.as_deref().unwrap_or_default(),
        "](",
        link,
        title
            .as_ref()
            .map_or(String::new(), |t| concat_strings!(" \"", t, "\"")),
        ")"
    );
    Some(md.into())
}
