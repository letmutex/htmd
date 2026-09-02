use crate::{
    Element,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers},
    text_util::{concat_strings, normalize_title},
};

pub(super) fn img_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    let mut link: Option<String> = None;
    let mut alt: Option<String> = None;
    let mut title: Option<String> = None;
    for attr in element.attrs.iter() {
        let name = &attr.name.local;
        if name == "href" {
            link = Some(attr.value.to_string())
        } else if name == "src" {
            link = Some(attr.value.to_string());
        } else if name == "alt" {
            alt = Some(attr.value.to_string());
        } else if name == "title" {
            title = Some(attr.value.to_string());
        } else {
            serialize_if_extra_attrs!(handlers, element, 0);
        }
    }

    link.as_ref()?;

    // Handle new lines in alt and in title
    let alt = alt.as_deref().map(normalize_title);
    let title = title.as_deref().map(normalize_title);

    let link = link.map(|text| text.replace('(', "\\(").replace(')', "\\)"));

    let has_spaces_in_link = link.as_ref().is_some_and(|link| link.contains(' '));

    let md = concat_strings!(
        "![",
        alt.as_ref().unwrap_or(&String::new()),
        "](",
        if has_spaces_in_link { "<" } else { "" },
        link.as_ref().unwrap_or(&String::new()),
        if has_spaces_in_link { ">" } else { "" },
        title
            .as_ref()
            .map_or(String::new(), |t| concat_strings!(" \"", t, "\"")),
        ")"
    );
    Some(md.into())
}
