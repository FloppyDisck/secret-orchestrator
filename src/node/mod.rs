// TODO: Figure out flexible json and array
//       Store both current node data and "processed data"

// TODO: node outline is red if errored out, maybe show error on hover

// TODO: save or clone nodes

// TODO: infinite loop countermeasure maybe using petgraph

// TODO: array to json not working
// TODO: json split not working

pub mod data;
pub mod template;

use crate::node::data::{DataType, ValueType};
use crate::node::template::{add_param, remove_param, Template, TemplateIterator, IO};
use anyhow::anyhow;
use eframe::egui::{Checkbox, ComboBox, Context, DragValue, Slider, TextEdit, TextStyle, Ui};
use eframe::{egui, App, Frame};
use egui_node_graph::{
    DataTypeTrait, Graph, GraphEditorState, InputParamKind, NodeDataTrait, NodeId, NodeResponse,
    NodeTemplateIter, NodeTemplateTrait, OutputId, UserResponseTrait, WidgetValueTrait,
};
use serde_json::{json, Map, Number, Value};
use std::borrow::Cow;
use std::collections::HashMap;
use std::default::Default;
use std::str::FromStr;

/// The node's state
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeState {
    template: Template,
}

/// Code side effects not supported by the lib
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    /// Determines which node output to track
    SetActiveNode(NodeId),
    /// Stops tracking node output
    ClearActiveNode,

    /// Determines which json/array node to edit
    SetEditingNode(NodeId),
    /// Finished editing the json/array node
    ClearEditingNode,

    AddParam(NodeId),
    RemoveParam(NodeId),

    UpdateArrayType(NodeId),
}

/// Global state for the graph side effects to add extra functionality
#[derive(Default)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct GraphState {
    pub active_node: Option<NodeId>,

    pub editing_node: Option<NodeId>,
    pub json_name: String,
    pub new_type: DataType,
}

type NodeGraph = Graph<NodeState, DataType, ValueType>;

impl UserResponseTrait for Response {}
impl NodeDataTrait for NodeState {
    type Response = Response;
    type UserState = GraphState;
    type DataType = DataType;
    type ValueType = ValueType;

    fn bottom_ui(
        &self,
        ui: &mut Ui,
        node_id: NodeId,
        graph: &Graph<Self, Self::DataType, Self::ValueType>,
        user_state: &mut Self::UserState,
    ) -> Vec<NodeResponse<Self::Response, Self>>
    where
        Self::Response: UserResponseTrait,
    {
        let mut responses = vec![];

        let is_editing = user_state
            .editing_node
            .map(|id| id == node_id)
            .unwrap_or(false);

        // TODO: construct array logic
        //      Must have add/remove button that just increases the total of bubbles,
        //      Have a type dropdown and an update button that force updated all params to that type

        if graph[node_id].user_data.template.is_json()
            || graph[node_id].user_data.template.is_array()
        {
            if !is_editing {
                if ui.button("Edit").clicked() {
                    responses.push(NodeResponse::User(Response::SetEditingNode(node_id)))
                }
            } else {
                ui.horizontal(|ui| {
                    if graph[node_id].user_data.template.is_json() {
                        ui.add(TextEdit::singleline(&mut user_state.json_name));
                    }
                    user_state.new_type.combo_box(ui);
                    if graph[node_id].user_data.template.is_array() {
                        if ui.button("Update").clicked() {
                            responses.push(NodeResponse::User(Response::UpdateArrayType(node_id)))
                        }
                    }
                    if ui.button("Add").clicked() {
                        responses.push(NodeResponse::User(Response::AddParam(node_id)))
                    };
                    if self.template == Template::DeconstructJson || self.template.is_array() {
                        if ui.button("Remove").clicked() {
                            responses.push(NodeResponse::User(Response::RemoveParam(node_id)))
                        }
                    }
                });
                if ui.button("Done").clicked() {
                    responses.push(NodeResponse::User(Response::ClearEditingNode))
                }
            }
        }

        let is_active = user_state
            .active_node
            .map(|id| id == node_id)
            .unwrap_or(false);

        if !is_active {
            if ui.button("👁 Set active").clicked() {
                responses.push(NodeResponse::User(Response::SetActiveNode(node_id)));
            }
        } else {
            let button =
                egui::Button::new(egui::RichText::new("👁 Active").color(egui::Color32::BLACK))
                    .fill(egui::Color32::GOLD);
            if ui.add(button).clicked() {
                responses.push(NodeResponse::User(Response::ClearActiveNode));
            }
        }

        responses
    }
}

