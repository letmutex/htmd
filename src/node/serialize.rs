use html5ever::serialize::TraversalScope;

use crate::dom::{DomNode, SyntheticTextElement};

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
