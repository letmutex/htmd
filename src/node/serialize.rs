use html5ever::serialize::{HtmlSerializer, SerializeOpts, Serializer, TraversalScope};
use html5ever::{QualName, local_name, ns};

use crate::dom::{DomNode, SyntheticTextElement};
use crate::util::text::concat_strings;

use super::{Node, NodeInner};

impl Node<'_> {
    pub(crate) fn serialize_html(
        &self,
        traversal_scope: TraversalScope,
    ) -> std::io::Result<String> {
        match &self.0 {
            #[cfg(feature = "ego-tree")]
            NodeInner::Ego(node) => DomNode::serialize_html(*node, traversal_scope),
            #[cfg(feature = "rcdom")]
            NodeInner::Rc(node) => DomNode::serialize_html(node.as_rc(), traversal_scope),
            NodeInner::SyntheticElement(synth) => serialize_synthetic(synth, traversal_scope),
            NodeInner::SyntheticText(synth) => serialize_text_str(synth.text),
        }
    }

    pub(crate) fn serialize_opening_tag(&self) -> std::io::Result<Option<String>> {
        let Some(name) = self.qual_name() else {
            return Ok(None);
        };
        let opts = SerializeOpts {
            traversal_scope: TraversalScope::IncludeNode,
            ..Default::default()
        };
        let mut bytes = Vec::new();
        let mut serializer = HtmlSerializer::new(&mut bytes, opts);
        let attrs = self.attrs();
        serializer.start_elem(
            name.clone(),
            attrs
                .as_slice()
                .iter()
                .map(|attr| (&attr.name, &attr.value[..])),
        )?;
        String::from_utf8(bytes)
            .map(Some)
            .map_err(std::io::Error::other)
    }

    pub(crate) fn serialize_closing_tag(&self) -> Option<String> {
        let name = self.qual_name()?;
        if is_void_html_element(name) {
            return Some(String::new());
        }
        Some(concat_strings!("</", &name.local, ">"))
    }
}

fn is_void_html_element(name: &QualName) -> bool {
    name.ns == ns!(html)
        && matches!(
            name.local,
            local_name!("area")
                | local_name!("base")
                | local_name!("basefont")
                | local_name!("bgsound")
                | local_name!("br")
                | local_name!("col")
                | local_name!("embed")
                | local_name!("frame")
                | local_name!("hr")
                | local_name!("img")
                | local_name!("input")
                | local_name!("keygen")
                | local_name!("link")
                | local_name!("meta")
                | local_name!("param")
                | local_name!("source")
                | local_name!("track")
                | local_name!("wbr")
        )
}

fn serialize_synthetic(
    synth: &SyntheticTextElement<'_>,
    traversal_scope: TraversalScope,
) -> std::io::Result<String> {
    use html5ever::serialize::{HtmlSerializer, SerializeOpts, Serializer};
    use std::io::Write;

    let include_node = matches!(traversal_scope, TraversalScope::IncludeNode);
    let mut bytes = Vec::new();
    let opts = SerializeOpts {
        traversal_scope,
        ..Default::default()
    };
    let mut serializer = HtmlSerializer::new(&mut bytes, opts);
    if include_node {
        serializer.start_elem(
            synth.qual_name.clone(),
            synth.attrs.iter().map(|attr| (&attr.name, &attr.value[..])),
        )?;
    }
    serializer.write_text(synth.text)?;
    if include_node {
        serializer.end_elem(synth.qual_name.clone())?;
    }
    serializer.writer.flush()?;
    String::from_utf8(bytes).map_err(std::io::Error::other)
}

fn serialize_text_str(text: &str) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    html5ever::serialize::serialize(
        &mut bytes,
        &TextSerializable(text),
        html5ever::serialize::SerializeOpts {
            traversal_scope: TraversalScope::IncludeNode,
            ..Default::default()
        },
    )?;
    String::from_utf8(bytes).map_err(std::io::Error::other)
}

struct TextSerializable<'a>(&'a str);

impl html5ever::serialize::Serialize for TextSerializable<'_> {
    fn serialize<S: html5ever::serialize::Serializer>(
        &self,
        serializer: &mut S,
        _traversal_scope: TraversalScope,
    ) -> std::io::Result<()> {
        serializer.write_text(self.0)
    }
}