type EditorState = GraphEditorState<NodeState, DataType, ValueType, Template, GraphState>;
#[derive(Default)]
pub struct NodeGraphState {
    state: EditorState,
    user_state: GraphState,
}

#[cfg(feature = "persistence")]
const PERSISTENCE_KEY: &str = "egui_node_graph";

#[cfg(feature = "persistence")]
impl NodeGraphState {
    /// If the persistence feature is enabled, Called once before the first frame.
    /// Load previous app state (if any).
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let state = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, PERSISTENCE_KEY))
            .unwrap_or_default();
        Self {
            state,
            user_state: GraphState::default(),
        }
    }
}

impl App for NodeGraphState {
    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, PERSISTENCE_KEY, &self.state);
    }

    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::widgets::global_dark_light_mode_switch(ui);
            });
        });

        let graph_response = egui::CentralPanel::default()
            .show(ctx, |ui| {
                self.state
                    .draw_graph_editor(ui, TemplateIterator, &mut self.user_state)
            })
            .inner;
        for node_response in graph_response.node_responses {
            // Here, we ignore all other graph events. But you may find
            // some use for them. For example, by playing a sound when a new
            // connection is created

            if let NodeResponse::User(user_event) = node_response {
                match user_event {
                    Response::SetActiveNode(node) => self.user_state.active_node = Some(node),
                    Response::ClearActiveNode => self.user_state.active_node = None,
                    Response::SetEditingNode(node) => {
                        self.user_state.editing_node = Some(node);
                        self.user_state.json_name.clear();
                        self.user_state.new_type = match self.state.graph[node].user_data.template {
                            Template::ConstructArray(data) => data,
                            Template::DeconstructArray(data) => data,
                            _ => DataType::Bool,
                        };
                    }
                    Response::ClearEditingNode => {
                        self.user_state.editing_node = None;
                    }
                    Response::AddParam(id) => {
                        let types = match self.state.graph[id].user_data.template {
                            Template::ConstructJson => Some((IO::Input, None)),
                            Template::DeconstructJson => Some((IO::Output, None)),
                            Template::ConstructArray(data) => Some((IO::Input, Some(data))),
                            Template::DeconstructArray(data) => Some((IO::Output, Some(data))),
                            _ => None,
                        };

                        if let Some((io, array_type)) = types {
                            // Work on json
                            if let Some(array_type) = array_type {
                                let total = match io {
                                    IO::Input => self.state.graph[id].inputs.len(),
                                    IO::Output => self.state.graph[id].outputs.len(),
                                };

                                add_param(
                                    id,
                                    array_type,
                                    &total.to_string(),
                                    io,
                                    &mut self.state.graph,
                                );
                            } else {
                                add_param(
                                    id,
                                    self.user_state.new_type,
                                    &self.user_state.json_name,
                                    io,
                                    &mut self.state.graph,
                                );
                            }
                        }

                        self.user_state.json_name.clear();
                    }
                    Response::RemoveParam(id) => {
                        let types = match self.state.graph[id].user_data.template {
                            Template::ConstructJson => Some((IO::Input, false)),
                            Template::DeconstructJson => Some((IO::Output, false)),
                            Template::ConstructArray(_) => Some((IO::Input, true)),
                            Template::DeconstructArray(_) => Some((IO::Output, true)),
                            _ => None,
                        };

                        if let Some((io, is_array)) = types {
                            if is_array {
                                let name = (match io {
                                    IO::Input => self.state.graph[id].inputs.len(),
                                    IO::Output => self.state.graph[id].outputs.len(),
                                } - 1)
                                    .to_string();

                                remove_param(id, &name, io, &mut self.state.graph);
                            } else {
                                remove_param(
                                    id,
                                    &self.user_state.json_name,
                                    io,
                                    &mut self.state.graph,
                                );
                            }
                        }
                    }
                    Response::UpdateArrayType(id) => {
                        // We need to rewrite all of the params
                        let types = match self.state.graph[id].user_data.template {
                            Template::ConstructArray(data) => Some((IO::Input, data)),
                            Template::DeconstructArray(data) => Some((IO::Output, data)),
                            _ => None,
                        };

                        if let Some((io, data)) = types {
                            // Avoid updating if we arent changing the data type
                            if data != self.user_state.new_type {
                                let total = match io {
                                    IO::Input => self.state.graph[id].inputs.len(),
                                    IO::Output => self.state.graph[id].outputs.len(),
                                };

                                // Remove all io
                                for i in 0..total {
                                    remove_param(id, &i.to_string(), io, &mut self.state.graph);
                                }

                                // Add all io
                                for i in 0..total {
                                    add_param(
                                        id,
                                        self.user_state.new_type,
                                        &i.to_string(),
                                        io,
                                        &mut self.state.graph,
                                    );
                                }

                                self.state.graph[id].user_data.template =
                                    match self.state.graph[id].user_data.template {
                                        Template::ConstructArray(_) => {
                                            Template::ConstructArray(self.user_state.new_type)
                                        }
                                        _ => Template::DeconstructArray(self.user_state.new_type),
                                    }
                            }
                        }
                    }
                }
            }
        }

        if let Some(node) = self.user_state.active_node {
            if self.state.graph.nodes.contains_key(node) {
                let text = match evaluate_node(&self.state.graph, node, &mut HashMap::new()) {
                    Ok(value) => {
                        if value.len() == 1 {
                            format!("The result is: {:?}", value[0])
                        } else {
                            format!("The result is: {:?}", value)
                        }
                    }
                    Err(err) => format!("Execution error: {}", err),
                };
                ctx.debug_painter().text(
                    egui::pos2(10.0, 35.0),
                    egui::Align2::LEFT_TOP,
                    text,
                    TextStyle::Button.resolve(&ctx.style()),
                    egui::Color32::WHITE,
                );
            } else {
                self.user_state.active_node = None;
            }
        }
    }
}

