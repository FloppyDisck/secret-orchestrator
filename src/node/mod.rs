// TODO: model the localsecret-rs modules to actually work
// TODO: make playing area dragable

// TODO: store old node state
// TODO: hover over node and see state
// TODO: outline red if node errors

// TODO: save or clone nodes

// TODO: infinite loop countermeasure maybe using petgraph

pub mod data;
pub mod evaluator;
pub mod state;
pub mod template;

use crate::node::data::{DataType, ValueType};
use crate::node::evaluator::evaluate_node;
use crate::node::state::NodeState;
use crate::node::template::{add_param, remove_param, Template, TemplateIterator, IO};
use eframe::egui::{Context, TextStyle};
use eframe::{egui, App, Frame};
use egui_node_graph::{Graph, GraphEditorState, NodeId, NodeResponse, UserResponseTrait};
use std::collections::HashMap;
use std::default::Default;

type EditorState = GraphEditorState<NodeState, DataType, ValueType, Template, GraphState>;
type NodeGraph = Graph<NodeState, DataType, ValueType>;

#[derive(Default)]
pub struct OrchestratorNodeGraph {
    state: EditorState,
    user_state: GraphState,
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

#[cfg(feature = "persistence")]
const PERSISTENCE_KEY: &str = "egui_node_graph";

#[cfg(feature = "persistence")]
impl OrchestratorNodeGraph {
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

impl App for OrchestratorNodeGraph {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
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

    #[cfg(feature = "persistence")]
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, PERSISTENCE_KEY, &self.state);
    }
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

impl UserResponseTrait for Response {}
