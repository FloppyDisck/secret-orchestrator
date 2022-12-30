use crate::node::{GraphState, NodeState, Response};
use anyhow::anyhow;
use eframe::egui;
use eframe::egui::{Checkbox, ComboBox, TextEdit, Ui};
use egui_node_graph::{DataTypeTrait, NodeId, WidgetValueTrait};
use serde_json::{json, Number, Value};
use std::borrow::Cow;

/// Determines the communication ranges for the types
#[derive(PartialEq, Eq, Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum DataType {
    #[default]
    Bool,
    Number,
    String,
    Array,
    Json,
}

impl DataType {
    pub fn combo_box(&mut self, ui: &mut Ui) {
        ComboBox::from_label("")
            .selected_text(format!("{:?}", self))
            .show_ui(ui, |ui| {
                ui.selectable_value(self, Self::Bool, "bool");
                ui.selectable_value(self, Self::Number, "number");
                ui.selectable_value(self, Self::String, "string");
                ui.selectable_value(self, Self::Array, "array");
                ui.selectable_value(self, Self::Json, "json");
            });
    }
}

/// Implements the Node intractable points color
impl DataTypeTrait<GraphState> for DataType {
    fn data_type_color(&self, _user_state: &mut GraphState) -> egui::Color32 {
        match self {
            DataType::Bool => egui::Color32::from_rgb(255, 51, 255),
            DataType::Number => egui::Color32::from_rgb(51, 51, 255),
            DataType::String => egui::Color32::from_rgb(51, 153, 255),
            DataType::Array => egui::Color32::from_rgb(51, 255, 255),
            DataType::Json => egui::Color32::from_rgb(255, 255, 51),
        }
    }

    fn name(&self) -> Cow<str> {
        Cow::Borrowed(match self {
            DataType::Bool => "boolean",
            DataType::Number => "number",
            DataType::String => "string",
            DataType::Array => "array",
            DataType::Json => "json",
        })
    }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Serialize, serde::Deserialize))]
pub enum ValueType {
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Json(Value),
}

impl WidgetValueTrait for ValueType {
    type Response = Response;
    type UserState = GraphState;
    type NodeData = NodeState;

    /// Runs per update
    fn value_widget(
        &mut self,
        param_name: &str,
        node_id: NodeId,
        ui: &mut Ui,
        user_state: &mut Self::UserState,
        node_data: &Self::NodeData,
    ) -> Vec<Self::Response> {
        let mut res = vec![];

        ui.horizontal(|ui| {
            ui.label(param_name);
            match self {
                ValueType::Bool(value) => {
                    ui.add(Checkbox::new(value, ""));
                }
                ValueType::Number(_) => {}
                ValueType::String(value) => {
                    ui.add(TextEdit::singleline(value));
                }
                ValueType::Array(_value) => {}
                ValueType::Json(_value) => {}
            }

            if let Some(node) = user_state.editing_node {
                if node == node_id && node_data.template.is_json() {
                    if ui.button("Remove").clicked() {
                        user_state.json_name = param_name.to_string();
                        res.push(Response::RemoveParam(node_id));
                    }
                }
            }
        });

        res
    }
}

impl Default for ValueType {
    fn default() -> Self {
        Self::Bool(false)
    }
}

impl TryInto<bool> for ValueType {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<bool, Self::Error> {
        match self {
            ValueType::Bool(b) => Ok(b),
            ValueType::Number(n) => Ok(n.as_f64().unwrap() > 0.0),
            _ => anyhow::bail!("Invalid cast from {:?} to boolean", self),
        }
    }
}
impl TryInto<Number> for ValueType {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Number, Self::Error> {
        match self {
            ValueType::Bool(b) => Ok(Number::from(b as i8)),
            ValueType::String(s) => {
                let n = s
                    .parse::<f64>()
                    .map_err(|_| anyhow!("Could not parse {} to float", s));
                Ok(Number::from_f64(n?).unwrap())
            }
            _ => anyhow::bail!("Invalid cast from {:?} to float", self),
        }
    }
}
impl TryInto<String> for ValueType {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            ValueType::Bool(b) => Ok(b.to_string()),
            ValueType::Number(n) => Ok(n.to_string()),
            ValueType::String(s) => Ok(s),
            ValueType::Array(a) => Ok(format!("{:?}", a)),
            ValueType::Json(j) => Ok(format!("{}", j.to_string())),
        }
    }
}
impl TryInto<Value> for ValueType {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Value, Self::Error> {
        match self {
            ValueType::Bool(b) => Ok(json!(b)),
            ValueType::Number(n) => Ok(json!(n)),
            ValueType::String(s) => Ok(json!(s)),
            ValueType::Array(a) => Ok(json!(a)),
            ValueType::Json(j) => Ok(j),
        }
    }
}

impl TryInto<Vec<Value>> for ValueType {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<Value>, Self::Error> {
        match self {
            ValueType::Array(a) => Ok(a),
            ValueType::Json(j) => Ok(vec![j]),
            _ => anyhow::bail!("Not gonna bother casting {:?} to vec", self),
        }
    }
}
