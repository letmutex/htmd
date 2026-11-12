mod dom_node;
mod serialize;
mod sink;

use ego_tree::NodeId;
use html5ever::tendril::StrTendril;
use html5ever::{Attribute, QualName};

pub(crate) use sink::EgoTreeSink;

#[derive(Debug, Clone)]
pub(crate) enum EgoNodeData {
    Document,
    DocumentFragment,
    Doctype {
        name: StrTendril,
    },
    Text(StrTendril),
    Comment(StrTendril),
    Element(Box<EgoElementData>),
    ProcessingInstruction {
        target: StrTendril,
        contents: StrTendril,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct EgoElementData {
    pub(crate) name: QualName,
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) template_contents: Option<NodeId>,
    pub(crate) mathml_annotation_xml_integration_point: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use html5ever::tree_builder::{ElementFlags, NodeOrText, TreeSink};
    use html5ever::{LocalName, QualName, ns};

    use crate::NodeType;

    fn el_name(local: &str) -> QualName {
        QualName::new(None, ns!(html), LocalName::from(local))
    }

    #[test]
    fn processing_instruction_is_ignored_and_serialized() {
        let sink = EgoTreeSink::default();
        let doc = sink.get_document();
        let pi = sink.create_pi("xml".into(), "version=\"1.0\"".into());
        sink.append(&doc, NodeOrText::AppendNode(pi));

        let tree = crate::Tree(crate::node::TreeInner::Ego(sink.finish()));
        let pi_node = tree.root().first_child().unwrap();
        assert_eq!(NodeType::ProcessingInstruction, pi_node.node_type());
        assert_eq!(None, pi_node.tag_name());
        assert_eq!("<?xml version=\"1.0\">", pi_node.html());
        assert_eq!("", crate::convert(&tree).unwrap());
    }

    #[test]
    fn sink_append_before_sibling_reparent_and_template() {
        let sink = EgoTreeSink::default();
        let doc = sink.get_document();
        let div = sink.create_element(el_name("div"), vec![], ElementFlags::default());
        let container = sink.create_element(el_name("section"), vec![], ElementFlags::default());
        let p = sink.create_element(el_name("p"), vec![], ElementFlags::default());
        let span = sink.create_element(el_name("span"), vec![], ElementFlags::default());
        let mut template_flags = ElementFlags::default();
        template_flags.template = true;
        let template = sink.create_element(el_name("template"), vec![], template_flags);

        sink.append(&doc, NodeOrText::AppendNode(div));
        sink.append(&doc, NodeOrText::AppendNode(container));
        sink.append(&div, NodeOrText::AppendNode(p));
        sink.append(&div, NodeOrText::AppendNode(span));
        sink.append_before_sibling(&span, NodeOrText::AppendText("a".into()));
        sink.append_before_sibling(&span, NodeOrText::AppendText("b".into()));
        sink.append_before_sibling(&p, NodeOrText::AppendNode(span));

        let frag = sink.get_template_contents(&template);
        let tpl_p = sink.create_element(el_name("p"), vec![], ElementFlags::default());
        sink.append(&frag, NodeOrText::AppendNode(tpl_p));
        sink.reparent_children(&div, &container);

        let tree = crate::Tree(crate::node::TreeInner::Ego(sink.finish()));
        assert_eq!(
            "<div></div><section><span></span><p></p>ab</section>",
            tree.root().html()
        );
    }
}
