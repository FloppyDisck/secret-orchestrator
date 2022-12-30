use crate::node::data::{DataType, ValueType};
use crate::node::template::Template;
use crate::node::NodeGraph;
use egui_node_graph::{NodeId, OutputId};
use serde_json::{Map, Number, Value};
use std::collections::HashMap;
use std::str::FromStr;

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
