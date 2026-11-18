use crate::{
    element_handler::{
        Element, HandlerResult, Handlers, element_util::serialize_if_extra_attrs_or_inline,
    },
    options::HrStyle,
};

pub(super) fn hr_handler(handlers: &dyn Handlers, element: &Element) -> Option<HandlerResult> {
    serialize_if_extra_attrs_or_inline!(handlers, element, 0);
    match handlers.options().hr_style {
        HrStyle::Dashes => Some("\n\n- - -\n\n".into()),
        HrStyle::Asterisks => Some("\n\n* * *\n\n".into()),
        HrStyle::Underscores => Some("\n\n_ _ _\n\n".into()),
    }
}
