extern crate core;

mod node;

use crate::node::OrchestratorNodeGraph;
use eframe::egui::Context;
use eframe::{egui, Frame};

// TODO: main window design

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(320.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native(
        "Secret Orchestrator",
        options,
        Box::new(|_cc| Box::new(Orchestrator::default())),
    )
}

struct Orchestrator {
    graph_state: OrchestratorNodeGraph,
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self {
            graph_state: Default::default(),
        }
    }
}

impl eframe::App for Orchestrator {
    fn update(&mut self, ctx: &Context, frame: &mut Frame) {
        self.graph_state.update(ctx, frame)
    }
}
