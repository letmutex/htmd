use std::io;

use ego_tree::NodeRef;
use ego_tree::iter::Edge;
use html5ever::serialize::{Serialize, SerializeOpts, Serializer, TraversalScope, serialize};

use super::EgoNodeData;

struct EgoSerializable<'a>(NodeRef<'a, EgoNodeData>);

impl Serialize for EgoSerializable<'_> {
    fn serialize<S>(&self, serializer: &mut S, traversal_scope: TraversalScope) -> io::Result<()>
    where
        S: Serializer,
    {
        for edge in self.0.traverse() {
            match edge {
                Edge::Open(node) => {
                    if node == self.0 && matches!(traversal_scope, TraversalScope::ChildrenOnly(_))
                    {
                        continue;
                    }
                    match node.value() {
                        EgoNodeData::Doctype { name } => serializer.write_doctype(name)?,
                        EgoNodeData::Comment(contents) => serializer.write_comment(contents)?,
                        EgoNodeData::Text(contents) => serializer.write_text(contents)?,
                        EgoNodeData::Element(elem) => {
                            let attrs = elem.attrs.iter().map(|attr| (&attr.name, &attr.value[..]));
                            serializer.start_elem(elem.name.clone(), attrs)?;
                        }
                        EgoNodeData::ProcessingInstruction { target, contents } => {
                            serializer.write_processing_instruction(target, contents)?;
                        }
                        EgoNodeData::Document | EgoNodeData::DocumentFragment => {}
                    }
                }
                Edge::Close(node) => {
                    if node == self.0 && matches!(traversal_scope, TraversalScope::ChildrenOnly(_))
                    {
                        continue;
                    }
                    if let EgoNodeData::Element(elem) = node.value() {
                        serializer.end_elem(elem.name.clone())?;
                    }
                }
            }
        }
        Ok(())
    }
}

pub(super) fn serialize_ego_node(
    node: NodeRef<'_, EgoNodeData>,
    traversal_scope: TraversalScope,
) -> io::Result<String> {
    let scope = match node.value() {
        EgoNodeData::Document | EgoNodeData::DocumentFragment => TraversalScope::ChildrenOnly(None),
        _ => traversal_scope,
    };
    let opts = SerializeOpts {
        traversal_scope: scope,
        ..Default::default()
    };
    let mut bytes = Vec::new();
    serialize(&mut bytes, &EgoSerializable(node), opts)?;
    String::from_utf8(bytes).map_err(io::Error::other)
}
