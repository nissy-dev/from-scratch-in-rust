//! This module includes some implementations on node styles.

use crate::{
    css::{CSSValue, Stylesheet},
    dom::{Node, NodeType},
};
use std::collections::HashMap;

pub type PropertyMap = HashMap<String, CSSValue>;

#[derive(Debug, PartialEq)]
pub enum Display {
    Inline,
    Block,
    None,
}

/// `StyledNode` wraps `Node` with related CSS properties.
/// It forms a tree as `Node` does.
#[derive(Debug, PartialEq)]
pub struct StyledNode<'a> {
    pub node_type: &'a NodeType,
    pub children: Vec<StyledNode<'a>>,

    pub properties: PropertyMap,
}

pub fn to_styled_node<'a>(node: &'a Box<Node>, stylesheet: &Stylesheet) -> Option<StyledNode<'a>> {
    let mut properties: PropertyMap = [].iter().cloned().collect();
    for matched_rule in stylesheet.rules.iter().filter(|r| r.matches(node)) {
        for declaration in &matched_rule.declarations {
            properties.insert(declaration.name.clone(), declaration.value.clone());
        }
    }
    if properties.get("display") == Some(&CSSValue::Keyword("none".into())) {
        return None;
    }

    let children = node
        .children
        .iter()
        .filter_map(|x| to_styled_node(x, stylesheet))
        .collect();

    Some(StyledNode {
        node_type: &node.node_type,
        properties,
        children,
    })
}