type OutputsCache = HashMap<OutputId, ValueType>;

// Recursively evaluates all dependencies of this node, then evaluates the node itself.
pub fn evaluate_node(
    graph: &NodeGraph,
    node_id: NodeId,
    outputs_cache: &mut OutputsCache,
) -> anyhow::Result<Vec<String>> {
    // To solve a similar problem as creating node types above, we define an
    // Evaluator as a convenience. It may be overkill for this small example,
    // but something like this makes the code much more readable when the
    // number of nodes starts growing.

    struct Evaluator<'a> {
        graph: &'a NodeGraph,
        outputs_cache: &'a mut OutputsCache,
        node_id: NodeId,
    }
    impl<'a> Evaluator<'a> {
        fn new(graph: &'a NodeGraph, outputs_cache: &'a mut OutputsCache, node_id: NodeId) -> Self {
            Self {
                graph,
                outputs_cache,
                node_id,
            }
        }
        fn evaluate_input(&mut self, name: &str) -> anyhow::Result<ValueType> {
            // Calling `evaluate_input` recursively evaluates other nodes in the
            // graph until the input value for a parameter has been computed.
            evaluate_input(self.graph, self.node_id, name, self.outputs_cache)
        }
        fn populate_output(&mut self, name: &str, value: ValueType) -> anyhow::Result<ValueType> {
            // TODO: improve with cache comparation
            // After computing an output, we don't just return it, but we also
            // populate the outputs cache with it. This ensures the evaluation
            // only ever computes an output once.
            //
            // The return value of the function is the "final" output of the
            // node, the thing we want to get from the evaluation. The example
            // would be slightly more contrived when we had multiple output
            // values, as we would need to choose which of the outputs is the
            // one we want to return. Other outputs could be used as
            // intermediate values.
            //
            // Note that this is just one possible semantic interpretation of
            // the graphs, you can come up with your own evaluation semantics!
            populate_output(self.graph, self.outputs_cache, self.node_id, name, value)
        }
    }

    let node = &graph[node_id];
    let mut evaluator = Evaluator::new(graph, outputs_cache, node_id);
    match node.user_data.template {
        // TODO: finish
        Template::MakeBool => {
            let bool = evaluator.evaluate_input("bool")?.try_into()?;
            Ok(vec![evaluator
                .populate_output("out", ValueType::Bool(bool))?
                .try_into()?])
        }
        Template::MakeNumber => {
            let mut num: String = evaluator.evaluate_input("number")?.try_into()?;
            if num == "" {
                num = "0".to_string();
            }
            Ok(vec![evaluator
                .populate_output("out", ValueType::Number(Number::from_str(&num)?))?
                .try_into()?])
        }
        Template::MakeString => {
            let string = evaluator.evaluate_input("string")?.try_into()?;
            Ok(vec![evaluator
                .populate_output("out", ValueType::String(string))?
                .try_into()?])
        }
        Template::ConstructJson => {
            let mut raw_json = Map::new();
            for (input, _) in evaluator.graph[node_id].inputs.iter() {
                let res = evaluator.evaluate_input(&input)?.try_into()?;
                raw_json.insert(input.to_string(), res);
            }
            Ok(vec![evaluator
                .populate_output("out", ValueType::Json(Value::Object(raw_json)))?
                .try_into()?])
        }
        Template::DeconstructJson => {
            let input: Value = evaluator.evaluate_input("json")?.try_into()?;

            let mut res: Vec<String> = vec![];

            for (name, id) in evaluator.graph[node_id].outputs.iter() {
                // Check the output type
                let output = match evaluator.graph.outputs.get(*id).unwrap().typ {
                    DataType::Bool => ValueType::Bool(input[name].as_bool().unwrap_or_default()),
                    DataType::Number => {
                        let number = match input[name].clone() {
                            Value::Number(n) => n,
                            _ => Number::from(0),
                        };
                        ValueType::Number(number)
                    }
                    DataType::String => {
                        ValueType::String(input[name].as_str().unwrap_or_default().to_string())
                    }
                    DataType::Array => {
                        ValueType::Array(input[name].as_array().unwrap_or(&Vec::new()).clone())
                    }
                    DataType::Json => ValueType::Json(input[name].clone()),
                };

                res.push(evaluator.populate_output(&name, output)?.try_into()?);
            }

            Ok(res)
        }
        Template::ConstructArray(_) => {
            let mut arr: Vec<Value> = vec![];

            for (input, _) in evaluator.graph[node_id].inputs.iter() {
                arr.push(evaluator.evaluate_input(&input)?.try_into()?);
            }

            Ok(vec![evaluator
                .populate_output("out", ValueType::Array(arr))?
                .try_into()?])
        }
        Template::DeconstructArray(data) => {
            let arr: Vec<Value> = evaluator.evaluate_input("array")?.try_into()?;

            let mut res: Vec<String> = vec![];

            for i in 0..evaluator.graph[node_id].outputs.len() {
                let value = arr.get(i).unwrap_or(&Value::Null);

                let data = match data {
                    DataType::Bool => ValueType::Bool(value.as_bool().unwrap_or_default()),
                    DataType::Number => {
                        let number = match value.clone() {
                            Value::Number(n) => n,
                            _ => Number::from(0),
                        };
                        ValueType::Number(number)
                    }
                    DataType::String => {
                        ValueType::String(value.as_str().unwrap_or_default().to_string())
                    }
                    DataType::Array => {
                        ValueType::Array(value.as_array().unwrap_or(&vec![]).clone())
                    }
                    DataType::Json => ValueType::Json(value.clone()),
                };
                res.push(
                    evaluator
                        .populate_output(&(i).to_string(), data)?
                        .try_into()?,
                );
            }

            Ok(res)
        }
        _ => Ok(vec![evaluator
            .populate_output("out", ValueType::Bool(false))?
            .try_into()?]),
    }
}

