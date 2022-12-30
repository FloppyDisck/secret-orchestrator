use crate::node::data::{DataType, ValueType};
use crate::node::template::Template;
use crate::node::{GraphState, Response};
use eframe::egui;
use eframe::egui::{TextEdit, Ui};
use egui_node_graph::{Graph, NodeDataTrait, NodeId, NodeResponse, UserResponseTrait};

/// The node's state
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub struct NodeState {
    pub(crate) template: Template,
}

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
                        user_state.json_name = user_state.json_name.replace(" ", "_");
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
