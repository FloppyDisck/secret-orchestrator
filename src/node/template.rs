use crate::node::data::{DataType, ValueType};
use crate::node::{GraphState, NodeGraph, NodeState};
use egui_node_graph::{Graph, InputParamKind, NodeId, NodeTemplateIter, NodeTemplateTrait};
use serde_json::{Number, Value};
use std::borrow::Cow;

/// Represents the different supported node types
#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum Template {
    MakeBool,
    MakeNumber,
    MakeString,

    ConstructArray(DataType),
    DeconstructArray(DataType),

    ConstructJson,
    DeconstructJson,

    Account,
    Store,
    Instantiate,
    ConstructMsg,
    DeconstructMsg,
}

impl Template {
    pub fn is_json(&self) -> bool {
        match self {
            Template::ConstructJson | Template::DeconstructJson => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        match self {
            Template::ConstructArray(_) | Template::DeconstructArray(_) => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum IO {
    Input,
    Output,
}

pub fn add_param(id: NodeId, param_type: DataType, name: &str, io: IO, graph: &mut NodeGraph) {
    match io {
        IO::Input => {
            // Avoid duplicated
            let is_duplicate = graph[id]
                .inputs
                .iter()
                .find(|item| item.0 == name)
                .is_some();

            if !is_duplicate {
                let value = match param_type {
                    DataType::Bool => ValueType::Bool(true),
                    DataType::Number => ValueType::Number(Number::from(0)),
                    DataType::String => ValueType::String("".to_string()),
                    DataType::Array => ValueType::Array(vec![]),
                    DataType::Json => ValueType::Json(Value::default()),
                };

                graph.add_input_param(
                    id,
                    name.to_string(),
                    param_type,
                    value,
                    InputParamKind::ConnectionOrConstant,
                    true,
                );
            }
        }
        IO::Output => {
            let is_duplicate = graph[id]
                .outputs
                .iter()
                .find(|item| item.0 == name)
                .is_some();

            if !is_duplicate {
                graph.add_output_param(id, name.to_string(), param_type);
            }
        }
    };
}

pub fn remove_param(id: NodeId, name: &str, io: IO, graph: &mut NodeGraph) {
    match io {
        IO::Input => {
            let input_id = graph[id]
                .inputs
                .iter()
                .find(|item| item.0 == name)
                .unwrap()
                .1;

            graph.remove_input_param(input_id);
        }
        IO::Output => {
            let output_id = graph[id]
                .outputs
                .iter()
                .find(|item| item.0 == name)
                .unwrap()
                .1;

            graph.remove_output_param(output_id);
        }
    }
}

impl NodeTemplateTrait for Template {
    type NodeData = NodeState;
    type DataType = DataType;
    type ValueType = ValueType;
    type UserState = GraphState;

    fn node_finder_label(&self, user_state: &mut Self::UserState) -> Cow<str> {
        Cow::Borrowed(match self {
            Template::MakeBool => "Boolean",
            Template::MakeNumber => "Number",
            Template::MakeString => "String",

            Template::ConstructArray(_) => "Array Constructor",
            Template::DeconstructArray(_) => "Array Splitter",

            Template::ConstructJson => "Json Constructor",
            Template::DeconstructJson => "Json Splitter",

            Template::Account => "Account",
            Template::Store => "Store Contract",
            Template::Instantiate => "Instantiate Contract",
            Template::ConstructMsg => "Msg Constructor",
            Template::DeconstructMsg => "Msg Splitter",
        })
    }

    fn node_graph_label(&self, user_state: &mut Self::UserState) -> String {
        self.node_finder_label(user_state).into()
    }

    fn user_data(&self, user_state: &mut Self::UserState) -> Self::NodeData {
        NodeState { template: *self }
    }

    /// Only runs at node creation
    fn build_node(
        &self,
        graph: &mut Graph<Self::NodeData, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
        node_id: NodeId,
    ) {
        match self {
            Template::MakeBool => {
                add_param(node_id, DataType::Bool, "bool", IO::Input, graph);
                add_param(node_id, DataType::Bool, "out", IO::Output, graph);
            }
            Template::MakeNumber => {
                add_param(node_id, DataType::String, "number", IO::Input, graph);
                add_param(node_id, DataType::Number, "out", IO::Output, graph);
            }
            Template::MakeString => {
                add_param(node_id, DataType::String, "string", IO::Input, graph);
                add_param(node_id, DataType::String, "out", IO::Output, graph);
            }

            Template::ConstructArray(_) => {
                add_param(node_id, DataType::Array, "out", IO::Output, graph);
            }

            Template::DeconstructArray(_) => {
                add_param(node_id, DataType::Array, "array", IO::Input, graph);
            }

            Template::ConstructJson => {
                add_param(node_id, DataType::Json, "out", IO::Output, graph);
            }
            Template::DeconstructJson => {
                add_param(node_id, DataType::Json, "json", IO::Input, graph);
            }

            Template::Account => {
                add_param(node_id, DataType::String, "mnemonic", IO::Input, graph);
                // TODO: might need to create a specific account type
                add_param(node_id, DataType::String, "account", IO::Output, graph);
            }
            Template::Store => {
                add_param(node_id, DataType::String, "file", IO::Input, graph);
                add_param(node_id, DataType::String, "account", IO::Input, graph);
                add_param(node_id, DataType::Number, "id", IO::Output, graph);
                add_param(node_id, DataType::String, "code hash", IO::Output, graph);
            }
            Template::Instantiate => {
                add_param(node_id, DataType::Number, "id", IO::Input, graph);
                add_param(node_id, DataType::Json, "msg", IO::Input, graph);
                add_param(node_id, DataType::String, "label", IO::Input, graph);
                add_param(node_id, DataType::String, "account", IO::Input, graph);

                add_param(node_id, DataType::String, "address", IO::Output, graph);
            }
            Template::ConstructMsg => {
                add_param(node_id, DataType::String, "type", IO::Input, graph);
                add_param(node_id, DataType::Json, "json", IO::Input, graph);
                add_param(node_id, DataType::Json, "msg", IO::Output, graph);
            }
            Template::DeconstructMsg => {
                add_param(node_id, DataType::Json, "msg", IO::Input, graph);

                add_param(node_id, DataType::String, "type", IO::Output, graph);
                add_param(node_id, DataType::Json, "json", IO::Output, graph);
            }
        }
    }
}

// Helper
pub struct TemplateIterator;
impl NodeTemplateIter for TemplateIterator {
    type Item = Template;

    fn all_kinds(&self) -> Vec<Self::Item> {
        vec![
            Template::MakeBool,
            Template::MakeNumber,
            Template::MakeString,
            Template::ConstructArray(DataType::Bool),
            Template::DeconstructArray(DataType::Bool),
            Template::ConstructJson,
            Template::DeconstructJson,
            Template::Account,
            Template::Store,
            Template::Instantiate,
            Template::ConstructMsg,
            Template::DeconstructMsg,
        ]
    }
}
