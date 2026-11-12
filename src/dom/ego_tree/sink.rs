use std::borrow::Cow;
use std::cell::{Ref, RefCell};

use ego_tree::{NodeId, Tree};
use html5ever::tendril::StrTendril;
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, QualName};

use super::{EgoElementData, EgoNodeData};

pub(crate) struct EgoTreeSink {
    tree: RefCell<Tree<EgoNodeData>>,
}

impl Default for EgoTreeSink {
    fn default() -> Self {
        Self {
            tree: RefCell::new(Tree::new(EgoNodeData::Document)),
        }
    }
}

impl TreeSink for EgoTreeSink {
    type Output = Tree<EgoNodeData>;
    type Handle = NodeId;
    type ElemName<'a> = Ref<'a, QualName>;

    fn finish(self) -> Self::Output {
        self.tree.into_inner()
    }

    fn parse_error(&self, _msg: Cow<'static, str>) {}

    fn get_document(&self) -> Self::Handle {
        self.tree.borrow().root().id()
    }

    fn elem_name<'a>(&'a self, target: &'a Self::Handle) -> Self::ElemName<'a> {
        Ref::map(self.tree.borrow(), |tree| {
            let EgoNodeData::Element(elem) = tree.get(*target).unwrap().value() else {
                panic!("not an element!");
            };
            &elem.name
        })
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        flags: ElementFlags,
    ) -> Self::Handle {
        let mut tree = self.tree.borrow_mut();
        let template_contents = if flags.template {
            Some(tree.orphan(EgoNodeData::DocumentFragment).id())
        } else {
            None
        };
        tree.orphan(EgoNodeData::Element(Box::new(EgoElementData {
            name,
            attrs,
            template_contents,
            mathml_annotation_xml_integration_point: flags.mathml_annotation_xml_integration_point,
        })))
        .id()
    }

    fn create_comment(&self, text: StrTendril) -> Self::Handle {
        self.tree
            .borrow_mut()
            .orphan(EgoNodeData::Comment(text))
            .id()
    }

    fn create_pi(&self, target: StrTendril, data: StrTendril) -> Self::Handle {
        self.tree
            .borrow_mut()
            .orphan(EgoNodeData::ProcessingInstruction {
                target,
                contents: data,
            })
            .id()
    }

    fn append(&self, parent: &Self::Handle, child: NodeOrText<Self::Handle>) {
        let mut tree = self.tree.borrow_mut();
        let mut parent_mut = tree.get_mut(*parent).unwrap();
        match child {
            NodeOrText::AppendNode(id) => {
                parent_mut.append_id(id);
            }
            NodeOrText::AppendText(text) => {
                let appended = parent_mut.last_child().is_some_and(|mut last| {
                    if let EgoNodeData::Text(existing) = last.value() {
                        existing.push_tendril(&text);
                        true
                    } else {
                        false
                    }
                });
                if !appended {
                    parent_mut.append(EgoNodeData::Text(text));
                }
            }
        }
    }

    fn append_based_on_parent_node(
        &self,
        element: &Self::Handle,
        prev_element: &Self::Handle,
        child: NodeOrText<Self::Handle>,
    ) {
        let has_parent = self.tree.borrow().get(*element).unwrap().parent().is_some();
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        _public_id: StrTendril,
        _system_id: StrTendril,
    ) {
        self.tree
            .borrow_mut()
            .root_mut()
            .append(EgoNodeData::Doctype { name });
    }

    fn get_template_contents(&self, target: &Self::Handle) -> Self::Handle {
        let tree = self.tree.borrow();
        let EgoNodeData::Element(elem) = tree.get(*target).unwrap().value() else {
            panic!("not a template element!");
        };
        elem.template_contents.expect("not a template element!")
    }

    fn same_node(&self, x: &Self::Handle, y: &Self::Handle) -> bool {
        x == y
    }

    fn set_quirks_mode(&self, _mode: QuirksMode) {}

    fn append_before_sibling(&self, sibling: &Self::Handle, new_node: NodeOrText<Self::Handle>) {
        let mut tree = self.tree.borrow_mut();
        let mut sibling_mut = tree.get_mut(*sibling).unwrap();
        assert!(
            sibling_mut.parent().is_some(),
            "append_before_sibling called on node without parent"
        );
        match new_node {
            NodeOrText::AppendNode(id) => {
                sibling_mut.insert_id_before(id);
            }
            NodeOrText::AppendText(text) => {
                let appended = sibling_mut.prev_sibling().is_some_and(|mut prev| {
                    if let EgoNodeData::Text(existing) = prev.value() {
                        existing.push_tendril(&text);
                        true
                    } else {
                        false
                    }
                });
                if !appended {
                    sibling_mut.insert_before(EgoNodeData::Text(text));
                }
            }
        }
    }

    fn add_attrs_if_missing(&self, target: &Self::Handle, attrs: Vec<Attribute>) {
        let mut tree = self.tree.borrow_mut();
        let mut node = tree.get_mut(*target).unwrap();
        let EgoNodeData::Element(elem) = node.value() else {
            panic!("not an element");
        };
        for attr in attrs {
            if !elem.attrs.iter().any(|existing| existing.name == attr.name) {
                elem.attrs.push(attr);
            }
        }
    }

    fn remove_from_parent(&self, target: &Self::Handle) {
        self.tree.borrow_mut().get_mut(*target).unwrap().detach();
    }

    fn reparent_children(&self, node: &Self::Handle, new_parent: &Self::Handle) {
        self.tree
            .borrow_mut()
            .get_mut(*new_parent)
            .unwrap()
            .reparent_from_id_append(*node);
    }

    fn is_mathml_annotation_xml_integration_point(&self, handle: &Self::Handle) -> bool {
        let tree = self.tree.borrow();
        let EgoNodeData::Element(elem) = tree.get(*handle).unwrap().value() else {
            panic!("not an element!");
        };
        elem.mathml_annotation_xml_integration_point
    }
}
