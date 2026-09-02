use crate::{
    Element,
    element_handler::element_util::serialize_if_extra_attrs,
    element_handler::{HandlerResult, Handlers},
    options::{BrStyle, TranslationMode},
};

pub(super) fn br_handler(handlers: &dyn Handlers, element: Element) -> Option<HandlerResult> {
    serialize_if_extra_attrs!(
        handlers,
        element,
        // In faithful mode, only emit `<br>`, not one of the problematic CommonMark encodings.
        if handlers.options().translation_mode == TranslationMode::Faithful {
            -1
        } else {
            0
        }
    );

    match handlers.options().br_style {
        BrStyle::TwoSpaces => Some("  \n".into()),
        BrStyle::Backslash => Some("\\\n".into()),
    }
}