fn populate_output(
    graph: &NodeGraph,
    outputs_cache: &mut OutputsCache,
    node_id: NodeId,
    param_name: &str,
    value: ValueType,
) -> anyhow::Result<ValueType> {
    let output_id = graph[node_id].get_output(param_name)?;
    let out = value.clone();
    outputs_cache.insert(output_id, value);
    Ok(out)
}

// Evaluates the input value of
fn evaluate_input(
    graph: &NodeGraph,
    node_id: NodeId,
    param_name: &str,
    outputs_cache: &mut OutputsCache,
) -> anyhow::Result<ValueType> {
    let input_id = graph[node_id].get_input(param_name)?;

    // The output of another node is connected.
    if let Some(other_output_id) = graph.connection(input_id) {
        // The value was already computed due to the evaluation of some other
        // node. We simply return value from the cache.
        if let Some(other_value) = outputs_cache.get(&other_output_id) {
            Ok(other_value.clone())
        }
        // This is the first time encountering this node, so we need to
        // recursively evaluate it.
        else {
            // Calling this will populate the cache
            evaluate_node(graph, graph[other_output_id].node, outputs_cache)?;

            // Now that we know the value is cached, return it
            Ok(outputs_cache
                .get(&other_output_id)
                .expect("Cache should be populated")
                .clone())
        }
    }
    // No existing connection, take the inline value instead.
    else {
        Ok(graph[input_id].value.clone())
    }
}