impl<'a> StyledNode<'a> {
    pub fn display(&self) -> Display {
        match self.properties.get("display") {
            Some(CSSValue::Keyword(s)) => match s.as_str() {
                "block" => Display::Block,
                "none" => Display::None,
                _ => Display::Inline,
            },
            _ => Display::Inline,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        css::{AttributeSelectorOp, Declaration, Rule, SimpleSelector},
        dom::Element,
    };

    use super::*;

    #[test]
    fn test_to_styled_node_single() {
        let e = &Element::new(
            "p".to_string(),
            [("id".to_string(), "test".to_string())]
                .iter()
                .cloned()
                .collect(),
            vec![],
        );
        let testcases = vec![
            (
                // * { display: block; }
                Stylesheet::new(vec![Rule {
                    selectors: vec![SimpleSelector::UniversalSelector],
                    declarations: vec![Declaration {
                        name: "display".to_string(),
                        value: CSSValue::Keyword("block".to_string()),
                    }],
                }]),
                vec![(
                    "display".to_string(),
                    CSSValue::Keyword("block".to_string()),
                )],
            ),
            (
                // div { display: block; }
                Stylesheet::new(vec![Rule {
                    selectors: vec![SimpleSelector::TypeSelector {
                        tag_name: "div".into(),
                    }],
                    declarations: vec![Declaration {
                        name: "display".into(),
                        value: CSSValue::Keyword("block".to_string()),
                    }],
                }]),
                vec![],
            ),
            (
                // * { display: block; }
                // div { display: inline; }
                Stylesheet::new(vec![
                    Rule {
                        selectors: vec![SimpleSelector::UniversalSelector],
                        declarations: vec![Declaration {
                            name: "display".to_string(),
                            value: CSSValue::Keyword("block".into()),
                        }],
                    },
                    Rule {
                        selectors: vec![SimpleSelector::TypeSelector {
                            tag_name: "div".into(),
                        }],
                        declarations: vec![Declaration {
                            name: "display".into(),
                            value: CSSValue::Keyword("inline".into()),
                        }],
                    },
                ]),
                vec![(
                    "display".to_string(),
                    CSSValue::Keyword("block".to_string()),
                )],
            ),
            (
                // * { display: block; }
                // p { display: inline; testname: testvalue; }
                Stylesheet::new(vec![
                    Rule {
                        selectors: vec![SimpleSelector::UniversalSelector],
                        declarations: vec![Declaration {
                            name: "display".to_string(),
                            value: CSSValue::Keyword("block".into()),
                        }],
                    },
                    Rule {
                        selectors: vec![SimpleSelector::TypeSelector {
                            tag_name: "p".into(),
                        }],
                        declarations: vec![
                            Declaration {
                                name: "display".into(),
                                value: CSSValue::Keyword("inline".into()),
                            },
                            Declaration {
                                name: "testname".into(),
                                value: CSSValue::Keyword("testvalue".into()),
                            },
                        ],
                    },
                ]),
                vec![
                    ("display".into(), CSSValue::Keyword("inline".into())),
                    ("testname".into(), CSSValue::Keyword("testvalue".into())),
                ],
            ),
            (
                // * { display: block; }
                // p[id=hello] { testname: testvalue; }
                Stylesheet::new(vec![
                    Rule {
                        selectors: vec![SimpleSelector::UniversalSelector],
                        declarations: vec![Declaration {
                            name: "display".to_string(),
                            value: CSSValue::Keyword("block".into()),
                        }],
                    },
                    Rule {
                        selectors: vec![SimpleSelector::AttributeSelector {
                            tag_name: "p".into(),
                            op: AttributeSelectorOp::Eq,
                            attribute: "id".into(),
                            value: "hello".into(),
                        }],
                        declarations: vec![Declaration {
                            name: "testname".into(),
                            value: CSSValue::Keyword("testvalue".into()),
                        }],
                    },
                ]),
                vec![("display".into(), CSSValue::Keyword("block".into()))],
            ),
            (
                // * { display: block; }
                // p[id=hello] { testname: testvalue; }
                Stylesheet::new(vec![
                    Rule {
                        selectors: vec![SimpleSelector::UniversalSelector],
                        declarations: vec![Declaration {
                            name: "display".to_string(),
                            value: CSSValue::Keyword("block".into()),
                        }],
                    },
                    Rule {
                        selectors: vec![SimpleSelector::AttributeSelector {
                            tag_name: "p".into(),
                            op: AttributeSelectorOp::Eq,
                            attribute: "id".into(),
                            value: "test".into(),
                        }],
                        declarations: vec![Declaration {
                            name: "testname".into(),
                            value: CSSValue::Keyword("testvalue".into()),
                        }],
                    },
                ]),
                vec![
                    ("display".into(), CSSValue::Keyword("block".into())),
                    ("testname".into(), CSSValue::Keyword("testvalue".into())),
                ],
            ),
        ];

        for (stylesheet, properties) in testcases {
            assert_eq!(
                to_styled_node(e, &stylesheet),
                Some(StyledNode {
                    node_type: &e.node_type,
                    properties: properties.iter().cloned().collect(),
                    children: vec![],
                })
            );
        }
    }

    #[test]
    fn test_to_styled_node_nested() {
        let parent = &Element::new(
            "div".to_string(),
            [("id".to_string(), "test".to_string())]
                .iter()
                .cloned()
                .collect(),
            vec![Element::new(
                "p".to_string(),
                [("id".to_string(), "test".to_string())]
                    .iter()
                    .cloned()
                    .collect(),
                vec![],
            )],
        );
        let child_node_type = Element::new(
            "p".to_string(),
            [("id".to_string(), "test".to_string())]
                .iter()
                .cloned()
                .collect(),
            vec![],
        )
        .node_type;

        {
            // * { display: block; }
            let stylesheet = Stylesheet::new(vec![Rule {
                selectors: vec![SimpleSelector::UniversalSelector],
                declarations: vec![Declaration {
                    name: "display".to_string(),
                    value: CSSValue::Keyword("block".to_string()),
                }],
            }]);

            assert_eq!(
                to_styled_node(parent, &stylesheet),
                Some(StyledNode {
                    node_type: &parent.node_type,
                    properties: [(
                        "display".to_string(),
                        CSSValue::Keyword("block".to_string()),
                    )]
                    .iter()
                    .cloned()
                    .collect(),
                    children: vec![StyledNode {
                        node_type: &child_node_type,
                        properties: [(
                            "display".to_string(),
                            CSSValue::Keyword("block".to_string()),
                        )]
                        .iter()
                        .cloned()
                        .collect(),
                        children: vec![],
                    }],
                })
            );
        }

        {
            // p { display: block; }
            let stylesheet = Stylesheet::new(vec![Rule {
                selectors: vec![SimpleSelector::TypeSelector {
                    tag_name: "p".into(),
                }],
                declarations: vec![Declaration {
                    name: "display".to_string(),
                    value: CSSValue::Keyword("block".to_string()),
                }],
            }]);

            assert_eq!(
                to_styled_node(parent, &stylesheet),
                Some(StyledNode {
                    node_type: &parent.node_type,
                    properties: [].iter().cloned().collect(),
                    children: vec![StyledNode {
                        node_type: &child_node_type,
                        properties: [(
                            "display".to_string(),
                            CSSValue::Keyword("block".to_string()),
                        )]
                        .iter()
                        .cloned()
                        .collect(),
                        children: vec![],
                    }],
                })
            );
        }
    }

    #[test]
    fn test_to_styled_node_nested_single() {
        let parent = &Element::new(
            "div".to_string(),
            [("id".to_string(), "test".to_string())]
                .iter()
                .cloned()
                .collect(),
            vec![],
        );

        // p { display: none; }
        let stylesheet = Stylesheet::new(vec![Rule {
            selectors: vec![SimpleSelector::TypeSelector {
                tag_name: "div".into(),
            }],
            declarations: vec![Declaration {
                name: "display".to_string(),
                value: CSSValue::Keyword("none".to_string()),
            }],
        }]);

        assert_eq!(to_styled_node(parent, &stylesheet), None);
    }

    #[test]
    fn test_to_styled_node_nested_none() {
        let parent = &Element::new(
            "div".to_string(),
            [("id".to_string(), "test".to_string())]
                .iter()
                .cloned()
                .collect(),
            vec![Element::new(
                "p".to_string(),
                [("id".to_string(), "test".to_string())]
                    .iter()
                    .cloned()
                    .collect(),
                vec![],
            )],
        );

        // p { display: none; }
        let stylesheet = Stylesheet::new(vec![Rule {
            selectors: vec![SimpleSelector::TypeSelector {
                tag_name: "p".into(),
            }],
            declarations: vec![Declaration {
                name: "display".to_string(),
                value: CSSValue::Keyword("none".to_string()),
            }],
        }]);

        assert_eq!(
            to_styled_node(parent, &stylesheet),
            Some(StyledNode {
                node_type: &parent.node_type,
                properties: [].iter().cloned().collect(),
                children: vec![],
            })
        );
    }
}
